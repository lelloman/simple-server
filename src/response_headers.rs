//! Explicit response-header operations, independent of Axum and lifecycle.
//!
//! Applications own header values, route selection, cache eligibility and layer
//! placement. These helpers only mutate the supplied header map; they never
//! inspect or poll a response body. Nothing is installed automatically.
//!
//! ```
//! use simple_server::response_headers::{self, HeaderMap, HeaderValue, header};
//! let mut headers = HeaderMap::new();
//! response_headers::insert_if_absent(
//!     &mut headers, header::CACHE_CONTROL, HeaderValue::from_static("no-store"),
//! );
//! response_headers::merge_vary(&mut headers, "Authorization")?;
//! # Ok::<(), simple_server::response_headers::InvalidHeaderName>(())
//! ```

// These are framework-independent HTTP primitives, not Axum aliases.
pub use http::header::InvalidHeaderName;
pub use http::{HeaderMap, HeaderName, HeaderValue, header};

/// Insert a value only when the field is absent. If present, preserve every
/// existing value (including empty values and sensitive flags).
pub fn insert_if_absent(headers: &mut HeaderMap, name: HeaderName, value: HeaderValue) {
    headers.entry(name).or_insert(value);
}

/// Replace all existing values of this field with one explicit value.
/// Other fields, including repeated `Set-Cookie` values, are untouched.
pub fn replace(headers: &mut HeaderMap, name: HeaderName, value: HeaderValue) {
    headers.insert(name, value);
}

/// Merge one field name (or `*`) into Vary, comparing names without ASCII case.
///
/// Inspect every existing field line and comma-separated token. An existing
/// wildcard or matching name makes this a no-op. Otherwise append a separate
/// Vary field line, retaining all existing bytes, repeated values and sensitive
/// flags. Existing duplicates and malformed/non-ASCII values are not rewritten.
/// A requested wildcard is appended unless one already exists.
///
/// Invalid input (including empty names or comma-separated lists) returns an
/// error without changing the map. The supplied spelling is preserved.
pub fn merge_vary(headers: &mut HeaderMap, required: &str) -> Result<(), InvalidHeaderName> {
    // Validate exactly one HTTP field name before reading or mutating the map.
    HeaderName::from_bytes(required.as_bytes())?;
    let exists = headers.get_all(header::VARY).iter().any(|value| {
        value.as_bytes().split(|&b| b == b',').any(|token| {
            let token = token.trim_ascii();
            token == b"*" || token.eq_ignore_ascii_case(required.as_bytes())
        })
    });
    if !exists {
        let value = HeaderValue::from_bytes(required.as_bytes())
            .expect("a validated HTTP field name is a valid header value");
        headers.append(header::VARY, value);
    }
    Ok(())
}
