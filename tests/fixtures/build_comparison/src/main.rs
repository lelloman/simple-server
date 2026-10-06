#[cfg(all(feature = "source", feature = "engine"))]
compile_error!("choose one backend");
#[cfg(not(any(feature = "source", feature = "engine")))]
compile_error!("choose one backend");

#[cfg(feature = "engine")]
use simple_server::engine_web as web;
#[cfg(feature = "source")]
use simple_server::web;
use web::{Json, Path};

// The benchmark changes this value to force a real application-only rebuild.
const REVISION: u32 = 1;

async fn health() -> &'static str {
    "ok"
}
async fn echo(Json(value): Json<serde_json::Value>) -> Json<serde_json::Value> {
    Json(value)
}
async fn item(Path(id): Path<u64>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"id": id, "revision": REVISION}))
}

#[cfg_attr(feature = "engine", simple_server::main(worker_threads = 2))]
#[cfg_attr(feature = "source", tokio::main(worker_threads = 2))]
async fn main() -> std::io::Result<()> {
    #[cfg(feature = "engine")]
    let (listener, router, shutdown) = {
        use web::{Method, MethodRouter, Router, Shutdown};
        let stop = Shutdown::new();
        let signal = stop.clone();
        let router = Router::new()?
            .route(
                "/health",
                MethodRouter::new()?.on_handler(Method::GET, health)?,
            )?
            .route(
                "/echo",
                MethodRouter::new()?.on_handler(Method::POST, echo)?,
            )?
            .route(
                "/items/{id}",
                MethodRouter::new()?.on_handler(Method::GET, item)?,
            )?
            .route(
                "/shutdown",
                MethodRouter::new()?.on_handler(Method::POST, move || {
                    let signal = signal.clone();
                    async move {
                        signal.request();
                        "stopping"
                    }
                })?,
            )?;
        (web::bind("127.0.0.1:0").await?, router, stop)
    };
    #[cfg(feature = "source")]
    let (listener, router, shutdown) = {
        use simple_server::{lifecycle::Shutdown, net::TcpListener};
        use web::{
            Router,
            routing::{get, post},
        };
        let stop = Shutdown::new();
        let signal = stop.clone();
        let router = Router::new()
            .route("/health", get(health))
            .route("/echo", post(echo))
            .route("/items/{id}", get(item))
            .route(
                "/shutdown",
                post(move || {
                    let signal = signal.clone();
                    async move {
                        signal.request();
                        "stopping"
                    }
                }),
            );
        (TcpListener::bind("127.0.0.1:0").await?, router, stop)
    };
    #[cfg(feature = "engine")]
    println!("{}", listener.local_addr());
    #[cfg(feature = "source")]
    println!("{}", listener.local_addr()?);
    web::serve(listener, router, shutdown).await
}
