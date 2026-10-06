use simple_server::{
    client::Client,
    engine_web::{
        self, Extension, Form, Handler, Json, Method, MethodRouter, Path, Query, RequestMetadata,
        Response, Router, Shutdown, State,
    },
    runtime::spawn,
};

#[derive(Clone)]
struct EchoService;
impl engine_web::Service<engine_web::Request> for EchoService {
    type Response = Response;
    type Error = std::convert::Infallible;
    type Future = std::future::Ready<Result<Response, Self::Error>>;
    fn poll_ready(
        &mut self,
        _: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }
    fn call(&mut self, request: engine_web::Request) -> Self::Future {
        let metadata = request.extensions().get::<RequestMetadata>().unwrap();
        assert_eq!(metadata.path_params[0].1, "engine");
        // The service returns before its response body finishes streaming.
        std::future::ready(Ok(Response::new(request.into_body())))
    }
}

#[simple_server::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = Router::new()?.route(
        "/echo/{name}",
        MethodRouter::new()?.on_service(Method::POST, EchoService)?,
    )?;
    async fn typed(
        State(prefix): State<String>,
        Query(query): Query<std::collections::BTreeMap<String, String>>,
        Json(body): Json<serde_json::Value>,
    ) -> Json<serde_json::Value> {
        Json(serde_json::json!({"prefix":prefix,"word":query["word"],"input":body}))
    }
    let router = router.route(
        "/typed",
        MethodRouter::new()?.on_handler(Method::POST, typed)?,
    )?;
    async fn form(
        Extension(prefix): Extension<String>,
        Form(mut fields): Form<std::collections::BTreeMap<String, String>>,
    ) -> Form<std::collections::BTreeMap<String, String>> {
        fields.insert("prefix".into(), prefix);
        Form(fields)
    }
    let router = router.route(
        "/form",
        MethodRouter::new()?.on(Method::POST, |mut request| async move {
            // Host-local request context is installed before typed extraction.
            request.extensions_mut().insert("engine".to_owned());
            form.call(request, ()).await
        })?,
    )?;
    async fn path(Path((id, name)): Path<(u32, String)>) -> Json<serde_json::Value> {
        Json(serde_json::json!({"id":id,"name":name}))
    }
    let router = router.route(
        "/items/{id}/{name}",
        MethodRouter::new()?.on_handler(Method::GET, path)?,
    )?;
    let router = router
        .route(
            "/redirect",
            MethodRouter::new()?.on_handler(Method::GET, || async {
                (
                    [("x-engine-helper", "redirect")],
                    engine_web::response::Redirect::temporary("/typed"),
                )
            })?,
        )?
        .route(
            "/invalid-headers",
            MethodRouter::new()?.on_handler(Method::GET, || async {
                (
                    engine_web::StatusCode::CREATED,
                    [("x-value", "bad\nvalue")],
                    "body",
                )
            })?,
        )?;
    let router = router.with_state("engine".to_owned())?;
    let listener = engine_web::bind("127.0.0.1:0").await?;
    let address = listener.local_addr();
    let url = format!("http://{}/echo/engine", listener.local_addr());
    let shutdown = Shutdown::new();
    let server = spawn(engine_web::serve(listener, router, shutdown.clone()));
    let response = Client::builder()
        .no_proxy()
        .build()?
        .post(url)
        .body(b"public API, prebuilt engine".to_vec())
        .send()
        .await?;
    assert_eq!(response.text().await?, "public API, prebuilt engine");
    let response = Client::builder()
        .no_proxy()
        .build()?
        .post(format!("http://{address}/typed?word=hello"))
        .json(&serde_json::json!({"count":7}))
        .send()
        .await?;
    assert_eq!(
        response.json::<serde_json::Value>().await?,
        serde_json::json!({"prefix":"engine","word":"hello","input":{"count":7}})
    );
    let response = Client::builder()
        .no_proxy()
        .build()?
        .post(format!("http://{address}/form"))
        .header("content-type", "application/x-www-form-urlencoded")
        .body(b"word=a+b%2Bc".to_vec())
        .send()
        .await?;
    assert_eq!(
        response.headers()["content-type"],
        "application/x-www-form-urlencoded"
    );
    assert_eq!(response.text().await?, "prefix=engine&word=a+b%2Bc");
    let client = Client::builder().no_proxy().build()?;
    let response = client
        .get(format!("http://{address}/items/7/a%252Fb+c"))
        .send()
        .await?;
    assert_eq!(
        response.json::<serde_json::Value>().await?,
        serde_json::json!({"id":7,"name":"a%2Fb+c"})
    );
    let response = client
        .get(format!("http://{address}/items/bad/name"))
        .send()
        .await?;
    assert_eq!(response.status().as_u16(), 400);
    assert_eq!(
        response.text().await?,
        "Invalid URL: Cannot parse value at index 0 with value `bad` to a `u32`"
    );
    let response = client
        .get(format!("http://{address}/items/7/%FF"))
        .send()
        .await?;
    assert_eq!(response.status().as_u16(), 400);
    assert_eq!(
        response.text().await?,
        "Invalid URL: Invalid UTF-8 in `name`"
    );
    let client = Client::builder().no_proxy().no_redirect().build()?;
    let response = client
        .get(format!("http://{address}/redirect"))
        .send()
        .await?;
    assert_eq!(response.status().as_u16(), 307);
    assert_eq!(response.headers()["location"], "/typed");
    assert_eq!(response.headers()["x-engine-helper"], "redirect");
    assert!(response.bytes().await?.is_empty());
    let response = client
        .get(format!("http://{address}/invalid-headers"))
        .send()
        .await?;
    assert_eq!(response.status().as_u16(), 500);
    assert_eq!(response.text().await?, "failed to parse header value");
    shutdown.request();
    server.await??;
    println!("public engine HTTP consumer passed");
    Ok(())
}
