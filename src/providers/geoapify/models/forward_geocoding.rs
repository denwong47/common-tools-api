//! Models for the
//! [Forward Geocoding API](https://apidocs.geoapify.com/docs/geocoding/forward-geocoding/)
//! (`GET https://api.geoapify.com/v1/geocode/search`).
//!
//! - [`ForwardGeocodingQuery`] holds the query parameters. Convert it with
//!   [`CanExportQuery::to_query`].
//! - [`ForwardGeocodingResponse`] is the response body: a list of
//!   [`GeocodingResult`](super::geocoding::GeocodingResult)s, plus the
//!   request as Geoapify understood it in [`ForwardGeocodingQueryEcho`].

use serde::{Deserialize, Serialize, Serializer};
use std::fmt;

use super::geocoding::{BoundingBox, Coordinates, GeocodingResponse, ResponseFormat, ResultType};
use crate::traits::query::CanExportQuery;

/// The full JSON response of the Forward Geocoding API.
pub type ForwardGeocodingResponse = GeocodingResponse<ForwardGeocodingQueryEcho>;

/// The query parameters for the Forward Geocoding API.
///
/// Build it with [`text`](Self::text) or [`structured`](Self::structured),
/// then the `with_*` methods. The API key is not part of the query; the
/// session adds it.
///
/// `format=json` is always sent, since [`ForwardGeocodingResponse`] only
/// models the JSON format.
///
/// Values are not validated here. Geoapify responds with HTTP 400 if one is
/// out of range.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ForwardGeocodingQuery {
    /// The address to search for.
    #[serde(flatten)]
    pub address: AddressQuery,
    /// Only return results of this type.
    #[serde(rename = "type")]
    pub location_type: Option<LocationType>,
    /// The language of the results, as an ISO 639-1 code, e.g. `"en"`.
    /// Geoapify defaults to `"en"`.
    pub lang: Option<String>,
    /// The maximum number of results, from 1 to 100. Geoapify defaults to 5.
    pub limit: Option<u8>,
    /// Only return results that match every one of these filters.
    #[serde(skip_serializing_if = "Filter::is_empty")]
    pub filter: Filter,
    /// Rank results that match any of these first.
    #[serde(skip_serializing_if = "Bias::is_empty")]
    pub bias: Bias,
    /// Always [`ResponseFormat::Json`]. Private so that it cannot be changed.
    format: ResponseFormat,
}

impl CanExportQuery for ForwardGeocodingQuery {}

impl ForwardGeocodingQuery {
    /// Creates a query that searches for a free-form address, e.g.
    /// `"Leiper Weg 1, 03222 Lübbenau/Spreewald, Germany"`.
    pub fn text(text: impl Into<String>) -> Self {
        Self::new(AddressQuery::Text { text: text.into() })
    }

    /// Creates a query that searches for an address split into its parts.
    ///
    /// Set at least one field of `address`.
    pub fn structured(address: StructuredAddress) -> Self {
        Self::new(AddressQuery::Structured(address))
    }

    fn new(address: AddressQuery) -> Self {
        Self {
            address,
            location_type: None,
            lang: None,
            limit: None,
            filter: Filter::default(),
            bias: Bias::default(),
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

    /// Only return results that match every filter in `filter`.
    pub fn with_filter(mut self, filter: Filter) -> Self {
        self.filter = filter;
        self
    }

    /// Rank results that match any bias in `bias` first.
    pub fn with_bias(mut self, bias: Bias) -> Self {
        self.bias = bias;
        self
    }
}

/// The address to search for: either free-form text or structured parts.
///
/// Geoapify does not allow mixing the two.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum AddressQuery {
    /// A free-form address, sent as `text`. At most 1024 characters.
    Text { text: String },
    /// An address split into its parts.
    Structured(StructuredAddress),
}

/// An address split into its parts, for [`ForwardGeocodingQuery::structured`].
///
/// Set the fields you know and leave the rest as `None`; each is at most 1024
/// characters.
///
/// ```
/// # use common_tools_api::providers::geoapify::models::forward_geocoding::StructuredAddress;
/// let address = StructuredAddress {
///     street: Some("Leiper Weg".to_owned()),
///     city: Some("Lübbenau".to_owned()),
///     country: Some("Germany".to_owned()),
///     ..Default::default()
/// };
/// ```
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct StructuredAddress {
    /// The name of a place or amenity.
    pub name: Option<String>,
    pub housenumber: Option<String>,
    pub street: Option<String>,
    /// Postcode or ZIP code.
    pub postcode: Option<String>,
    pub city: Option<String>,
    /// State or region.
    pub state: Option<String>,
    pub country: Option<String>,
}

/// The types of place the Forward Geocoding API can be limited to, with
/// [`ForwardGeocodingQuery::location_type`].
///
/// Unlike reverse geocoding, there is no `building` type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocationType {
    Country,
    State,
    City,
    Postcode,
    Street,
    Amenity,
    /// Any administrative area: postcodes, districts, cities, counties and
    /// states.
    Locality,
}

