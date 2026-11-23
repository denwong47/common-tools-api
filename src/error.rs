//! The error type returned by this crate.

/// A specialised [`Result`](std::result::Result) type for this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Any error that can occur while using this crate.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("the site ID provided to the session was invalid: {0}")]
    /// The site ID provided to the session was invalid.
    InvalidSiteId(String),

    #[error("the API key provided to the session was invalid")]
    /// The API key provided to the session was invalid.
    InvalidApiKey,

    /// A URL could not be parsed or joined, e.g. an endpoint path or the site
    /// declared in the `User-Agent`.
    #[error("invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    /// A request could not be built, e.g. due to an invalid combination of method,
    /// URL, and body.
    #[error("invalid combination of request method={method:?}, url={url:?}, has_body={body:?}")]
    InvalidRequest {
        /// The HTTP method used for the request.
        method: reqwest::Method,
        /// The URL used for the request.
        url: url::Url,
        /// Whether the request had a body or not.
        body: bool,
    },

    /// A value could not be used as an HTTP header, e.g. a site URL containing
    /// characters that are not allowed in the `User-Agent` header.
    #[error("invalid header value: {0}")]
    InvalidHeaderValue(#[from] reqwest::header::InvalidHeaderValue),

    /// A query parameter had an invalid value.
    #[error("invalid query value for when {action}: {message}")]
    InvalidQueryValue {
        action: &'static str,
        message: String,
    },

    /// The HTTP request failed: connection errors, timeouts.
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// The HTTP response was not as expected, i.e. 200 OK with a JSON body.
    /// This can happen if KeyCDN is down or if the endpoint is invalid.
    ///
    /// This should not happen under normal circumstances - KeyCDN simply ignores
    /// invalid site IDs and do not return a response. If you see this error,
    /// please check the status of the API, or whether your request had been
    /// redirected.
    #[error("unexpected response: status={status:?}, body={body:?}")]
    UnexpectedResponse {
        /// The HTTP status code returned by KeyCDN.
        status: reqwest::StatusCode,
        /// The body of the response, if any.
        body: String,
    },

    /// The response body was not the JSON this crate expected.
    #[error("failed to parse response: {0}")]
    Json(#[from] serde_json::Error),

    /// KeyCDN responded, but reported that the request was not successful.
    #[error("KeyCDN returned status {status:?}: {description}")]
    Api {
        /// The `status` field of the response, e.g. `"error"`.
        status: String,
        /// The `description` field of the response, explaining the failure.
        description: String,
    },

    /// KeyCDN reported success but the response contained no data.
    #[error("KeyCDN reported success but returned no data: {description}")]
    MissingData {
        /// The `description` field of the response.
        description: String,
    },
}
