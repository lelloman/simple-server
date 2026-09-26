#![cfg(feature = "http-tracing")]
use futures_util::{FutureExt, StreamExt, stream};
use http_body_util::{BodyExt, StreamBody};
use simple_server::{
    axum::{
        Router,
        body::{Body, Bytes, to_bytes},
        http::{HeaderMap, Request, Response, StatusCode},
        middleware::{self, Next},
        routing::get,
    },
    http_tracing::{Observer, Outcome, Phase, trace, trace_with_observer},
};
type ResponseView<'a> = &'a Response<Body>;
include!("http_tracing_contract/mod.rs");
