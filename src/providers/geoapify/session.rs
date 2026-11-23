//! A session represents a state between the client and the Geoapify API. It holds
//! the necessary information for making requests, such as the site declaration
//! and any session-specific settings like [`RateLimiter`] information.

use crate::{
    error,
    models::{
        provider::Provider,
        rate_limit::{RateLimit, RateLimiter},
    },
};
use reqwest::header::HeaderMap;
use serde::de::DeserializeOwned;
use std::num::NonZeroU32;
use url::Url;

/// A session represents a state between the client and the Geoapify API. It holds
/// the necessary information for making requests, such as the site declaration
/// and any session-specific settings like [`RateLimiter`] information.
///
/// Use [`Session::new`] or [`Session::new_with_rate_limit`] to create a new
/// session.
pub struct Session {
    api_key: String,
    headers: HeaderMap,
    rate_limiter: RateLimiter,
    client: tokio::sync::OnceCell<reqwest::Client>,
}

impl Provider for Session {
    fn name<'s>() -> &'s str {
        "geoapify"
    }

    fn rate_limit() -> Option<RateLimit> {
        Some(RateLimit::new_per_second(NonZeroU32::new(5).unwrap()))
    }
}

impl Session {
    const BASE_URL: &'static str = "https://maps.geoapify.com/";

    const CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
    const READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

    /// Creates a new session with the given site ID and default headers.
    pub fn new_with_rate_limit(
        api_key: impl ToString,
        rate_limit: RateLimit,
    ) -> error::Result<Self> {
        let headers = HeaderMap::new();
        Ok(Self {
            api_key: api_key.to_string(), // Initialize the API key as an empty string or set it appropriately
            headers,
            rate_limiter: rate_limit.into(),
            client: tokio::sync::OnceCell::new(),
        })
    }

    /// Creates a new session with the given site ID and default headers and rate limit.
    ///
    /// [`RateLimit`] can be set with [`Session::new_with_rate_limit`].
    pub fn new(site: &str) -> error::Result<Self> {
        Self::new_with_rate_limit(
            site,
            RateLimit::default_for::<Self>().expect("Geoapify should have a default rate limit"),
        )
    }

    /// Set the site URL which is required for the `User-Agent` header. This is
    /// used to identify the site making requests to the Geoapify API.
    pub fn set_api_key(&mut self, api_key: impl ToString) -> error::Result<()> {
        // Validate that the api_key is not empty.
        let api_key = api_key.to_string();
        if api_key.is_empty() {
            return Err(error::Error::InvalidApiKey);
        }

        self.api_key = api_key;

        Ok(())
    }

    /// Returns a reference to the session's headers.
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Build the full endpoint URL by joining the base URL with the provided endpoint path.
    pub fn endpoint(endpoint: &str) -> error::Result<Url> {
        let base_url = Url::parse(Self::BASE_URL)?;
        base_url.join(endpoint).map_err(error::Error::from)
    }

    /// Returns a reference to the session's internal
    /// [`client`](reqwest::Client), initializing it if it hasn't been created
    /// yet.
    pub async fn client(&self) -> error::Result<&reqwest::Client> {
        self.client
            .get_or_try_init(|| async {
                let builder = reqwest::Client::builder()
                    .default_headers(self.headers.clone())
                    .connect_timeout(Self::CONNECT_TIMEOUT)
                    .read_timeout(Self::READ_TIMEOUT);

                #[cfg(feature = "rustls")]
                let builder = builder.use_rustls_tls();

                // If both `rustls` and `native-tls` features are enabled, prefer `rustls`.
                #[cfg(all(feature = "native-tls", not(feature = "rustls")))]
                let builder = builder.use_native_tls();

                Ok(builder.build()?)
            })
            .await
    }

    /// Makes a request, handling rate limiting and returning the deserialized
    /// response.
    pub async fn request<T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        endpoint: &str,
        query: Option<&[(&str, &str)]>,
        headers: Option<HeaderMap>,
        body: Option<serde_json::Value>,
    ) -> error::Result<T> {
        let url = Self::endpoint(endpoint)?;
        let client = self.client().await?;

        let mut builder = client.request(method.clone(), url.clone());

        if let Some(additional_headers) = headers {
            builder = builder.headers(additional_headers);
        }

        if let Some(query_params) = query {
            builder = builder.query(query_params);
        }

        if let Some(body) = body {
            // Reject requests with a body for GET or HEAD methods, as they should not have a body.
            if [reqwest::Method::GET, reqwest::Method::HEAD].contains(&method) {
                return Err(error::Error::InvalidRequest {
                    method,
                    url,
                    body: !body.is_null(), // always true
                });
            }
            builder = builder.json(&body);
        }

        self.rate_limiter
            .acquire_and_execute(async move {
                let response = builder.send().await?;

                if response.status().is_success() {
                    let bytes = response.bytes().await?;
                    serde_json::from_slice::<T>(&bytes).map_err(error::Error::from)
                } else {
                    // This assumes that a failure will still return some sort of response text body;
                    // if the API returns an octat-stream or binary response, this will fail.
                    Err(error::Error::UnexpectedResponse {
                        status: response.status(),
                        body: response.text().await.unwrap_or_else(|err| {
                            format!("Failed to read response body due to: {}", err)
                        }),
                    })
                }
            })
            .await
    }
}
