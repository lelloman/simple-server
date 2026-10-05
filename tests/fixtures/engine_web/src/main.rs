use simple_server::{
    client::Client,
    engine_web::{
        self, Json, Method, MethodRouter, Query, RequestMetadata, Response, Router, Shutdown, State,
    },
    runtime::spawn,
};

#[simple_server::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = Router::new()?.route(
        "/echo/{name}",
        MethodRouter::new()?.on(Method::POST, |request| async move {
            let metadata = request.extensions().get::<RequestMetadata>().unwrap();
            assert_eq!(metadata.path_params[0].1, "engine");
            // Stream the incoming body after the handler itself has returned.
            Response::new(request.into_body())
        })?,
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
        MethodRouter::new()?.on_state(Method::POST, "engine".to_owned(), typed)?,
    )?;
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
    shutdown.request();
    server.await??;
    println!("public engine HTTP consumer passed");
    Ok(())
}
