//! HTTP transport behind byte-oriented callbacks. No HTTP/Tokio type crosses
//! the ABI. Request bodies are borrowed resources until a host clone is made;
//! response callbacks are retained while their producer's reply buffer is live.
use crate::{
    callback::{self, ForeignCallback, ForeignFuture},
    operations::{decode, encode, id},
};
use axum::{
    body::{Body, Bytes},
    extract::{ConnectInfo, FromRequestParts, MatchedPath, OriginalUri, RawPathParams, Request},
    http::{HeaderMap, HeaderName, HeaderValue, Response, StatusCode},
};
use http_body::{Body as HttpBody, Frame, SizeHint};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    future::Future,
    io,
    net::SocketAddr,
    pin::Pin,
    sync::{Arc, Mutex, OnceLock},
    task::{Context, Poll},
};
use tokio::net::TcpListener;

pub const BODY: u32 = 6;
pub const LISTENER: u32 = 7;
type SharedBody = Arc<tokio::sync::Mutex<Body>>;
static BODIES: OnceLock<Mutex<HashMap<u64, SharedBody>>> = OnceLock::new();
static LISTENERS: OnceLock<Mutex<HashMap<u64, TcpListener>>> = OnceLock::new();
fn bodies() -> &'static Mutex<HashMap<u64, SharedBody>> {
    BODIES.get_or_init(Default::default)
}
fn listeners() -> &'static Mutex<HashMap<u64, TcpListener>> {
    LISTENERS.get_or_init(Default::default)
}
fn number(value: &Value, key: &str) -> Result<u64, String> {
    value[key].as_u64().ok_or_else(|| format!("missing {key}"))
}
pub fn release(kind: u32, id: u64) {
    // Drop outside the map lock: a body can contain host callbacks whose
    // destructors reenter resource release.
    match kind {
        BODY => {
            let value = bodies().lock().unwrap().remove(&id);
            drop(value);
        }
        LISTENER => {
            let value = listeners().lock().unwrap().remove(&id);
            drop(value);
        }
        _ => (),
    }
}
struct BorrowedBody(u64);
impl Drop for BorrowedBody {
    fn drop(&mut self) {
        release(BODY, self.0);
    }
}

pub fn resource_new(command: &Value) -> Result<Vec<u8>, String> {
    let body = bodies()
        .lock()
        .unwrap()
        .get(&number(command, "body")?)
        .cloned()
        .ok_or("body released")?;
    let id = id();
    bodies().lock().unwrap().insert(id, body);
    Ok(encode(json!({"ok":true,"id":id}), &[]))
}
pub fn headers_to_wire(headers: &HeaderMap) -> Value {
    json!(
        headers
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_bytes()))
            .collect::<Vec<_>>()
    )
}
fn headers_from_wire(value: &Value) -> Result<HeaderMap, String> {
    let mut headers = HeaderMap::new();
    for pair in value.as_array().ok_or("headers must be an array")? {
        let name =
            HeaderName::from_bytes(pair[0].as_str().ok_or("missing header name")?.as_bytes())
                .map_err(|e| e.to_string())?;
        let bytes: Vec<u8> = serde_json::from_value(pair[1].clone()).map_err(|e| e.to_string())?;
        let value = HeaderValue::from_bytes(&bytes).map_err(|e| e.to_string())?;
        headers.append(name, value);
    }
    Ok(headers)
}

