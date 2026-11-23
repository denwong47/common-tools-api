//! The [Reverse Geocoding API](https://apidocs.geoapify.com/docs/geocoding/reverse-geocoding/)
//! (`GET /v1/geocode/reverse`): find the address or place at a pair of
//! coordinates.
//!
//! The request and response models are in
//! [`models::reverse_geocoding`](crate::providers::geoapify::models::reverse_geocoding).

use super::super as geoapify;
use super::super::models::reverse_geocoding::{ReverseGeocodingQuery, ReverseGeocodingResponse};

use crate::traits::query::CanExportQuery;

/// The endpoint path, relative to [`Session`](geoapify::Session)'s base URL.
const ENDPOINT: &str = "geocode/reverse";

impl geoapify::Session {
    /// Find the places at, or nearest to, the coordinates in `query`.
    ///
    /// The results are sorted best match first. If Geoapify finds nothing,
    /// [`results`](ReverseGeocodingResponse::results) is empty rather than an
    /// error.
    ///
    /// Geoapify's errors (e.g. HTTP 400 for out-of-range coordinates, 401 for
    /// an invalid API key, 429 for going over the quota) are returned as
    /// [`Error::UnexpectedResponse`](crate::error::Error::UnexpectedResponse),
    /// with Geoapify's JSON error body as the `body`.
    pub async fn reverse_geocode(
        &self,
        query: &ReverseGeocodingQuery,
    ) -> crate::error::Result<ReverseGeocodingResponse> {
        let query = query.to_query()?;
        let query: Vec<(&str, &str)> = query
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect();

        self.request(reqwest::Method::GET, ENDPOINT, Some(&query), None, None)
            .await
    }
}

#[cfg(all(test, feature = "test-integration"))]
mod tests {
    use super::super::super::models::{geocoding::ResultType, reverse_geocoding::LocationType};
    use super::*;
    use crate::error::Error;

    /// The environment variable the integration tests read the API key from.
    const API_KEY_VAR: &str = "GEOAPIFY_API_KEY";

    /// A session using the API key from [`API_KEY_VAR`], or `None` if it is
    /// not set, so that the tests can be skipped where no key is available.
    fn session_from_env() -> Option<geoapify::Session> {
        match std::env::var(API_KEY_VAR) {
            Ok(api_key) => Some(geoapify::Session::new(api_key).expect("invalid API key")),
            Err(_) => {
                eprintln!("{API_KEY_VAR} is not set; skipping test");
                None
            }
        }
    }

    /// The coordinates from the example response in the module docs of
    /// `models::reverse_geocoding`: a restaurant in Lehde, Germany.
    #[tokio::test]
    async fn reverse_geocode_lehde() {
        let Some(session) = session_from_env() else {
            return;
        };

        let query = ReverseGeocodingQuery::new(51.87485003734697, 14.008155801332068);
        let response = session
            .reverse_geocode(&query)
            .await
            .expect("reverse geocoding failed");

        let result = response.results.first().expect("no results");
        assert_eq!(result.country_code.as_deref(), Some("de"));
        assert_eq!(result.state.as_deref(), Some("Brandenburg"));
        assert_eq!(result.postcode.as_deref(), Some("03222"));
        assert!(result.timezone.is_some());

        let echo = response.query.expect("no query echo");
        assert_eq!(echo.lat, 51.87485003734697);
        assert_eq!(echo.lon, 14.008155801332068);
    }

    #[tokio::test]
    async fn reverse_geocode_with_options() {
        let Some(session) = session_from_env() else {
            return;
        };

        let query = ReverseGeocodingQuery::new(51.87485003734697, 14.008155801332068)
            .with_type(LocationType::City)
            .with_lang("de")
            .with_limit(3)
            .with_country_codes(["de"]);
        let response = session
            .reverse_geocode(&query)
            .await
            .expect("reverse geocoding failed");

        assert!(!response.results.is_empty());
        assert!(response.results.len() <= 3);
        assert!(
            response
                .results
                .iter()
                .all(|result| result.result_type == Some(ResultType::City)),
            "expected only cities, got {:?}",
            response.results
        );
    }

    /// Needs no API key, since it checks that an invalid one is rejected.
    #[tokio::test]
    async fn invalid_api_key_is_unexpected_response() {
        let session = geoapify::Session::new("not-a-valid-key").unwrap();

        let query = ReverseGeocodingQuery::new(51.5, -0.1);
        match session.reverse_geocode(&query).await {
            Err(Error::UnexpectedResponse { status, .. }) => {
                assert_eq!(status, reqwest::StatusCode::UNAUTHORIZED);
            }
            other => panic!("expected Error::UnexpectedResponse, got {other:?}"),
        }
    }
}
