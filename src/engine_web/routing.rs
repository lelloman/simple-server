use super::{Request, Response, body, wire};
use serde_json::json;
use simple_server_sys::{Callback, Reply, Resource};
use std::{future::Future, io, sync::Arc};

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
// The preview is a native router used only for eager validation. Pending
// handlers receive a fresh callback when their Rust state is supplied. Ready
// plans reuse their resource, including method-local state already bound earlier.
type Bind<S> = dyn Fn(S) -> io::Result<Resource> + Send + Sync;
#[derive(Clone)]
struct Plan<S> {
    preview: Resource,
    bind: Option<Arc<Bind<S>>>,
}
impl<S: Clone + Send + Sync + 'static> Plan<S> {
    fn ready(resource: Resource) -> Self {
        Self {
            preview: resource,
            bind: None,
        }
    }
    fn resolve(&self, state: S) -> io::Result<Resource> {
        match &self.bind {
            Some(bind) => bind(state),
            None => Ok(self.preview.clone()),
        }
    }
    fn map(
        self,
        apply: impl Fn(&Resource) -> io::Result<Resource> + Send + Sync + 'static,
    ) -> io::Result<Self> {
        let preview = apply(&self.preview)?;
        if self.bind.is_none() {
            return Ok(Self::ready(preview));
        }
        Ok(Self {
            preview,
            bind: Some(Arc::new(move |state| apply(&self.resolve(state)?))),
        })
    }
    fn combine(
        self,
        other: Self,
        apply: impl Fn(&Resource, &Resource) -> io::Result<Resource> + Send + Sync + 'static,
    ) -> io::Result<Self> {
        let preview = apply(&self.preview, &other.preview)?;
        if self.bind.is_none() && other.bind.is_none() {
            return Ok(Self::ready(preview));
        }
        Ok(Self {
            preview,
            bind: Some(Arc::new(move |state: S| {
                apply(&self.resolve(state.clone())?, &other.resolve(state)?)
            })),
        })
    }
}
fn placeholder() -> io::Result<Callback> {
    callback(|_| async {
        super::IntoResponse::into_response(super::StatusCode::INTERNAL_SERVER_ERROR)
    })
}
fn route_handler(router: &Resource, handler: &Callback) -> io::Result<Resource> {
    wire::resource(
        8,
        json!({"op":"router_fallback","router":router.id(),"handler":handler.id()}),
    )
}
fn method_handler(
    methods: &Resource,
    method: &super::Method,
    handler: &Callback,
) -> io::Result<Resource> {
    wire::resource(
        9,
        json!({"op":"method_handler","methods":methods.id(),"method":method.as_str(),"handler":handler.id()}),
    )
}
fn any_handler(handler: &Callback) -> io::Result<Resource> {
    wire::resource(9, json!({"op":"method_any","handler":handler.id()}))
}
/// Immutable route collection whose missing application state is `S`.
/// Invalid routes are rejected during construction. Only `Router<()>` can serve.
///
/// Bind missing state before serving:
/// ```no_run
/// use simple_server::engine_web::{Router, MethodRouter, Method, State};
/// async fn app() -> std::io::Result<Router> {
///     Router::<String>::new()?
///         .route("/", MethodRouter::new()?.on_handler(Method::GET,
///             |State(name): State<String>| async move { name })?)?
///         .with_state("shared".to_owned())
/// }
/// ```
/// An unbound stateful router cannot be served:
/// ```compile_fail
/// use simple_server::engine_web::{serve, Router, TcpListener, Shutdown};
/// async fn invalid(listener: TcpListener, router: Router<String>) {
///     serve(listener, router, Shutdown::new()).await.unwrap();
/// }
/// ```
/// A method group must require the same missing state as its enclosing router:
/// ```compile_fail
/// use simple_server::engine_web::{Router, MethodRouter};
/// let _ = Router::<String>::new().unwrap()
///     .route("/", MethodRouter::<u32>::new().unwrap());
/// ```
/// Supplying a different state type is a compile-time error:
/// ```compile_fail
/// use simple_server::engine_web::Router;
/// let _: Router = Router::<String>::new().unwrap().with_state(7u32).unwrap();
/// ```
#[derive(Clone)]
pub struct Router<S = ()> {
    plan: Plan<S>,
}
impl<S: Clone + Send + Sync + 'static> Router<S> {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            plan: Plan::ready(wire::resource(8, json!({"op":"router_new"}))?),
        })
    }
    pub fn route(self, path: &str, methods: MethodRouter<S>) -> io::Result<Self> {
        let path = path.to_owned();
        Ok(Self { plan: self.plan.combine(methods.plan, move |router, methods| {
            wire::resource(8, json!({"op":"router_route","router":router.id(),"path":path,"methods":methods.id()}))
        })? })
    }
    pub fn merge(self, other: Self) -> io::Result<Self> {
        Ok(Self {
            plan: self.plan.combine(other.plan, |router, other| {
                wire::resource(
                    8,
                    json!({"op":"router_merge","router":router.id(),"other":other.id()}),
                )
            })?,
        })
    }
    pub fn nest(self, path: &str, other: Self) -> io::Result<Self> {
        let path = path.to_owned();
        Ok(Self {
            plan: self.plan.combine(other.plan, move |router, other| {
                wire::resource(
                    8,
                    json!({"op":"router_nest","router":router.id(),"path":path,"other":other.id()}),
                )
            })?,
        })
    }
    pub fn fallback<F, Fut>(self, handler: F) -> io::Result<Self>
    where
        F: Fn(Request) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        let handler = callback(handler)?;
        Ok(Self {
            plan: self
                .plan
                .map(move |router| route_handler(router, &handler))?,
        })
    }
    /// Use a Tower service for unmatched routes. Each request gets a clone;
    /// readiness is awaited on that same clone before call. Service errors must
    /// already be mapped into responses. Bodies remain lazy.
    pub fn fallback_service<T, B>(self, service: T) -> io::Result<Self>
    where
        T: super::Service<Request, Response = http::Response<B>, Error = std::convert::Infallible>
            + Clone
            + Send
            + Sync
            + 'static,
        T::Future: Send + 'static,
        B: http_body::Body<Data = super::Bytes> + Send + 'static,
        B::Error: Into<super::body::BoxError>,
    {
        self.fallback(move |request| super::service::call(service.clone(), request))
    }
    pub fn fallback_methods(self, methods: MethodRouter<S>) -> io::Result<Self> {
        Ok(Self { plan: self.plan.combine(methods.plan, |router, methods| {
            wire::resource(8, json!({"op":"router_fallback_methods","router":router.id(),"methods":methods.id()}))
        })? })
    }
    /// Mount a Tower service below a prefix, stripping it from the request URI.
    /// RequestMetadata retains the original URI. Requires an engine implementing
    /// router_nest_service; older engines return a construction error.
    pub fn nest_service<T, B>(self, path: &str, service: T) -> io::Result<Self>
    where
        T: super::Service<Request, Response = http::Response<B>, Error = std::convert::Infallible>
            + Clone
            + Send
            + Sync
            + 'static,
        T::Future: Send + 'static,
        B: http_body::Body<Data = super::Bytes> + Send + 'static,
        B::Error: Into<super::body::BoxError>,
    {
        let path = path.to_owned();
        let handler = callback(move |request| super::service::call(service.clone(), request))?;
        Ok(Self {
            plan: self.plan.map(move |router| {
                wire::resource(8, json!({"op":"router_nest_service","router":router.id(),"path":path,"handler":handler.id()}))
            })?,
        })
    }
    pub fn fallback_handler<H, T>(self, handler: H) -> io::Result<Self>
    where
        H: super::Handler<T, S>,
        T: 'static,
    {
        let preview = route_handler(&self.plan.preview, &placeholder()?)?;
        Ok(Self {
            plan: Plan {
                preview,
                bind: Some(Arc::new(move |state: S| {
                    let router = self.plan.resolve(state.clone())?;
                    let handler = handler.clone();
                    let handler =
                        callback(move |request| handler.clone().call(request, state.clone()))?;
                    route_handler(&router, &handler)
                })),
            },
        })
    }
    /// Bind state to existing handlers. Newly added handlers may require `S2`.
    /// State remains in host callbacks; it never crosses the engine ABI.
    pub fn with_state<S2: Clone + Send + Sync + 'static>(self, state: S) -> io::Result<Router<S2>> {
        Ok(Router {
            plan: Plan::ready(self.plan.resolve(state)?),
        })
    }
}
impl Router {
    pub(super) fn into_resource(self) -> io::Result<Resource> {
        self.plan.resolve(())
    }
}
/// Method handlers whose missing state is `S`. Bound method-local state is
/// independent of any state subsequently supplied to the enclosing router.
#[derive(Clone)]
pub struct MethodRouter<S = ()> {
    plan: Plan<S>,
}
impl<S: Clone + Send + Sync + 'static> MethodRouter<S> {
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            plan: Plan::ready(wire::resource(9, json!({"op":"method_new"}))?),
        })
    }
    pub fn on<F, Fut>(self, method: super::Method, handler: F) -> io::Result<Self>
    where
        F: Fn(Request) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        let handler = callback(handler)?;
        Ok(Self {
            plan: self
                .plan
                .map(move |methods| method_handler(methods, &method, &handler))?,
        })
    }
    pub fn any<F, Fut>(handler: F) -> io::Result<Self>
    where
        F: Fn(Request) -> Fut + Clone + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        Ok(Self {
            plan: Plan::ready(any_handler(&callback(handler)?)?),
        })
    }
    /// Register a Tower service for one method. GET retains native HEAD support
    /// and unmatched methods retain native 405/Allow behavior.
    pub fn on_service<T, B>(self, method: super::Method, service: T) -> io::Result<Self>
    where
        T: super::Service<Request, Response = http::Response<B>, Error = std::convert::Infallible>
            + Clone
            + Send
            + Sync
            + 'static,
        T::Future: Send + 'static,
        B: http_body::Body<Data = super::Bytes> + Send + 'static,
        B::Error: Into<super::body::BoxError>,
    {
        self.on(method, move |request| {
            super::service::call(service.clone(), request)
        })
    }
    /// Register a Tower service for any method, awaiting readiness per request.
    pub fn any_service<T, B>(service: T) -> io::Result<Self>
    where
        T: super::Service<Request, Response = http::Response<B>, Error = std::convert::Infallible>
            + Clone
            + Send
            + Sync
            + 'static,
        T::Future: Send + 'static,
        B: http_body::Body<Data = super::Bytes> + Send + 'static,
        B::Error: Into<super::body::BoxError>,
    {
        Self::any(move |request| super::service::call(service.clone(), request))
    }
    /// Register a typed handler requiring this method group's missing state.
    /// Only its last argument may consume the body.
    pub fn on_handler<H, T>(self, method: super::Method, handler: H) -> io::Result<Self>
    where
        H: super::Handler<T, S>,
        T: 'static,
    {
        let preview = method_handler(&self.plan.preview, &method, &placeholder()?)?;
        Ok(Self {
            plan: Plan {
                preview,
                bind: Some(Arc::new(move |state: S| {
                    let methods = self.plan.resolve(state.clone())?;
                    let handler = handler.clone();
                    let handler =
                        callback(move |request| handler.clone().call(request, state.clone()))?;
                    method_handler(&methods, &method, &handler)
                })),
            },
        })
    }
    /// Register a typed handler for any method, requiring this group's state.
    pub fn any_handler<H, T>(handler: H) -> io::Result<Self>
    where
        H: super::Handler<T, S>,
        T: 'static,
    {
        let preview = any_handler(&placeholder()?)?;
        Ok(Self {
            plan: Plan {
                preview,
                bind: Some(Arc::new(move |state: S| {
                    let handler = handler.clone();
                    any_handler(&callback(move |request| {
                        handler.clone().call(request, state.clone())
                    })?)
                })),
            },
        })
    }
    /// Bind state only to this handler; other handlers keep their state requirements.
    pub fn on_state<H, T, Local>(
        self,
        method: super::Method,
        state: Local,
        handler: H,
    ) -> io::Result<Self>
    where
        H: super::Handler<T, Local>,
        T: 'static,
        Local: Clone + Send + Sync + 'static,
    {
        self.on(method, move |request| {
            handler.clone().call(request, state.clone())
        })
    }
    /// Bind all existing handlers, independently of the enclosing router's state.
    pub fn with_state<S2: Clone + Send + Sync + 'static>(
        self,
        state: S,
    ) -> io::Result<MethodRouter<S2>> {
        Ok(MethodRouter {
            plan: Plan::ready(self.plan.resolve(state)?),
        })
    }
}
