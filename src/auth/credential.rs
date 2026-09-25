use http::{HeaderMap, HeaderName};

/// Header syntax failure. Deliberately contains no credential bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialError {
    Missing,
    Repeated,
    InvalidText,
    InvalidScheme,
    Empty,
}
impl std::fmt::Display for CredentialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "credential header: {self:?}")
    }
}
impl std::error::Error for CredentialError {}
#[derive(Clone, Copy, Debug)]
pub enum RepeatedHeaders {
    Reject,
    First,
}
#[derive(Clone, Copy, Debug)]
pub enum SchemeCase {
    Exact,
    AsciiInsensitive,
}

/// Borrowed secret. Debug redacts it; exposure requires an explicit method call.
/// No normalization or validation of the credential itself is performed.
pub struct Credential<'a>(&'a str);
impl<'a> Credential<'a> {
    pub fn expose(&self) -> &'a str {
        self.0
    }
}
impl std::fmt::Debug for Credential<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Credential([REDACTED])")
    }
}

/// Extract one header or one literal scheme plus a single ASCII space. The
/// remaining bytes are opaque: never trimmed, split, decoded, or used to decide
/// another credential source. Default: reject repeated headers and empty values.
/// `First` and `allow_empty` are explicit compatibility options. Source fallback
/// is configured separately with CredentialSources or application policy; this
/// parser never attempts another source.
#[derive(Clone, Debug)]
pub struct HeaderCredential {
    name: HeaderName,
    scheme: Option<(String, SchemeCase)>,
    repeated: RepeatedHeaders,
    allow_empty: bool,
}
impl HeaderCredential {
    pub fn new(name: HeaderName) -> Self {
        Self {
            name,
            scheme: None,
            repeated: RepeatedHeaders::Reject,
            allow_empty: false,
        }
    }
    pub fn name(&self) -> &HeaderName {
        &self.name
    }
    pub fn with_scheme(mut self, scheme: impl Into<String>, case: SchemeCase) -> Self {
        self.scheme = Some((scheme.into(), case));
        self
    }
    pub fn repeated(mut self, repeated: RepeatedHeaders) -> Self {
        self.repeated = repeated;
        self
    }
    pub fn allow_empty(mut self, allow: bool) -> Self {
        self.allow_empty = allow;
        self
    }
    pub fn extract<'a>(&self, headers: &'a HeaderMap) -> Result<Credential<'a>, CredentialError> {
        let mut values = headers.get_all(&self.name).iter();
        let value = values.next().ok_or(CredentialError::Missing)?;
        if matches!(self.repeated, RepeatedHeaders::Reject) && values.next().is_some() {
            return Err(CredentialError::Repeated);
        }
        let mut value = value.to_str().map_err(|_| CredentialError::InvalidText)?;
        if let Some((scheme, case)) = &self.scheme {
            let prefix = value
                .get(..scheme.len())
                .ok_or(CredentialError::InvalidScheme)?;
            let matches = match case {
                SchemeCase::Exact => prefix == scheme,
                SchemeCase::AsciiInsensitive => prefix.eq_ignore_ascii_case(scheme),
            };
            if !matches {
                return Err(CredentialError::InvalidScheme);
            }
            value = value
                .get(scheme.len()..)
                .and_then(|rest| rest.strip_prefix(' '))
                .ok_or(CredentialError::InvalidScheme)?;
        }
        if value.is_empty() && !self.allow_empty {
            return Err(CredentialError::Empty);
        }
        Ok(Credential(value))
    }
}