struct CallbackBody {
    callback: Option<Arc<ForeignCallback>>,
    pending: Option<ForeignFuture>,
    hint: SizeHint,
}
impl HttpBody for CallbackBody {
    type Data = Bytes;
    type Error = io::Error;
    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, io::Error>>> {
        let Some(callback) = &self.callback else {
            return Poll::Ready(None);
        };
        if self.pending.is_none() {
            self.pending = Some(callback.call(&[]));
        }
        let result = match Pin::new(self.pending.as_mut().unwrap()).poll(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(result) => result,
        };
        self.pending = None;
        let decoded = result.and_then(|bytes| decode(&bytes));
        let frame = match decoded {
            Ok((header, body)) => match header["kind"].as_str() {
                Some("data") => {
                    let length = body.len() as u64;
                    let mut hint = SizeHint::new();
                    hint.set_lower(self.hint.lower().saturating_sub(length));
                    if let Some(upper) = self.hint.upper() {
                        hint.set_upper(upper.saturating_sub(length));
                    }
                    self.hint = hint;
                    Ok(Some(Frame::data(Bytes::from(body))))
                }
                Some("trailers") => {
                    self.callback = None;
                    self.hint = SizeHint::with_exact(0);
                    headers_from_wire(&header["headers"])
                        .map(|headers| Some(Frame::trailers(headers)))
                }
                Some("end") => {
                    self.callback = None;
                    self.hint = SizeHint::with_exact(0);
                    Ok(None)
                }
                Some("error") => Err(header["message"]
                    .as_str()
                    .unwrap_or("host body failed")
                    .into()),
                _ => Err("invalid host body frame".into()),
            },
            Err(error) => Err(error),
        };
        Poll::Ready(match frame {
            Ok(frame) => frame.map(Ok),
            Err(error) => {
                self.callback = None;
                self.hint = SizeHint::with_exact(0);
                Some(Err(io::Error::other(error)))
            }
        })
    }
    fn is_end_stream(&self) -> bool {
        self.callback.is_none()
    }
    fn size_hint(&self) -> SizeHint {
        self.hint
    }
}

async fn forward(
    request: Request,
    callback: Arc<ForeignCallback>,
) -> Result<Response<Body>, String> {
    let (mut parts, body) = request.into_parts();
    let matched_path = parts
        .extensions
        .get::<MatchedPath>()
        .map(|path| path.as_str().to_owned());
    let original_uri = parts
        .extensions
        .get::<OriginalUri>()
        .map(|uri| uri.0.to_string());
    let path_params = RawPathParams::from_request_parts(&mut parts, &())
        .await
        .map(|params| {
            params
                .iter()
                .map(|(key, value)| (key.to_owned(), value.to_owned()))
                .collect::<Vec<_>>()
        });
    let hint = body.size_hint();
    let body_id = id();
    bodies()
        .lock()
        .unwrap()
        .insert(body_id, Arc::new(tokio::sync::Mutex::new(body)));
    let _borrowed = BorrowedBody(body_id);
    let peer = parts
        .extensions
        .get::<ConnectInfo<SocketAddr>>()
        .map(|peer| peer.0.to_string());
    let request = encode(
        json!({"method":parts.method.as_str(),"uri":parts.uri.to_string(),"version":format!("{:?}",parts.version),"headers":headers_to_wire(&parts.headers),"peer":peer,"body":body_id,"lower":hint.lower(),"upper":hint.upper(),"matched_path":matched_path,"original_uri":original_uri,"path_params":path_params.as_ref().ok(),"path_error":path_params.as_ref().err().map(ToString::to_string),"path_status":path_params.as_ref().err().map(|error| error.status().as_u16())}),
        &[],
    );
    let reply = callback.call(&request).await?;
    let (header, payload) = decode(&reply)?;
    let status = u16::try_from(number(&header, "status")?).map_err(|e| e.to_string())?;
    let status = StatusCode::from_u16(status).map_err(|e| e.to_string())?;
    let headers = headers_from_wire(&header["headers"])?;
    let body = if let Some(body) = header["body"].as_u64() {
        // Acquire before dropping reply. The producer's release callback owns
        // the temporary registration and removes it after this reference exists.
        let callback = callback::get(body)?;
        let mut hint = SizeHint::new();
        let lower = header["lower"].as_u64().unwrap_or(0);
        let upper = header["upper"].as_u64();
        if upper.is_some_and(|upper| upper < lower) {
            return Err("invalid body size hint".into());
        }
        hint.set_lower(lower);
        if let Some(upper) = upper {
            hint.set_upper(upper);
        }
        Body::new(CallbackBody {
            callback: Some(callback),
            pending: None,
            hint,
        })
    } else {
        Body::from(payload)
    };
    let mut response = Response::new(body);
    *response.status_mut() = status;
    *response.headers_mut() = headers;
    Ok(response)
}

