//! The [Forward Geocoding API](https://apidocs.geoapify.com/docs/geocoding/forward-geocoding/)
//! (`GET /v1/geocode/search`): find the places matching an address.
//!
//! The request and response models are in
//! [`models::forward_geocoding`](crate::providers::geoapify::models::forward_geocoding).

use super::super as geoapify;
use super::super::models::forward_geocoding::{ForwardGeocodingQuery, ForwardGeocodingResponse};

use crate::traits::query::CanExportQuery;

/// The endpoint path, relative to [`Session`](geoapify::Session)'s base URL.
const ENDPOINT: &str = "geocode/search";

impl geoapify::Session {
    /// Find the places matching the address in `query`.
    ///
    /// The results are sorted best match first. Check each result's
    /// [`rank`](crate::providers::geoapify::models::geocoding::GeocodingResult::rank)
    /// to see how well it matches: `confidence` is from 0 to 1, and
    /// `match_type` says which address level it matched at. If Geoapify finds
    /// nothing, [`results`](ForwardGeocodingResponse::results) is empty
    /// rather than an error.
    ///
    /// Geoapify's errors (e.g. HTTP 400 for invalid parameters, 401 for an
    /// invalid API key, 429 for going over the quota) are returned as
    /// [`Error::UnexpectedResponse`](crate::error::Error::UnexpectedResponse),
    /// with Geoapify's JSON error body as the `body`.
    pub async fn forward_geocode(
        &self,
        query: &ForwardGeocodingQuery,
    ) -> crate::error::Result<ForwardGeocodingResponse> {
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
    use super::super::super::models::{
        forward_geocoding::{Bias, CountryCodes, Filter, LocationType, StructuredAddress},
        geocoding::{Coordinates, ResultType},
    };
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

    /// The address of the result in the reverse geocoding sample response.
    #[tokio::test]
    async fn forward_geocode_text() {
        let Some(session) = session_from_env() else {
            return;
        };

        let query = ForwardGeocodingQuery::text("Leiper Weg 1, 03222 Lübbenau/Spreewald, Germany")
            .with_bias(Bias::default().with_country_codes(CountryCodes::Disabled));
        let response = session
            .forward_geocode(&query)
            .await
            .expect("forward geocoding failed");

        let result = response.results.first().expect("no results");
        assert_eq!(result.country_code.as_deref(), Some("de"));
        assert_eq!(result.postcode.as_deref(), Some("03222"));
        assert!(
            result
                .rank
                .as_ref()
                .and_then(|rank| rank.confidence)
                .is_some()
        );

        let echo = response.query.expect("no query echo");
        assert!(echo.parsed.is_some());
    }

    #[tokio::test]
    async fn forward_geocode_structured_with_filter() {
        let Some(session) = session_from_env() else {
            return;
        };

        let query = ForwardGeocodingQuery::structured(StructuredAddress {
            city: Some("Berlin".to_owned()),
            ..Default::default()
        })
        .with_type(LocationType::City)
        .with_limit(3)
        .with_filter(
            Filter::default().with_country_codes(CountryCodes::List(vec!["de".to_owned()])),
        )
        .with_bias(Bias::default().with_proximity(Coordinates {
            lon: 13.4,
            lat: 52.5,
        }));
        let response = session
            .forward_geocode(&query)
            .await
            .expect("forward geocoding failed");

        assert!(!response.results.is_empty());
        assert!(response.results.len() <= 3);
        for result in &response.results {
            assert_eq!(result.country_code.as_deref(), Some("de"));
            assert_eq!(result.result_type, Some(ResultType::City));
        }
    }

    /// Needs no API key, since it checks that an invalid one is rejected.
    #[tokio::test]
    async fn invalid_api_key_is_unexpected_response() {
        let session = geoapify::Session::new("not-a-valid-key").unwrap();

        let query = ForwardGeocodingQuery::text("Berlin");
        match session.forward_geocode(&query).await {
            Err(Error::UnexpectedResponse {
                provider, status, ..
            }) => {
                assert_eq!(provider, "geoapify");
                assert_eq!(status, reqwest::StatusCode::UNAUTHORIZED);
            }
            other => panic!("expected Error::UnexpectedResponse, got {other:?}"),
        }
    }
}
