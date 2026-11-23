//! The error type returned by this crate.

use crate::models::provider::Provider;

/// A specialised [`Result`](std::result::Result) type for this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Any error that can occur while using this crate.
///
/// Errors raised by a provider's session carry a `provider` field: the
/// [`Provider::name`] of that session, e.g. `"keycdn"` or `"geoapify"`. Use
/// [`Error::provider`] to read it without matching on the variant.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// The site ID provided to the session was invalid. Used by KeyCDN, whose
    /// sessions identify the site making requests.
    #[error("[{provider}] the site ID provided to the session was invalid: {site}")]
    InvalidSiteId {
        /// The provider whose session rejected the site ID.
        provider: &'static str,
        /// The site ID that was rejected.
        site: String,
    },

    /// The API key provided to the session was invalid, e.g. empty.
    #[error("[{provider}] the API key provided to the session was invalid")]
    InvalidApiKey {
        /// The provider whose session rejected the API key.
        provider: &'static str,
    },

    /// A URL could not be parsed or joined, e.g. an endpoint path.
    #[error("[{provider}] invalid URL: {source}")]
    InvalidUrl {
        /// The provider whose session built the URL.
        provider: &'static str,
        source: url::ParseError,
    },

    /// A request could not be built, e.g. due to an invalid combination of method,
    /// URL, and body.
    #[error(
        "[{provider}] invalid combination of request method={method:?}, url={url:?}, has_body={body:?}"
    )]
    InvalidRequest {
        /// The provider the request was for.
        provider: &'static str,
        /// The HTTP method used for the request.
        method: reqwest::Method,
        /// The URL used for the request. Boxed to keep [`Error`] small.
        url: Box<url::Url>,
        /// Whether the request had a body or not.
        body: bool,
    },

    /// A value could not be used as an HTTP header, e.g. a site URL containing
    /// characters that are not allowed in the `User-Agent` header.
    #[error("[{provider}] invalid header value: {source}")]
    InvalidHeaderValue {
        /// The provider whose session built the header.
        provider: &'static str,
        source: reqwest::header::InvalidHeaderValue,
    },

    /// A query could not be converted into query parameters by
    /// [`CanExportQuery`](crate::traits::query::CanExportQuery).
    ///
    /// This has no `provider`, since queries are converted before any
    /// session is involved.
    #[error("invalid query value for when {action}: {message}")]
    InvalidQueryValue {
        action: &'static str,
        message: String,
    },

    /// The HTTP request failed: connection errors, timeouts.
    #[error("[{provider}] HTTP request failed: {source}")]
    Http {
        /// The provider the request was sent to.
        provider: &'static str,
        source: reqwest::Error,
    },

    /// The provider responded with a non-2xx status.
    ///
    /// Providers that report errors with HTTP statuses, such as Geoapify, end
    /// up here, with their error body in `body`. For KeyCDN this should not
    /// happen under normal circumstances, since it reports errors in a 200
    /// response (see [`Error::Api`]) and drops invalid requests without
    /// responding. If you see it from KeyCDN, check the status of the API, or
    /// whether your request had been redirected.
    #[error("[{provider}] unexpected response: status={status:?}, body={body:?}")]
    UnexpectedResponse {
        /// The provider that responded.
        provider: &'static str,
        /// The HTTP status code returned by the provider.
        status: reqwest::StatusCode,
        /// The body of the response, if any.
        body: String,
    },

    /// The response body was not the JSON this crate expected.
    #[error("[{provider}] failed to parse response: {source}")]
    Json {
        /// The provider that responded.
        provider: &'static str,
        source: serde_json::Error,
    },

    /// The provider responded successfully at the HTTP level, but its response
    /// body reported that the request failed. Used by KeyCDN.
    #[error("[{provider}] API returned status {status:?}: {description}")]
    Api {
        /// The provider that responded.
        provider: &'static str,
        /// The `status` field of the response, e.g. `"error"`.
        status: String,
        /// The `description` field of the response, explaining the failure.
        description: String,
    },

    /// The provider reported success, but the response contained no data. Used
    /// by KeyCDN.
    #[error("[{provider}] API reported success but returned no data: {description}")]
    MissingData {
        /// The provider that responded.
        provider: &'static str,
        /// The `description` field of the response.
        description: String,
    },
}

impl Error {
    /// The [`Provider::name`] of the session that raised this error, or
    /// `None` if it was not raised by a session.
    pub fn provider(&self) -> Option<&'static str> {
        match self {
            Self::InvalidSiteId { provider, .. }
            | Self::InvalidApiKey { provider }
            | Self::InvalidUrl { provider, .. }
            | Self::InvalidRequest { provider, .. }
            | Self::InvalidHeaderValue { provider, .. }
            | Self::Http { provider, .. }
            | Self::UnexpectedResponse { provider, .. }
            | Self::Json { provider, .. }
            | Self::Api { provider, .. }
            | Self::MissingData { provider, .. } => Some(provider),
            Self::InvalidQueryValue { .. } => None,
        }
    }
}

/// Errors from other crates that a session can run into, and which
/// [`Error`] variant each one becomes once its provider is known.
///
/// These replace `#[from]` conversions, which cannot fill in the `provider`
/// field.
#[allow(unused)]
pub(crate) trait IntoProviderError {
    fn into_provider_error(self, provider: &'static str) -> Error;
}

impl IntoProviderError for url::ParseError {
    fn into_provider_error(self, provider: &'static str) -> Error {
        Error::InvalidUrl {
            provider,
            source: self,
        }
    }
}

impl IntoProviderError for reqwest::header::InvalidHeaderValue {
    fn into_provider_error(self, provider: &'static str) -> Error {
        Error::InvalidHeaderValue {
            provider,
            source: self,
        }
    }
}

impl IntoProviderError for reqwest::Error {
    fn into_provider_error(self, provider: &'static str) -> Error {
        Error::Http {
            provider,
            source: self,
        }
    }
}

impl IntoProviderError for serde_json::Error {
    fn into_provider_error(self, provider: &'static str) -> Error {
        Error::Json {
            provider,
            source: self,
        }
    }
}

/// Converts a foreign error into an [`Error`] for provider `P`, so that a
/// session can write `fallible().for_provider::<Self>()?`.
#[allow(unused)]
pub(crate) trait ForProvider<T> {
    fn for_provider<P: Provider>(self) -> Result<T>;
}

impl<T, E: IntoProviderError> ForProvider<T> for std::result::Result<T, E> {
    fn for_provider<P: Provider>(self) -> Result<T> {
        self.map_err(|err| err.into_provider_error(P::name()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::rate_limit::RateLimit;

    struct TestProvider;

    impl Provider for TestProvider {
        fn name<'s>() -> &'s str {
            "test"
        }

        fn rate_limit() -> Option<RateLimit> {
            None
        }
    }

    #[test]
    fn for_provider_fills_in_provider() {
        let err = url::Url::parse("not a url")
            .for_provider::<TestProvider>()
            .unwrap_err();

        assert!(matches!(
            err,
            Error::InvalidUrl {
                provider: "test",
                ..
            }
        ));
        assert_eq!(err.provider(), Some("test"));
        assert!(err.to_string().starts_with("[test] invalid URL: "));
    }

    #[test]
    fn query_errors_have_no_provider() {
        let err = Error::InvalidQueryValue {
            action: "testing",
            message: "message".to_owned(),
        };

        assert_eq!(err.provider(), None);
    }
}