#[derive(Clone)]
pub(crate) struct HostHandler(pub Arc<ForeignCallback>);
pub(crate) enum HandlerMarker {}
impl axum::handler::Handler<(HandlerMarker,), ()> for HostHandler {
    type Future = Pin<Box<dyn Future<Output = Response<Body>> + Send>>;
    fn call(self, request: Request, _: ()) -> Self::Future {
        Box::pin(async move {
            forward(request, self.0).await.unwrap_or_else(|_| {
                let mut response = Response::new(Body::from("internal server error"));
                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                response
            })
        })
    }
}

pub fn operation(command: Value) -> Result<crate::operations::FutureBytes, String> {
    match command["op"].as_str() {
        Some("server_bind") => {
            let address = command["address"]
                .as_str()
                .ok_or("missing bind address")?
                .to_owned();
            Ok(Box::pin(async move {
                let result = async {
                    let listener = TcpListener::bind(address).await?;
                    let address = listener.local_addr()?;
                    let id = id();
                    listeners().lock().unwrap().insert(id, listener);
                    Ok::<_, io::Error>(encode(
                        json!({"ok":true,"id":id,"address":address.to_string()}),
                        &[],
                    ))
                }
                .await;
                result.unwrap_or_else(|error| encode(json!({"ok":false,"message":error.to_string(),"raw_os_error":error.raw_os_error()}), &[]))
            }))
        }
        Some("server_body_frame") => {
            let body = bodies()
                .lock()
                .unwrap()
                .get(&number(&command, "body")?)
                .cloned()
                .ok_or("body released")?;
            Ok(Box::pin(async move {
                match body.lock().await.frame().await {
                    None => encode(json!({"kind":"end"}), &[]),
                    Some(Err(error)) => {
                        encode(json!({"kind":"error","message":error.to_string()}), &[])
                    }
                    Some(Ok(frame)) => match frame.into_data() {
                        Ok(data) => encode(json!({"kind":"data"}), &data),
                        Err(frame) => match frame.into_trailers() {
                            Ok(headers) => encode(
                                json!({"kind":"trailers","headers":headers_to_wire(&headers)}),
                                &[],
                            ),
                            Err(_) => encode(
                                json!({"kind":"error","message":"unsupported body frame"}),
                                &[],
                            ),
                        },
                    },
                }
            }))
        }
        Some("server_serve") => {
            let router = if let Some(id) = command["router"].as_u64() {
                crate::routing::router(id)?
            } else {
                axum::Router::new()
                    .fallback(HostHandler(callback::get(number(&command, "handler")?)?))
            };
            let shutdown = callback::get(number(&command, "shutdown")?)?;
            let listener = listeners()
                .lock()
                .unwrap()
                .remove(&number(&command, "listener")?)
                .ok_or("listener released or already serving")?;
            Ok(Box::pin(async move {
                let mut stop = Box::pin(shutdown.call(&[]));
                // Honor a shutdown already requested without accepting a connection.
                if let Some(result) = std::future::poll_fn(|cx| {
                    Poll::Ready(match stop.as_mut().poll(cx) {
                        Poll::Ready(result) => Some(result),
                        Poll::Pending => None,
                    })
                })
                .await
                {
                    return match result {
                        Ok(_) => encode(json!({"ok":true}), &[]),
                        Err(error) => encode(json!({"ok":false,"message":error}), &[]),
                    };
                }
                let shutdown_error = Arc::new(Mutex::new(None));
                let error_slot = shutdown_error.clone();
                let result = axum::serve(
                    listener,
                    router.into_make_service_with_connect_info::<SocketAddr>(),
                )
                .with_graceful_shutdown(async move {
                    if let Err(error) = stop.await {
                        *error_slot.lock().unwrap() = Some(error);
                    }
                })
                .await;
                let error = shutdown_error
                    .lock()
                    .unwrap()
                    .take()
                    .or_else(|| result.err().map(|error| error.to_string()));
                match error {
                    Some(error) => encode(json!({"ok":false,"message":error}), &[]),
                    None => encode(json!({"ok":true}), &[]),
                }
            }))
        }
        _ => Err("unknown server operation".into()),
    }
}
