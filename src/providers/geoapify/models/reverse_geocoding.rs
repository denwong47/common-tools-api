//! Models for the
//! [Reverse Geocoding API](https://apidocs.geoapify.com/docs/geocoding/reverse-geocoding/)
//! (`GET https://api.geoapify.com/v1/geocode/reverse`).
//!
//! - [`ReverseGeocodingQuery`] holds the query parameters. Convert it with
//!   [`CanExportQuery::to_query`].
//! - [`ReverseGeocodingResponse`] is the response body: a list of
//!   [`GeocodingResult`](super::geocoding::GeocodingResult)s, plus the
//!   request as Geoapify understood it in [`ReverseGeocodingQueryEcho`].

use serde::{Deserialize, Serialize};

use super::geocoding::GeocodingResponse;
use crate::traits::query::CanExportQuery;

/// The full JSON response of the Reverse Geocoding API.
pub type ReverseGeocodingResponse = GeocodingResponse<ReverseGeocodingQueryEcho>;

/// The query parameters for the Reverse Geocoding API.
///
/// Build it with [`new`](Self::new) and the `with_*` methods. The API key is
/// not part of the query; the session adds it.
///
/// `format=json` is always sent, since
/// [`ReverseGeocodingResponse`] only models the JSON format.
///
/// Values are not validated here. Geoapify responds with HTTP 400 if one is
/// out of range.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReverseGeocodingQuery {
    /// Latitude in decimal degrees, from -90 to 90.
    pub lat: f64,
    /// Longitude in decimal degrees, from -180 to 180.
    pub lon: f64,
    /// Only return results of this type. If not set, Geoapify returns the
    /// most specific result it finds.
    #[serde(rename = "type")]
    pub location_type: Option<LocationType>,
    /// The language of the results, as an ISO 639-1 code, e.g. `"en"`.
    /// Geoapify defaults to `"en"`.
    pub lang: Option<String>,
    /// The maximum number of results, from 1 to 100. Geoapify defaults to 1.
    pub limit: Option<u8>,
    /// Only return results in these countries, as lowercase ISO 3166-1
    /// alpha-2 codes, e.g. `"de"`. At most 10.
    ///
    /// Sent as one `countrycodes` parameter per code. Geoapify does not
    /// document how to send several codes, and its OpenAPI spec implies this
    /// form.
    #[serde(rename = "countrycodes")]
    pub country_codes: Vec<String>,
    /// Always [`ResponseFormat::Json`]. Private so that it cannot be changed.
    format: ResponseFormat,
}

impl CanExportQuery for ReverseGeocodingQuery {}

impl ReverseGeocodingQuery {
    /// Creates a query for the place nearest to the given coordinates.
    pub fn new(lat: f64, lon: f64) -> Self {
        Self {
            lat,
            lon,
            location_type: None,
            lang: None,
            limit: None,
            country_codes: Vec::new(),
            format: ResponseFormat::Json,
        }
    }

    /// Only return results of this type.
    pub fn with_type(mut self, location_type: LocationType) -> Self {
        self.location_type = Some(location_type);
        self
    }

    /// Set the language of the results, as an ISO 639-1 code, e.g. `"en"`.
    pub fn with_lang(mut self, lang: impl Into<String>) -> Self {
        self.lang = Some(lang.into());
        self
    }

    /// Set the maximum number of results, from 1 to 100.
    pub fn with_limit(mut self, limit: u8) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Only return results in these countries, as lowercase ISO 3166-1
    /// alpha-2 codes, e.g. `"de"`. At most 10.
    pub fn with_country_codes(
        mut self,
        country_codes: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.country_codes = country_codes.into_iter().map(Into::into).collect();
        self
    }
}

/// The types of place the Reverse Geocoding API can be limited to, with
/// [`ReverseGeocodingQuery::location_type`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocationType {
    Country,
    State,
    City,
    Postcode,
    Street,
    Amenity,
    /// Only documented in Geoapify's OpenAPI spec.
    Locality,
    /// Only documented in Geoapify's OpenAPI spec.
    Building,
}

/// The `format` query parameter. Only JSON is supported by this crate.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ResponseFormat {
    #[default]
    Json,
}

/// The `query` object of a [`ReverseGeocodingResponse`]: the request as
/// Geoapify understood it.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "snake_case", serialize = "camelCase"))]
pub struct ReverseGeocodingQueryEcho {
    /// The requested latitude.
    pub lat: f64,
    /// The requested longitude.
    pub lon: f64,
    /// The [Open Location Code](https://maps.google.com/pluscodes/) of the
    /// requested coordinates, e.g. `"9F3PV2F5+W7"`.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plus_code: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::super::geocoding::{MatchType, ResultType};
    use super::*;

