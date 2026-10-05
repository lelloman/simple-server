//! Standalone transport proof using internal bindings, before the public Router
//! adapter is migrated. This application compiles no Axum/Hyper/Tokio source.
use serde_json::{Value, json};
use simple_server::{client::Client, runtime::spawn, time::sleep};
use simple_server_sys::{Callback, Operation, Resource, frame, resource_new, unframe};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

fn encode(value: Value) -> Vec<u8> {
    frame(&serde_json::to_vec(&value).unwrap(), &[]).unwrap()
}
fn decode(bytes: &[u8]) -> Value {
    serde_json::from_slice(unframe(bytes).unwrap().0).unwrap()
}
fn resource(kind: u32, command: Value) -> Resource {
    Resource::new(
        kind,
        decode(&resource_new(&encode(command)).unwrap())["id"]
            .as_u64()
            .unwrap(),
    )
}

#[simple_server::main]
async fn main() {
    let listener = decode(
        &Operation::new(&encode(json!({"op":"server_bind","address":"127.0.0.1:0"})))
            .unwrap()
            .await
            .unwrap(),
    );
    assert_eq!(listener["ok"], true);
    let address = listener["address"].as_str().unwrap();
    let handler = Callback::new(|request| async move {
        let request = decode(&request);
        assert_eq!(request["matched_path"], "/hello/{name}");
        let name = request["path_params"][0][1].as_str().unwrap();
        frame(
            br#"{"status":200,"headers":[]}"#,
            format!("Hello, {name}!").as_bytes(),
        )
        .unwrap()
    })
    .unwrap();
    let methods = resource(9, json!({"op":"method_new"}));
    let methods = resource(
        9,
        json!({"op":"method_handler","methods":methods.id(),"method":"GET","handler":handler.id()}),
    );
    let router = resource(8, json!({"op":"router_new"}));
    let router = resource(
        8,
        json!({"op":"router_route","router":router.id(),"path":"/hello/{name}","methods":methods.id()}),
    );
    let stopping = Arc::new(AtomicBool::new(false));
    let stop = stopping.clone();
    let shutdown = Callback::new(move |_| {
        let stop = stop.clone();
        async move {
            while !stop.load(Ordering::SeqCst) {
                sleep(Duration::from_millis(1)).await;
            }
            Vec::new()
        }
    })
    .unwrap();
    let serve = Operation::new(&encode(json!({"op":"server_serve","listener":listener["id"],"router":router.id(),"shutdown":shutdown.id()}))).unwrap();
    let task = spawn(serve);
    let response = Client::builder()
        .no_proxy()
        .build()
        .unwrap()
        .get(format!("http://{address}/hello/engine"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), "Hello, engine!");
    stopping.store(true, Ordering::SeqCst);
    assert_eq!(decode(&task.await.unwrap().unwrap())["ok"], true);
    println!("engine HTTP consumer passed");
}
