use simple_server::{
    client::Client,
    engine_web::{self, Method, MethodRouter, RequestMetadata, Response, Router, Shutdown},
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
    let listener = engine_web::bind("127.0.0.1:0").await?;
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
    shutdown.request();
    server.await??;
    println!("public engine HTTP consumer passed");
    Ok(())
}