    fn pairs(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn minimal_query() {
        let query = ReverseGeocodingQuery::new(51.87485003734697, 14.008155801332068);

        assert_eq!(
            query.to_query().unwrap(),
            pairs(&[
                ("lat", "51.87485003734697"),
                ("lon", "14.008155801332068"),
                ("format", "json"),
            ])
        );
    }

    #[test]
    fn full_query() {
        let query = ReverseGeocodingQuery::new(51.5, -0.1)
            .with_type(LocationType::Postcode)
            .with_lang("de")
            .with_limit(5)
            .with_country_codes(["gb", "ie"]);

        assert_eq!(
            query.to_query().unwrap(),
            pairs(&[
                ("lat", "51.5"),
                ("lon", "-0.1"),
                ("type", "postcode"),
                ("lang", "de"),
                ("limit", "5"),
                ("countrycodes", "gb"),
                ("countrycodes", "ie"),
                ("format", "json"),
            ])
        );
    }

    /// A real response from the API with `format=json`.
    #[test]
    fn deserializes_sample_response() {
        let json = r#"{
            "results": [{
                "datasource": {
                    "sourcename": "openstreetmap",
                    "attribution": "© OpenStreetMap contributors",
                    "license": "Open Database License",
                    "url": "https://www.openstreetmap.org/copyright"
                },
                "name": "Wotschofska",
                "country": "Germany",
                "country_code": "de",
                "state": "Brandenburg",
                "county": "Oberspreewald-Lausitz - Górne Błota-Łužyca",
                "city": "Lübbenau/Spreewald",
                "village": "Lehde",
                "postcode": "03222",
                "street": "Leiper Weg",
                "housenumber": "1",
                "iso3166_2": "DE-BB",
                "lon": 14.0089232,
                "lat": 51.8744384,
                "state_code": "BB",
                "county_code": "OL",
                "district": "Lübbenau/Spreewald",
                "distance": 69.7887571803706,
                "result_type": "amenity",
                "NUTS_3": "DE40B",
                "formatted": "Wotschofska, Leiper Weg 1, 03222 Lübbenau/Spreewald, Germany",
                "address_line1": "Wotschofska",
                "address_line2": "Leiper Weg 1, 03222 Lübbenau/Spreewald, Germany",
                "category": "catering.restaurant",
                "timezone": {
                    "name": "Europe/Berlin",
                    "offset_STD": "+01:00",
                    "offset_DST": "+02:00",
                    "offset_STD_seconds": 3600,
                    "offset_DST_seconds": 7200,
                    "abbreviation_STD": "CET",
                    "abbreviation_DST": "CEST"
                },
                "plus_code": "9F3PV2F5+QH",
                "rank": {"importance": 5.343633173109052e-05, "popularity": 5.08886881736031},
                "place_id": "51f159e89491042c4059ec2ef598edef4940f00103f901103efc1100000000c0020192030b576f747363686f66736b61e203226f70656e7374726565746d61703a76656e75653a6e6f64652f333031373433363332",
                "bbox": {"lon1": 14.0088732, "lat1": 51.8743884, "lon2": 14.0089732, "lat2": 51.8744884}
            }],
            "query": {"lat": 51.87485003734697, "lon": 14.008155801332068, "plus_code": "9F3PV2F5+W7"}
        }"#;

        let response: ReverseGeocodingResponse = serde_json::from_str(json).unwrap();

        assert_eq!(
            response.query,
            Some(ReverseGeocodingQueryEcho {
                lat: 51.87485003734697,
                lon: 14.008155801332068,
                plus_code: Some("9F3PV2F5+W7".to_owned()),
            })
        );

        let [result] = response.results.as_slice() else {
            panic!("expected one result, got {:?}", response.results);
        };
        assert_eq!(result.name.as_deref(), Some("Wotschofska"));
        assert_eq!(result.country_code.as_deref(), Some("de"));
        assert_eq!(result.village.as_deref(), Some("Lehde"));
        assert_eq!(result.housenumber.as_deref(), Some("1"));
        assert_eq!(result.iso3166_2.as_deref(), Some("DE-BB"));
        assert_eq!(result.nuts_3.as_deref(), Some("DE40B"));
        assert_eq!(result.result_type, Some(ResultType::Amenity));
        assert_eq!(result.category.as_deref(), Some("catering.restaurant"));
        assert_eq!((result.lat, result.lon), (51.8744384, 14.0089232));
        assert_eq!(result.distance, Some(69.7887571803706));

        let timezone = result.timezone.as_ref().unwrap();
        assert_eq!(timezone.name, "Europe/Berlin");
        assert_eq!(timezone.offset_std.as_deref(), Some("+01:00"));
        assert_eq!(timezone.offset_dst_seconds, 7200);
        assert_eq!(timezone.abbreviation_dst.as_deref(), Some("CEST"));

        let rank = result.rank.as_ref().unwrap();
        assert_eq!(rank.popularity, Some(5.08886881736031));
        assert_eq!(rank.match_type, None::<MatchType>);

        let bbox = result.bbox.unwrap();
        assert_eq!((bbox.lon1, bbox.lat2), (14.0088732, 51.8744884));

        assert_eq!(
            result.datasource.as_ref().unwrap().sourcename.as_deref(),
            Some("openstreetmap")
        );
        // Every field in the sample is modelled.
        assert!(result.other.is_empty(), "unexpected: {:?}", result.other);
    }

    #[test]
    fn deserializes_empty_response() {
        let json = r#"{"results": [], "query": {"lat": 0.0, "lon": 0.0}}"#;
        let response: ReverseGeocodingResponse = serde_json::from_str(json).unwrap();

        assert!(response.results.is_empty());
        assert_eq!(response.query.unwrap().plus_code, None);
    }
}
