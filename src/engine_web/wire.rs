use super::{Body, Request, RequestMetadata, StatusCode, Version};
use serde_json::{Value, json};
use simple_server_sys::{Resource, frame, resource_new, unframe};
use std::io;

pub(super) fn encode(header: Value, payload: &[u8]) -> io::Result<Vec<u8>> {
    frame(&serde_json::to_vec(&header)?, payload)
}
pub(super) fn decode(bytes: &[u8]) -> io::Result<(Value, &[u8])> {
    let (header, body) = unframe(bytes)?;
    Ok((serde_json::from_slice(header)?, body))
}
pub(super) fn check(header: &Value) -> io::Result<()> {
    if header["ok"].as_bool() == Some(true) {
        return Ok(());
    }
    if let Some(code) = header["raw_os_error"]
        .as_i64()
        .and_then(|code| i32::try_from(code).ok())
    {
        return Err(io::Error::from_raw_os_error(code));
    }
    Err(io::Error::other(
        header["message"]
            .as_str()
            .unwrap_or("engine HTTP operation failed"),
    ))
}
pub(super) fn id(header: &Value) -> io::Result<u64> {
    header["id"]
        .as_u64()
        .ok_or_else(|| io::Error::other("missing engine HTTP resource"))
}
pub(super) fn resource(kind: u32, command: Value) -> io::Result<Resource> {
    let bytes = resource_new(&encode(command, &[])?)?;
    let (header, _) = decode(&bytes)?;
    check(&header)?;
    Ok(Resource::new(kind, id(&header)?))
}
pub(super) fn headers(headers: &http::HeaderMap) -> Value {
    json!(
        headers
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_bytes()))
            .collect::<Vec<_>>()
    )
}
pub(super) fn read_headers(value: &Value) -> io::Result<http::HeaderMap> {
    let mut headers = http::HeaderMap::new();
    let pairs: Vec<(String, Vec<u8>)> = serde_json::from_value(value.clone())?;
    for (name, value) in pairs {
        headers.append(
            http::HeaderName::from_bytes(name.as_bytes()).map_err(io::Error::other)?,
            http::HeaderValue::from_bytes(&value).map_err(io::Error::other)?,
        );
    }
    Ok(headers)
}
pub(super) fn request(bytes: &[u8]) -> io::Result<Request> {
    let (header, _) = decode(bytes)?;
    // Acquire our own body registration while the callback's borrowed ID is live.
    let body = resource(6, json!({"op":"server_body_clone","body":header["body"]}))?;
    let body = Body::incoming(body, &header)?;
    let mut request = http::Request::builder()
        .method(
            header["method"]
                .as_str()
                .ok_or_else(|| io::Error::other("missing request method"))?,
        )
        .uri(
            header["uri"]
                .as_str()
                .ok_or_else(|| io::Error::other("missing request URI"))?,
        )
        .body(body)
        .map_err(io::Error::other)?;
    *request.version_mut() = version(&header)?;
    *request.headers_mut() = read_headers(&header["headers"])?;
    let metadata = RequestMetadata {
        peer: header["peer"]
            .as_str()
            .map(str::parse)
            .transpose()
            .map_err(io::Error::other)?,
        matched_path: header["matched_path"].as_str().map(str::to_owned),
        original_uri: header["original_uri"]
            .as_str()
            .map(str::parse)
            .transpose()
            .map_err(io::Error::other)?,
        path_params: if header["path_params"].is_null() {
            Vec::new()
        } else {
            serde_json::from_value(header["path_params"].clone())?
        },
        path_error: header["path_error"].as_str().map(|message| {
            (
                header["path_status"]
                    .as_u64()
                    .and_then(|code| u16::try_from(code).ok())
                    .and_then(|code| StatusCode::from_u16(code).ok())
                    .unwrap_or(StatusCode::BAD_REQUEST),
                message.to_owned(),
            )
        }),
    };
    *request.extensions_mut() =
        super::continuation::request_extensions(header["context"].as_u64().unwrap_or(0));
    if let Some(path) = metadata.matched_path.as_ref() {
        request
            .extensions_mut()
            .insert(super::extract::MatchedPath(path.clone()));
    } else {
        request
            .extensions_mut()
            .remove::<super::extract::MatchedPath>();
    }
    if let Some(peer) = metadata.peer {
        request
            .extensions_mut()
            .insert(super::extract::ConnectInfo(peer));
    }
    request.extensions_mut().insert(metadata);
    if let Some(id) = header["native_extensions"].as_u64() {
        let resource = resource(17, json!({"op":"tower_extensions_clone","extensions":id}))?;
        request
            .extensions_mut()
            .insert(super::continuation::NativeExtensions(resource));
    }
    Ok(request)
}

pub(super) fn version(header: &Value) -> io::Result<Version> {
    Ok(match header["version"].as_str() {
        Some("HTTP/0.9") => Version::HTTP_09,
        Some("HTTP/1.0") => Version::HTTP_10,
        Some("HTTP/1.1") => Version::HTTP_11,
        Some("HTTP/2.0") => Version::HTTP_2,
        Some("HTTP/3.0") => Version::HTTP_3,
        _ => return Err(io::Error::other("invalid HTTP version")),
    })
}