/// A circle on the map, for [`Filter`] and [`Bias`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Circle {
    /// The centre's longitude.
    pub lon: f64,
    /// The centre's latitude.
    pub lat: f64,
    /// The radius in metres.
    pub radius_meters: f64,
}

/// A country restriction, for [`Filter`] and [`Bias`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CountryCodes {
    /// These countries, as lowercase ISO 3166-1 alpha-2 codes, e.g. `"de"`.
    List(Vec<String>),
    /// The country Geoapify detects from the caller's IP address. For a
    /// server-side client, that is the server's country.
    Auto,
    /// No country restriction (`countrycode:none`).
    Disabled,
}

impl fmt::Display for CountryCodes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("countrycode:")?;
        match self {
            Self::List(codes) => f.write_str(&codes.join(",")),
            Self::Auto => f.write_str("auto"),
            Self::Disabled => f.write_str("none"),
        }
    }
}

impl fmt::Display for Circle {
    /// Formats as Geoapify's `circle:lon,lat,radiusMeters`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "circle:{},{},{}", self.lon, self.lat, self.radius_meters)
    }
}

/// Formats as Geoapify's `rect:lon1,lat1,lon2,lat2`.
fn rect_to_string(rect: &BoundingBox) -> String {
    format!(
        "rect:{},{},{},{}",
        rect.lon1, rect.lat1, rect.lon2, rect.lat2
    )
}

/// Hard filters for [`ForwardGeocodingQuery`]. Results must match all of the
/// filters that are set.
///
/// Geoapify accepts at most one filter of each kind, which is why each kind
/// is a single field. They are sent as one `filter` parameter, separated by
/// `|`, e.g. `filter=rect:...|countrycode:us`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Filter {
    /// Only results inside this circle.
    pub circle: Option<Circle>,
    /// Only results inside this rectangle.
    pub rect: Option<BoundingBox>,
    /// Only results in these countries.
    pub country_codes: Option<CountryCodes>,
    /// Only results inside this place, given by the
    /// [`place_id`](super::geocoding::GeocodingResult::place_id) of another
    /// result, e.g. a city.
    pub place: Option<String>,
}

impl Filter {
    /// Whether no filter is set.
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }

    pub fn with_circle(mut self, circle: Circle) -> Self {
        self.circle = Some(circle);
        self
    }

    pub fn with_rect(mut self, rect: BoundingBox) -> Self {
        self.rect = Some(rect);
        self
    }

    pub fn with_country_codes(mut self, country_codes: CountryCodes) -> Self {
        self.country_codes = Some(country_codes);
        self
    }

    pub fn with_place(mut self, place_id: impl Into<String>) -> Self {
        self.place = Some(place_id.into());
        self
    }
}

impl fmt::Display for Filter {
    /// Formats the filters that are set, separated by `|`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = [
            self.circle.as_ref().map(Circle::to_string),
            self.rect.as_ref().map(rect_to_string),
            self.country_codes.as_ref().map(CountryCodes::to_string),
            self.place
                .as_ref()
                .map(|place_id| format!("place:{place_id}")),
        ]
        .into_iter()
        .flatten()
        .collect();
        f.write_str(&parts.join("|"))
    }
}

impl Serialize for Filter {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// Ranking biases for [`ForwardGeocodingQuery`]. Results matching any of the
/// biases that are set are ranked first; other results can still be
/// returned.
///
/// Geoapify accepts at most one bias of each kind, which is why each kind is
/// a single field. They are sent as one `bias` parameter, separated by `|`.
///
/// If no country bias is set, Geoapify applies `countrycode:auto`, which
/// favours the country of the caller's IP address. For a server-side client,
/// that is the server's country; set
/// [`country_codes`](Self::country_codes) to [`CountryCodes::Disabled`] to
/// turn it off.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Bias {
    /// Rank results closer to this point first. Results'
    /// [`distance`](super::geocoding::GeocodingResult::distance) is measured
    /// from it.
    pub proximity: Option<Coordinates>,
    /// Rank results inside this circle first.
    pub circle: Option<Circle>,
    /// Rank results inside this rectangle first.
    pub rect: Option<BoundingBox>,
    /// Rank results in these countries first.
    pub country_codes: Option<CountryCodes>,
}

