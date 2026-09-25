use http::{HeaderMap, header::COOKIE};

/// Cookie extraction failure. Contains no cookie values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CookieCredentialError {
    Missing,
    Repeated,
    InvalidText,
    Malformed,
    Empty,
}
impl std::fmt::Display for CookieCredentialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cookie credential: {self:?}")
    }
}
impl std::error::Error for CookieCredentialError {}

/// Invalid configured cookie name (must be a nonempty ASCII HTTP token).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidCookieName;
impl std::fmt::Display for InvalidCookieName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid credential cookie name")
    }
}
impl std::error::Error for InvalidCookieName {}

/// Handling of repeated occurrences of the configured cookie, across all headers.
#[derive(Clone, Copy, Debug)]
pub enum RepeatedCookies {
    Reject,
    First,
    Last,
}

/// Read one named request cookie. Names are case-sensitive. Values are opaque:
/// no percent decoding or signature/decryption is performed. Matching values
/// must contain cookie-octets; surrounding quotes are removed. Empty values and
/// duplicates are rejected by default. Unrelated cookie pairs are ignored.
/// Invalid header text and malformed matching pairs are errors even with First
/// or Last duplicate handling. This parses credentials, not a general cookie jar.
#[derive(Clone, Debug)]
pub struct CookieCredential {
    name: String,
    repeated: RepeatedCookies,
    allow_empty: bool,
}
impl CookieCredential {
    pub fn new(name: impl Into<String>) -> Result<Self, InvalidCookieName> {
        let name = name.into();
        if name.is_empty()
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
        {
            return Err(InvalidCookieName);
        }
        Ok(Self {
            name,
            repeated: RepeatedCookies::Reject,
            allow_empty: false,
        })
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn repeated(mut self, policy: RepeatedCookies) -> Self {
        self.repeated = policy;
        self
    }
    pub fn allow_empty(mut self, allow: bool) -> Self {
        self.allow_empty = allow;
        self
    }
    pub fn extract<'a>(
        &self,
        headers: &'a HeaderMap,
    ) -> Result<super::Credential<'a>, CookieCredentialError> {
        let mut found = None;
        for line in headers.get_all(COOKIE) {
            let line = line
                .to_str()
                .map_err(|_| CookieCredentialError::InvalidText)?;
            for pair in line.split(';') {
                let pair = pair.trim_matches([' ', '\t']);
                let (name, value) = match pair.split_once('=') {
                    Some(pair) => pair,
                    None if pair == self.name => return Err(CookieCredentialError::Malformed),
                    None => continue,
                };
                if name.trim_matches([' ', '\t']) != self.name {
                    continue;
                }
                if name != self.name {
                    return Err(CookieCredentialError::Malformed);
                }
                let value = if value.starts_with('"') {
                    value
                        .strip_prefix('"')
                        .and_then(|v| v.strip_suffix('"'))
                        .ok_or(CookieCredentialError::Malformed)?
                } else {
                    value
                };
                if !value.bytes().all(
                    |b| matches!(b, 0x21 | 0x23..=0x2b | 0x2d..=0x3a | 0x3c..=0x5b | 0x5d..=0x7e),
                ) {
                    return Err(CookieCredentialError::Malformed);
                }
                if value.is_empty() && !self.allow_empty {
                    return Err(CookieCredentialError::Empty);
                }
                if found.is_some() {
                    match self.repeated {
                        RepeatedCookies::Reject => return Err(CookieCredentialError::Repeated),
                        RepeatedCookies::First => continue,
                        RepeatedCookies::Last => {}
                    }
                }
                found = Some(super::Credential::from_cookie(value));
            }
        }
        found.ok_or(CookieCredentialError::Missing)
    }
}
