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
        if metadata
            .original_uri
            .as_ref()
            .unwrap()
            .path()
            .starts_with("/mounted/")
        {
            assert_eq!(request.uri(), "/tail?x=1");
            assert_eq!(
                metadata.original_uri.as_ref().unwrap(),
                "/mounted/engine/echo/tail?x=1"
            );
        }
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
    let router = router.nest(
        "/mounted/{name}",
        Router::new()?.nest_service("/echo", EchoService)?,
    )?;
    let router = router.route(
        "/events",
        MethodRouter::new()?.on_handler(Method::GET, || async {
            use engine_web::sse::{Event, KeepAlive, Sse};
            Sse::new(futures_util::stream::iter([Ok::<
                _,
                std::convert::Infallible,
            >(
                Event::default().event("update").id("1").data("ready"),
            )]))
            .keep_alive(KeepAlive::default())
        })?,
    )?;
    async fn upload(
        mut multipart: engine_web::multipart::Multipart,
    ) -> Result<Json<serde_json::Value>, engine_web::multipart::MultipartError> {
        let field = multipart.next_field().await?.unwrap();
        let name = field.name().map(str::to_owned);
        let filename = field.file_name().map(str::to_owned);
        let data = field.bytes().await?;
        assert!(multipart.next_field().await?.is_none());
        Ok(Json(
            serde_json::json!({"name": name, "filename": filename, "data": data.to_vec()}),
        ))
    }
    let router = router.route(
        "/upload",
        MethodRouter::new()?.on_handler(Method::POST, upload)?,
    )?;
    use engine_web::middleware::{self, Layer};
    let service = middleware::map_response(|mut response: Response| async move {
        response
            .headers_mut()
            .insert("x-middleware", "host".parse().unwrap());
        response
    })
    .layer(middleware::handler_service(
        |State(value): State<String>| async move { value },
        "middleware state".to_owned(),
    ));
    let router = router.route(
        "/middleware",
        MethodRouter::new()?.on_service(Method::GET, service)?,
    )?;
    let router = router.layer(middleware::map_response(
        |mut response: Response| async move {
            response
                .headers_mut()
                .insert("x-router-layer", "yes".parse().unwrap());
            response
        },
    ))?;
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
        .get(format!("http://{address}/middleware"))
        .send()
        .await?;
    assert_eq!(response.headers()["x-middleware"], "host");
    assert_eq!(response.headers()["x-router-layer"], "yes");
    assert_eq!(response.bytes().await?.as_ref(), b"middleware state");
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
    let response = client
        .post(format!("http://{address}/mounted/engine/echo/tail?x=1"))
        .body(b"mounted service".to_vec())
        .send()
        .await?;
    assert_eq!(response.text().await?, "mounted service");
    let response = client
        .get(format!("http://{address}/events"))
        .send()
        .await?;
    assert_eq!(response.headers()["content-type"], "text/event-stream");
    assert_eq!(response.headers()["cache-control"], "no-cache");
    assert_eq!(
        response.text().await?,
        "event: update\nid: 1\ndata: ready\n\n"
    );
    let response = client.post(format!("http://{address}/upload"))
        .header("content-type", "multipart/form-data; boundary=test")
        .body(b"--test\r\nContent-Disposition: form-data; name=\"file\"; filename=\"x.bin\"\r\n\r\n\x00\xffdata\r\n--test--\r\n".to_vec()).send().await?;
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(
        response.json::<serde_json::Value>().await?,
        serde_json::json!({"name":"file", "filename":"x.bin", "data":[0,255,100,97,116,97]})
    );
    shutdown.request();
    server.await??;
    #[cfg(unix)]
    unix_smoke().await?;
    println!("public engine HTTP consumer passed");
    Ok(())
}

#[cfg(unix)]
async fn unix_smoke() -> Result<(), Box<dyn std::error::Error>> {
    use simple_server::{engine_web::unix, runtime::spawn_blocking};
    use std::{
        io::{Read, Write},
        time::{Duration, SystemTime, UNIX_EPOCH},
    };
    struct SocketDir(std::path::PathBuf);
    impl Drop for SocketDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(self.0.join("http.sock"));
            let _ = std::fs::remove_dir(&self.0);
        }
    }
    let directory = std::env::temp_dir().join(format!(
        "ss-unix-{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    ));
    std::fs::create_dir(&directory)?;
    let directory = SocketDir(directory);
    let path = directory.0.join("http.sock");
    let client = unix::UnixClient::new()?;
    let listener = unix::bind(&path).await?;
    assert_eq!(listener.path(), path);
    let router = Router::new()?.route(
        "/",
        MethodRouter::new()?.on_handler(Method::GET, || async { "unix engine" })?,
    )?;
    let stop = Shutdown::new();
    let server = spawn(unix::serve(listener, router, stop.clone()));
    let response = spawn_blocking(move || -> std::io::Result<String> {
        let mut socket = std::os::unix::net::UnixStream::connect(path)?;
        socket.set_read_timeout(Some(Duration::from_secs(3)))?;
        socket.write_all(b"GET / HTTP/1.1\r\nHost: local\r\nConnection: close\r\n\r\n")?;
        let mut output = String::new();
        socket.read_to_string(&mut output)?;
        Ok(output)
    })
    .await??;
    assert!(response.starts_with("HTTP/1.1 200"));
    assert!(response.ends_with("unix engine"));
    let response = client
        .request(
            directory.0.join("http.sock"),
            engine_web::Request::new(engine_web::Body::empty()),
        )
        .await?;
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(response.into_body().collect(128).await?, "unix engine");
    stop.request();
    simple_server::time::timeout(Duration::from_secs(3), server).await???;
    assert!(
        directory.0.join("http.sock").exists(),
        "socket cleanup belongs to application"
    );
    Ok(())
}