impl Bias {
    /// Whether no bias is set.
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }

    pub fn with_proximity(mut self, proximity: Coordinates) -> Self {
        self.proximity = Some(proximity);
        self
    }

    pub fn with_circle(mut self, circle: Circle) -> Self {
        self.circle = Some(circle);
        self
    }

    pub fn with_rect(mut self, rect: BoundingBox) -> Self {
        self.rect = Some(rect);
        self
    }

    pub fn with_country_codes(mut self, country_codes: CountryCodes) -> Self {
        self.country_codes = Some(country_codes);
        self
    }
}

impl fmt::Display for Bias {
    /// Formats the biases that are set, separated by `|`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = [
            self.proximity
                .map(|point| format!("proximity:{},{}", point.lon, point.lat)),
            self.circle.as_ref().map(Circle::to_string),
            self.rect.as_ref().map(rect_to_string),
            self.country_codes.as_ref().map(CountryCodes::to_string),
        ]
        .into_iter()
        .flatten()
        .collect();
        f.write_str(&parts.join("|"))
    }
}

impl Serialize for Bias {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// The `query` object of a [`ForwardGeocodingResponse`]: the request as
/// Geoapify understood it.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "snake_case", serialize = "camelCase"))]
pub struct ForwardGeocodingQueryEcho {
    /// The free-form address, if one was sent.
    pub text: Option<String>,
    /// The structured address parts that were sent.
    pub housenumber: Option<String>,
    pub street: Option<String>,
    pub postcode: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
    /// Coordinates, if the text was recognised as a coordinate pair.
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    /// A plus code, if the text was recognised as one.
    pub plus_code: Option<String>,
    /// How Geoapify split the free-form text into address parts.
    pub parsed: Option<ParsedAddress>,
    /// Place categories Geoapify recognised in the text.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<QueryCategory>,
}

/// How Geoapify parsed a free-form address. Values are lowercased, e.g.
/// `"larkin street"`.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "snake_case", serialize = "camelCase"))]
pub struct ParsedAddress {
    pub po_box: Option<String>,
    pub unit: Option<String>,
    pub house: Option<String>,
    pub housenumber: Option<String>,
    pub street: Option<String>,
    pub suburb: Option<String>,
    pub postcode: Option<String>,
    pub district: Option<String>,
    pub city: Option<String>,
    pub county: Option<String>,
    pub state: Option<String>,
    pub region: Option<String>,
    pub country: Option<String>,
    /// The kind of place the text seems to describe.
    pub expected_type: Option<ResultType>,
}

/// A place category Geoapify recognised in a free-form query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryCategory {
    /// Places API category keys, e.g. `"catering.restaurant"`.
    pub keys: Vec<String>,
    /// A human-readable label for the category.
    pub label: String,
}

#[cfg(test)]
mod tests {
    use super::super::geocoding::MatchType;
    use super::*;

