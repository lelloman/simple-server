//! Immutable router resources. Application state and typed extractors remain
//! in the host callbacks; matching and method semantics remain in Axum.
use crate::{
    callback,
    operations::{encode, id},
    server::HostHandler,
};
use axum::{
    Router,
    handler::Handler,
    routing::{MethodFilter, MethodRouter},
};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

pub const ROUTER: u32 = 8;
pub const METHODS: u32 = 9;
static ROUTERS: OnceLock<Mutex<HashMap<u64, Router>>> = OnceLock::new();
static METHOD_ROUTERS: OnceLock<Mutex<HashMap<u64, MethodRouter>>> = OnceLock::new();
fn routers() -> &'static Mutex<HashMap<u64, Router>> {
    ROUTERS.get_or_init(Default::default)
}
fn methods() -> &'static Mutex<HashMap<u64, MethodRouter>> {
    METHOD_ROUTERS.get_or_init(Default::default)
}
fn number(value: &Value, key: &str) -> Result<u64, String> {
    value[key].as_u64().ok_or_else(|| format!("missing {key}"))
}
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value[key].as_str().ok_or_else(|| format!("missing {key}"))
}
pub fn router(id: u64) -> Result<Router, String> {
    routers()
        .lock()
        .unwrap()
        .get(&id)
        .cloned()
        .ok_or_else(|| "router released".into())
}
fn method_router(id: u64) -> Result<MethodRouter, String> {
    methods()
        .lock()
        .unwrap()
        .get(&id)
        .cloned()
        .ok_or_else(|| "method router released".into())
}
pub fn release(kind: u32, id: u64) {
    match kind {
        ROUTER => {
            let value = routers().lock().unwrap().remove(&id);
            drop(value);
        }
        METHODS => {
            let value = methods().lock().unwrap().remove(&id);
            drop(value);
        }
        _ => (),
    }
}
pub fn resource_new(command: &Value) -> Result<Vec<u8>, String> {
    let operation = text(command, "op")?;
    if operation.starts_with("router_") {
        let result = match operation {
            "router_new" => Router::new(),
            "router_layer" | "router_route_layer" => {
                let callback = callback::get(number(command, "handler")?)?;
                let layer = axum::middleware::from_fn(
                    move |request: axum::extract::Request, next: axum::middleware::Next| {
                        crate::middleware::run(request, next, callback.clone())
                    },
                );
                let router = router(number(command, "router")?)?;
                if operation == "router_layer" {
                    router.layer(layer)
                } else {
                    router.route_layer(layer)
                }
            }
            "router_route" => router(number(command, "router")?)?.route(
                text(command, "path")?,
                method_router(number(command, "methods")?)?,
            ),
            "router_merge" => {
                router(number(command, "router")?)?.merge(router(number(command, "other")?)?)
            }
            "router_nest" => router(number(command, "router")?)?
                .nest(text(command, "path")?, router(number(command, "other")?)?),
            "router_nest_service" => router(number(command, "router")?)?.nest_service(
                text(command, "path")?,
                HostHandler(callback::get(number(command, "handler")?)?).with_state(()),
            ),
            "router_fallback" => router(number(command, "router")?)?
                .fallback(HostHandler(callback::get(number(command, "handler")?)?)),
            "router_fallback_methods" => router(number(command, "router")?)?
                .fallback(method_router(number(command, "methods")?)?),
            _ => return Err("unknown router command".into()),
        };
        let id = id();
        routers().lock().unwrap().insert(id, result);
        return Ok(encode(json!({"ok":true,"id":id}), &[]));
    }
    let result = match operation {
        "method_new" => MethodRouter::new(),
        "method_layer" | "method_route_layer" => {
            let callback = callback::get(number(command, "handler")?)?;
            let layer = axum::middleware::from_fn(
                move |request: axum::extract::Request, next: axum::middleware::Next| {
                    crate::middleware::run(request, next, callback.clone())
                },
            );
            let methods = method_router(number(command, "methods")?)?;
            if operation == "method_layer" {
                methods.layer(layer)
            } else {
                methods.route_layer(layer)
            }
        }

        "method_handler" => {
            let handler = HostHandler(callback::get(number(command, "handler")?)?);
            let base = method_router(number(command, "methods")?)?;
            let filter = match text(command, "method")? {
                "GET" => MethodFilter::GET,
                "POST" => MethodFilter::POST,
                "PUT" => MethodFilter::PUT,
                "DELETE" => MethodFilter::DELETE,
                "HEAD" => MethodFilter::HEAD,
                "OPTIONS" => MethodFilter::OPTIONS,
                "PATCH" => MethodFilter::PATCH,
                "TRACE" => MethodFilter::TRACE,
                "CONNECT" => MethodFilter::CONNECT,
                _ => return Err("unsupported route method".into()),
            };
            base.on(filter, handler)
        }
        "method_any" => {
            axum::routing::any(HostHandler(callback::get(number(command, "handler")?)?))
        }
        _ => return Err("unknown method router command".into()),
    };
    let id = id();
    methods().lock().unwrap().insert(id, result);
    Ok(encode(json!({"ok":true,"id":id}), &[]))
}
