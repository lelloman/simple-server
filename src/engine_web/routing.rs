use super::{Request, Response, body, wire};
use serde_json::json;
use simple_server_sys::{Callback, Reply, Resource};
use std::{future::Future, io};

fn callback<F, Fut>(handler: F) -> io::Result<Callback>
where
    F: Fn(Request) -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Response> + Send + 'static,
{
    Callback::with_reply(move |bytes| {
        let handler = handler.clone();
        async move {
            let result = async { body::response(handler(wire::request(&bytes)?).await) }.await;
            result.unwrap_or_else(|_| {
                Reply::new(
                    wire::encode(json!({"status":500,"headers":[]}), b"internal server error")
                        .expect("static response is serializable"),
                )
            })
        }
    })
}
/// Immutable engine route collection. Building a derived router does not change
/// earlier clones. Invalid routes return an error from the native router.
#[derive(Clone)]
pub struct Router {
    pub(super) resource: Resource,
}
impl Router {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            resource: wire::resource(8, json!({"op":"router_new"}))?,
        })
    }
    pub fn route(self, path: &str, methods: MethodRouter) -> io::Result<Self> {
        Ok(Self {
            resource: wire::resource(
                8,
                json!({"op":"router_route","router":self.resource.id(),"path":path,"methods":methods.resource.id()}),
            )?,
        })
    }
    pub fn merge(self, other: Self) -> io::Result<Self> {
        Ok(Self {
            resource: wire::resource(
                8,
                json!({"op":"router_merge","router":self.resource.id(),"other":other.resource.id()}),
            )?,
        })
    }
    pub fn nest(self, path: &str, other: Self) -> io::Result<Self> {
        Ok(Self {
            resource: wire::resource(
                8,
                json!({"op":"router_nest","router":self.resource.id(),"path":path,"other":other.resource.id()}),
            )?,
        })
    }
    pub fn fallback<F, Fut>(self, handler: F) -> io::Result<Self>
    where
        F: Fn(Request) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        let handler = callback(handler)?;
        Ok(Self {
            resource: wire::resource(
                8,
                json!({"op":"router_fallback","router":self.resource.id(),"handler":handler.id()}),
            )?,
        })
    }
    pub fn fallback_methods(self, methods: MethodRouter) -> io::Result<Self> {
        Ok(Self {
            resource: wire::resource(
                8,
                json!({"op":"router_fallback_methods","router":self.resource.id(),"methods":methods.resource.id()}),
            )?,
        })
    }
}
#[derive(Clone)]
pub struct MethodRouter {
    resource: Resource,
}
impl MethodRouter {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            resource: wire::resource(9, json!({"op":"method_new"}))?,
        })
    }
    pub fn on<F, Fut>(self, method: super::Method, handler: F) -> io::Result<Self>
    where
        F: Fn(Request) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        let handler = callback(handler)?;
        Ok(Self {
            resource: wire::resource(
                9,
                json!({"op":"method_handler","methods":self.resource.id(),"method":method.as_str(),"handler":handler.id()}),
            )?,
        })
    }
    pub fn any<F, Fut>(handler: F) -> io::Result<Self>
    where
        F: Fn(Request) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        let handler = callback(handler)?;
        Ok(Self {
            resource: wire::resource(9, json!({"op":"method_any","handler":handler.id()}))?,
        })
    }
}

impl MethodRouter {
    /// Register a typed handler with unit state; only its last argument may read
    /// the body. The raw `on` API remains available without annotation changes.
    pub fn on_handler<H, T>(self, method: super::Method, handler: H) -> io::Result<Self>
    where
        H: super::Handler<T, ()>,
        T: 'static,
    {
        self.on_state(method, (), handler)
    }
    /// Bind application state to this handler. Router-wide generic state remains
    /// separate parity work; state values are cloned for each request.
    pub fn on_state<H, T, S>(self, method: super::Method, state: S, handler: H) -> io::Result<Self>
    where
        H: super::Handler<T, S>,
        T: 'static,
        S: Clone + Send + Sync + 'static,
    {
        self.on(method, move |request| {
            handler.clone().call(request, state.clone())
        })
    }
}
impl Router {
    pub fn fallback_handler<H, T>(self, handler: H) -> io::Result<Self>
    where
        H: super::Handler<T, ()>,
        T: 'static,
    {
        self.fallback(move |request| handler.clone().call(request, ()))
    }
}