    fn pairs(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn text_query() {
        let query = ForwardGeocodingQuery::text("Leiper Weg 1, Lübbenau");

        assert_eq!(
            query.to_query().unwrap(),
            pairs(&[("text", "Leiper Weg 1, Lübbenau"), ("format", "json")])
        );
    }

    #[test]
    fn structured_query_leaves_out_unset_parts() {
        let query = ForwardGeocodingQuery::structured(StructuredAddress {
            housenumber: Some("1".to_owned()),
            street: Some("Leiper Weg".to_owned()),
            country: Some("Germany".to_owned()),
            ..Default::default()
        })
        .with_type(LocationType::Street)
        .with_lang("de")
        .with_limit(10);

        assert_eq!(
            query.to_query().unwrap(),
            pairs(&[
                ("housenumber", "1"),
                ("street", "Leiper Weg"),
                ("country", "Germany"),
                ("type", "street"),
                ("lang", "de"),
                ("limit", "10"),
                ("format", "json"),
            ])
        );
    }

    #[test]
    fn filters_and_biases_are_pipe_joined() {
        let query = ForwardGeocodingQuery::text("Berlin")
            .with_filter(
                Filter::default()
                    .with_rect(BoundingBox {
                        lon1: -115.548725,
                        lat1: 20.337831,
                        lon2: -75.55849,
                        lat2: 44.866306,
                    })
                    .with_country_codes(CountryCodes::List(vec!["us".into(), "ca".into()])),
            )
            .with_bias(
                Bias::default()
                    .with_proximity(Coordinates {
                        lon: 13.4,
                        lat: 52.5,
                    })
                    .with_country_codes(CountryCodes::Disabled),
            );

        assert_eq!(
            query.to_query().unwrap(),
            pairs(&[
                ("text", "Berlin"),
                (
                    "filter",
                    "rect:-115.548725,20.337831,-75.55849,44.866306|countrycode:us,ca"
                ),
                ("bias", "proximity:13.4,52.5|countrycode:none"),
                ("format", "json"),
            ])
        );
    }

    #[test]
    fn every_filter_and_bias_kind() {
        let circle = Circle {
            lon: -87.770231,
            lat: 41.878968,
            radius_meters: 5000.0,
        };
        let rect = BoundingBox {
            lon1: 1.0,
            lat1: 2.0,
            lon2: 3.0,
            lat2: 4.0,
        };

        let filter = Filter::default()
            .with_circle(circle)
            .with_rect(rect)
            .with_country_codes(CountryCodes::Auto)
            .with_place("51f0");
        assert_eq!(
            filter.to_string(),
            "circle:-87.770231,41.878968,5000|rect:1,2,3,4|countrycode:auto|place:51f0"
        );

        let bias = Bias::default()
            .with_proximity(Coordinates { lon: 5.5, lat: 6.0 })
            .with_circle(circle)
            .with_rect(rect)
            .with_country_codes(CountryCodes::List(vec!["de".into()]));
        assert_eq!(
            bias.to_string(),
            "proximity:5.5,6|circle:-87.770231,41.878968,5000|rect:1,2,3,4|countrycode:de"
        );

        assert!(Filter::default().is_empty());
        assert!(Bias::default().is_empty());
        assert!(!filter.is_empty());
    }

    /// Modelled on the examples in Geoapify's documentation. The `query`
    /// object is copied from the OpenAPI spec's example.
    #[test]
    fn deserializes_documented_response() {
        let json = r#"{
            "results": [{
                "datasource": {"sourcename": "openstreetmap"},
                "name": "Asian Art Museum",
                "country": "United States",
                "country_code": "us",
                "state": "California",
                "city": "San Francisco",
                "postcode": "94102",
                "district": "Civic Center",
                "street": "Larkin Street",
                "housenumber": "200",
                "lon": -122.4161,
                "lat": 37.7802,
                "result_type": "amenity",
                "formatted": "Asian Art Museum, 200 Larkin Street, San Francisco, CA 94102, United States of America",
                "category": "entertainment.museum",
                "rank": {
                    "importance": 0.5,
                    "popularity": 8.9,
                    "confidence": 1,
                    "confidence_city_level": 1,
                    "confidence_street_level": 1,
                    "confidence_building_level": 1,
                    "match_type": "inner_part"
                },
                "place_id": "51abc"
            }],
            "query": {
                "text": "200 Larkin Street, San Francisco, CA 94102, United States",
                "parsed": {
                    "housenumber": "200",
                    "street": "larkin street",
                    "postcode": "94102",
                    "city": "san francisco",
                    "state": "ca",
                    "country": "united states",
                    "expected_type": "building"
                }
            }
        }"#;

        let response: ForwardGeocodingResponse = serde_json::from_str(json).unwrap();

        let [result] = response.results.as_slice() else {
            panic!("expected one result, got {:?}", response.results);
        };
        assert_eq!(result.housenumber.as_deref(), Some("200"));
        let rank = result.rank.as_ref().unwrap();
        assert_eq!(rank.confidence, Some(1.0));
        assert_eq!(rank.confidence_building_level, Some(1.0));
        assert_eq!(rank.match_type, Some(MatchType::InnerPart));

        let query = response.query.unwrap();
        assert_eq!(
            query.text.as_deref(),
            Some("200 Larkin Street, San Francisco, CA 94102, United States")
        );
        let parsed = query.parsed.unwrap();
        assert_eq!(parsed.street.as_deref(), Some("larkin street"));
        assert_eq!(parsed.expected_type, Some(ResultType::Building));
        assert!(query.categories.is_empty());
    }
}
