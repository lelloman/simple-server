use super::{CookieCredential, CookieCredentialError, CredentialError, HeaderCredential};
use http::{HeaderMap, HeaderName};

/// Where the credential that was actually verified came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CredentialSource {
    Header(HeaderName),
    Cookie(String),
}

/// Owned selected secret. Debug redacts the value. Verification does not imply
/// that the raw credential should be propagated to handlers or logged.
pub struct SelectedCredential {
    value: String,
    source: CredentialSource,
}
impl SelectedCredential {
    pub fn expose(&self) -> &str {
        &self.value
    }
    pub fn source(&self) -> &CredentialSource {
        &self.source
    }
}
impl std::fmt::Debug for SelectedCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelectedCredential")
            .field("source", &self.source)
            .field("value", &"[REDACTED]")
            .finish()
    }
}

/// Source syntax failures remain separate from verifier/backend failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialSelectionError {
    Missing,
    Header(CredentialError),
    Cookie(CookieCredentialError),
}
impl std::fmt::Display for CredentialSelectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "credential selection: {self:?}")
    }
}
impl std::error::Error for CredentialSelectionError {}

/// Missing sources always proceed to the next source. Malformed sources reject
/// by default; trying another source requires an explicit compatibility choice.
/// Verifier failures NEVER trigger fallback.
#[derive(Clone, Copy, Debug, Default)]
pub enum MalformedCredentials {
    #[default]
    Reject,
    TryNext,
}
#[derive(Clone, Debug)]
enum Source {
    Header(HeaderCredential),
    Cookie(CookieCredential),
}

/// Nonempty ordered credential sources. Only the first successfully extracted
/// credential is verified. Lower-priority sources are not inspected thereafter.
/// If every source is absent, selection returns Missing. If fallback is enabled
/// and none succeeds, the first syntax error is retained (never made anonymous).
#[derive(Clone, Debug)]
pub struct CredentialSources {
    sources: Vec<Source>,
    malformed: MalformedCredentials,
}
impl CredentialSources {
    pub fn header(header: HeaderCredential) -> Self {
        Self {
            sources: vec![Source::Header(header)],
            malformed: MalformedCredentials::Reject,
        }
    }
    pub fn cookie(cookie: CookieCredential) -> Self {
        Self {
            sources: vec![Source::Cookie(cookie)],
            malformed: MalformedCredentials::Reject,
        }
    }
    pub fn or_header(mut self, header: HeaderCredential) -> Self {
        self.sources.push(Source::Header(header));
        self
    }
    pub fn or_cookie(mut self, cookie: CookieCredential) -> Self {
        self.sources.push(Source::Cookie(cookie));
        self
    }
    pub fn on_malformed(mut self, policy: MalformedCredentials) -> Self {
        self.malformed = policy;
        self
    }
    pub fn extract(
        &self,
        headers: &HeaderMap,
    ) -> Result<SelectedCredential, CredentialSelectionError> {
        let mut first_error = None;
        for source in &self.sources {
            let result = match source {
                Source::Header(parser) => parser
                    .extract(headers)
                    .map(|v| SelectedCredential {
                        value: v.expose().to_owned(),
                        source: CredentialSource::Header(parser.name().clone()),
                    })
                    .map_err(|e| {
                        if e == CredentialError::Missing {
                            CredentialSelectionError::Missing
                        } else {
                            CredentialSelectionError::Header(e)
                        }
                    }),
                Source::Cookie(parser) => parser
                    .extract(headers)
                    .map(|v| SelectedCredential {
                        value: v.expose().to_owned(),
                        source: CredentialSource::Cookie(parser.name().to_owned()),
                    })
                    .map_err(|e| {
                        if e == CookieCredentialError::Missing {
                            CredentialSelectionError::Missing
                        } else {
                            CredentialSelectionError::Cookie(e)
                        }
                    }),
            };
            match result {
                Ok(selected) => return Ok(selected),
                Err(CredentialSelectionError::Missing) => {}
                Err(error) => match self.malformed {
                    MalformedCredentials::Reject => return Err(error),
                    MalformedCredentials::TryNext => {
                        first_error.get_or_insert(error);
                    }
                },
            }
        }
        Err(first_error.unwrap_or(CredentialSelectionError::Missing))
    }
}

/// Optional admits missing credentials only, never malformed or rejected ones.
#[derive(Clone, Copy, Debug)]
pub enum Authentication {
    Required,
    Optional,
}

/// Selection and application access failures have distinct error paths. Debug
/// deliberately omits application error data, which might contain credentials.
pub enum CredentialAuthError<E> {
    Selection(CredentialSelectionError),
    Access(E),
}
impl<E> std::fmt::Debug for CredentialAuthError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Selection(e) => f.debug_tuple("Selection").field(e).finish(),
            Self::Access(_) => f.write_str("Access([REDACTED])"),
        }
    }
}
impl<E> std::fmt::Display for CredentialAuthError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Selection(e) => e.fmt(f),
            Self::Access(_) => f.write_str("credential access failed"),
        }
    }
}
impl<E: std::error::Error + 'static> std::error::Error for CredentialAuthError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Selection(e) => Some(e),
            Self::Access(e) => Some(e),
        }
    }
}
