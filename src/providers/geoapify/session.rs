//! A session represents a state between the client and the Geoapify API. It holds
//! the necessary information for making requests, such as the API key and any
//! session-specific settings like [`RateLimiter`] information.

use crate::{
    error::{self, ForProvider},
    models::{
        provider::Provider,
        rate_limit::{RateLimit, RateLimiter},
    },
};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::de::DeserializeOwned;
use std::num::NonZeroU32;
use url::Url;

/// A session represents a state between the client and the Geoapify API. It holds
/// the necessary information for making requests, such as the API key and any
/// session-specific settings like [`RateLimiter`] information.
///
/// The API key is sent in the `x-api-key` header rather than the `apiKey`
/// query parameter, so that it does not end up in logged URLs.
///
/// Use [`Session::new`] or [`Session::new_with_rate_limit`] to create a new
/// session.
pub struct Session {
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
    const BASE_URL: &'static str = "https://api.geoapify.com/v1/";

    /// The header Geoapify reads the API key from.
    const API_KEY_HEADER: HeaderName = HeaderName::from_static("x-api-key");

    const CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
    const READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

    /// Creates a new session with the given API key and rate limit.
    pub fn new_with_rate_limit(
        api_key: impl ToString,
        rate_limit: RateLimit,
    ) -> error::Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(Self::API_KEY_HEADER, Self::api_key_header_value(api_key)?);
        Ok(Self {
            headers,
            rate_limiter: rate_limit.into(),
            client: tokio::sync::OnceCell::new(),
        })
    }

    /// Creates a new session with the given API key and the default rate
    /// limit.
    ///
    /// [`RateLimit`] can be set with [`Session::new_with_rate_limit`].
    pub fn new(api_key: impl ToString) -> error::Result<Self> {
        Self::new_with_rate_limit(
            api_key,
            RateLimit::default_for::<Self>().expect("Geoapify should have a default rate limit"),
        )
    }

    /// Replace the API key sent with every request.
    pub fn set_api_key(&mut self, api_key: impl ToString) -> error::Result<()> {
        self.headers
            .insert(Self::API_KEY_HEADER, Self::api_key_header_value(api_key)?);

        // Drop the existing client if it exists, so that a new one will be
        // created with the updated headers.
        self.client.take();
        Ok(())
    }

    /// Validate an API key and turn it into a header value.
    ///
    /// The value is marked as sensitive, so that it is redacted from the
    /// [`Debug`] output of the headers.
    fn api_key_header_value(api_key: impl ToString) -> error::Result<HeaderValue> {
        let api_key = api_key.to_string();
        if api_key.is_empty() {
            return Err(error::Error::InvalidApiKey {
                provider: Self::name(),
            });
        }

        let mut value =
            HeaderValue::from_str(&api_key).map_err(|_| error::Error::InvalidApiKey {
                provider: Self::name(),
            })?;
        value.set_sensitive(true);
        Ok(value)
    }

    /// Returns a reference to the session's headers.
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Build the full endpoint URL by joining the base URL with the provided endpoint path.
    ///
    /// The path must be relative, e.g. `"geocode/reverse"`. A leading `/`
    /// would replace the base URL's `/v1/` path.
    pub fn endpoint(endpoint: &str) -> error::Result<Url> {
        let base_url = Url::parse(Self::BASE_URL).for_provider::<Self>()?;
        base_url.join(endpoint).for_provider::<Self>()
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

                builder.build().for_provider::<Self>()
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
                    provider: Self::name(),
                    method,
                    url: Box::new(url),
                    body: !body.is_null(), // always true
                });
            }
            builder = builder.json(&body);
        }

        self.rate_limiter
            .acquire_and_execute(async move {
                let response = builder.send().await.for_provider::<Self>()?;

                if response.status().is_success() {
                    let bytes = response.bytes().await.for_provider::<Self>()?;
                    serde_json::from_slice::<T>(&bytes).for_provider::<Self>()
                } else {
                    // This assumes that a failure will still return some sort of response text body;
                    // if the API returns an octat-stream or binary response, this will fail.
                    Err(error::Error::UnexpectedResponse {
                        provider: Self::name(),
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

#[cfg(test)]
mod tests {
    use super::*;

    const API_KEY: &str = "0123456789abcdef";

    #[test]
    fn api_key_is_sent_as_sensitive_header() {
        let session = Session::new(API_KEY).unwrap();

        let value = session.headers().get("x-api-key").unwrap();
        assert_eq!(value, API_KEY);
        assert!(value.is_sensitive());
        assert!(!format!("{:?}", session.headers()).contains(API_KEY));
    }

    #[test]
    fn set_api_key_replaces_header() {
        let mut session = Session::new(API_KEY).unwrap();
        session.set_api_key("fedcba9876543210").unwrap();

        let values: Vec<_> = session.headers().get_all("x-api-key").iter().collect();
        assert_eq!(values, ["fedcba9876543210"]);
    }

    #[test]
    fn invalid_api_keys_are_rejected() {
        for api_key in ["", "has\nnewline"] {
            assert!(
                matches!(
                    Session::new(api_key),
                    Err(error::Error::InvalidApiKey {
                        provider: "geoapify"
                    })
                ),
                "{api_key:?} should be rejected"
            );
        }

        // A failed `set_api_key` keeps the old key.
        let mut session = Session::new(API_KEY).unwrap();
        assert!(session.set_api_key("").is_err());
        assert_eq!(session.headers().get("x-api-key").unwrap(), API_KEY);
    }

    #[test]
    fn endpoint_keeps_version_path() {
        assert_eq!(
            Session::endpoint("geocode/reverse").unwrap().as_str(),
            "https://api.geoapify.com/v1/geocode/reverse"
        );
    }
}
