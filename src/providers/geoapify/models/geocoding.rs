//! Models shared by the Geoapify geocoding APIs.
//!
//! The [Reverse Geocoding API](https://apidocs.geoapify.com/docs/geocoding/reverse-geocoding/),
//! the [Forward Geocoding API](https://apidocs.geoapify.com/docs/geocoding/forward-geocoding/)
//! and the Address Autocomplete API all return the same result object,
//! modelled here as [`GeocodingResult`]. Only the echoed `query` object
//! differs between them, so [`GeocodingResponse`] is generic over it.
//!
//! These models are for `format=json` responses only. The default format,
//! `geojson`, wraps each result in a GeoJSON `Feature` and returns `bbox` as an
//! array instead of an object, so it will not deserialize into these types.
//!
//! Like the KeyCDN models, these types deserialize from the API's field names
//! and serialize to `camelCase`, leaving out `None` fields.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The full JSON response of a Geoapify geocoding API, with `format=json`.
///
/// `Q` is the echoed request: see
/// [`ReverseGeocodingQueryEcho`](super::reverse_geocoding::ReverseGeocodingQueryEcho)
/// for the Reverse Geocoding API.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeocodingResponse<Q> {
    /// The matching places, best match first. Empty if nothing was found.
    pub results: Vec<GeocodingResult>,
    /// The request as Geoapify understood it. Optional in Geoapify's schema
    /// for `format=json`.
    pub query: Option<Q>,
}

/// A single place returned by a Geoapify geocoding API.
///
/// Geoapify's schema marks every field as optional, and which address
/// components are present depends on the place and country. Only `lat` and
/// `lon` are required here, since a result without coordinates is not usable.
///
/// The address components that are common or useful worldwide have their own
/// fields. Geoapify documents around 40 more (e.g. `province`, `arrondissement`,
/// `city_block`, `postal_code`); those, and any fields Geoapify adds in future,
/// are kept as-is in [`other`](Self::other).
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "snake_case", serialize = "camelCase"))]
pub struct GeocodingResult {
    /// Geoapify's identifier for the place, usable with the Place Details API.
    pub place_id: Option<String>,
    /// The name of the place, e.g. `"Wotschofska"`. Usually only present for
    /// named places such as amenities.
    pub name: Option<String>,
    /// The full address on one line, e.g.
    /// `"Wotschofska, Leiper Weg 1, 03222 Lübbenau/Spreewald, Germany"`.
    pub formatted: Option<String>,
    /// The first line of the address, e.g. the name or the street and house
    /// number.
    pub address_line1: Option<String>,
    /// The rest of the address after [`address_line1`](Self::address_line1).
    pub address_line2: Option<String>,

    /// Latitude of the place in decimal degrees.
    pub lat: f64,
    /// Longitude of the place in decimal degrees.
    pub lon: f64,
    /// For reverse geocoding, the distance in metres from the requested
    /// coordinates to the place.
    pub distance: Option<f64>,
    /// The bounding box of the place.
    pub bbox: Option<BoundingBox>,
    /// The [Open Location Code](https://maps.google.com/pluscodes/) of the
    /// place, e.g. `"9F3PV2F5+QH"`.
    pub plus_code: Option<String>,
    /// A shortened plus code relative to a nearby locality, e.g.
    /// `"8W+4W, 02109 Boston, United States"`.
    pub plus_code_short: Option<String>,

    /// What kind of place this result is.
    pub result_type: Option<ResultType>,
    /// The place's main category from the
    /// [Places API categories](https://apidocs.geoapify.com/docs/places/#categories),
    /// e.g. `"catering.restaurant"`.
    pub category: Option<String>,
    /// All the place's categories from the Places API categories.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<String>,

    // Address components, broadest first.
    /// e.g. `"Europe"`.
    pub continent: Option<String>,
    /// e.g. `"Germany"`.
    pub country: Option<String>,
    /// ISO 3166-1 alpha-2 code in lowercase, e.g. `"de"`.
    pub country_code: Option<String>,
    /// A region above state level, used in some countries.
    pub region: Option<String>,
    /// e.g. `"Brandenburg"`.
    pub state: Option<String>,
    /// e.g. `"BB"`. Missing for many states.
    pub state_code: Option<String>,
    /// A division between state and county, used in some countries.
    pub state_district: Option<String>,
    /// e.g. `"Oberspreewald-Lausitz - Górne Błota-Łužyca"`.
    pub county: Option<String>,
    /// e.g. `"OL"`.
    pub county_code: Option<String>,
    /// e.g. `"Lübbenau/Spreewald"`.
    pub city: Option<String>,
    pub city_district: Option<String>,
    pub district: Option<String>,
    pub suburb: Option<String>,
    pub quarter: Option<String>,
    pub neighbourhood: Option<String>,
    pub municipality: Option<String>,
    pub town: Option<String>,
    /// e.g. `"Lehde"`.
    pub village: Option<String>,
    pub hamlet: Option<String>,
    /// e.g. `"03222"`.
    pub postcode: Option<String>,
    /// e.g. `"Leiper Weg"`.
    pub street: Option<String>,
    /// e.g. `"1"`. A string, since house numbers can be e.g. `"12a"`.
    pub housenumber: Option<String>,

    /// ISO 3166-2 subdivision code, e.g. `"DE-BB"`.
    #[serde(rename(serialize = "iso3166_2"))]
    pub iso3166_2: Option<String>,
    /// The EU's NUTS 3 region code, e.g. `"DE40B"`.
    #[serde(rename(deserialize = "NUTS_3", serialize = "nuts3"))]
    pub nuts_3: Option<String>,
    /// The place's name in other languages or forms, keyed by Geoapify's name
    /// tag, e.g. `"name:en"`.
    pub other_names: Option<BTreeMap<String, String>>,
    /// The population, for settlements.
    pub population: Option<f64>,

    /// The time zone at the place.
    pub timezone: Option<Timezone>,
    /// How well this result matches the request.
    pub rank: Option<Rank>,
    /// Where the data came from, and its licence.
    pub datasource: Option<Datasource>,

    /// Any fields not modelled above, as returned by the API.
    ///
    /// The keys are Geoapify's names, so they are not converted to
    /// `camelCase` when serializing.
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub other: BTreeMap<String, serde_json::Value>,
}

/// What kind of place a [`GeocodingResult`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultType {
    Amenity,
    /// Only documented in Geoapify's OpenAPI spec.
    AmenityName,
    Building,
    Street,
    Suburb,
    District,
    Postcode,
    City,
    County,
    State,
    Country,
    /// Geoapify returned `"unknown"`, or a value this crate does not know.
    #[serde(other)]
    Unknown,
}

/// A bounding box in decimal degrees.
///
/// Geoapify does not document which corner is which. In practice
/// `(lon1, lat1)` is the south-west corner and `(lon2, lat2)` the north-east.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    pub lon1: f64,
    pub lat1: f64,
    pub lon2: f64,
    pub lat2: f64,
}

/// The time zone of a [`GeocodingResult`].
///
/// The offsets are given both as strings (e.g. `"+01:00"`) and in seconds.
/// For zones without daylight saving time, the `DST` values equal the `STD`
/// ones.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "snake_case", serialize = "camelCase"))]
pub struct Timezone {
    /// IANA time zone name, e.g. `"Europe/Berlin"`.
    pub name: String,
    /// An alternative IANA name for the same zone.
    pub name_alt: Option<String>,
    /// ISO 3166-1 alpha-2 codes of the countries using this zone.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub country_codes: Vec<String>,
    /// Standard time UTC offset, e.g. `"+01:00"`.
    #[serde(rename(deserialize = "offset_STD"))]
    pub offset_std: Option<String>,
    /// Standard time UTC offset in seconds, e.g. `3600`.
    #[serde(rename(deserialize = "offset_STD_seconds"))]
    pub offset_std_seconds: i32,
    /// Daylight saving time UTC offset, e.g. `"+02:00"`.
    #[serde(rename(deserialize = "offset_DST"))]
    pub offset_dst: Option<String>,
    /// Daylight saving time UTC offset in seconds, e.g. `7200`.
    #[serde(rename(deserialize = "offset_DST_seconds"))]
    pub offset_dst_seconds: i32,
    /// Standard time abbreviation, e.g. `"CET"`. Only present if one exists.
    #[serde(rename(deserialize = "abbreviation_STD"))]
    pub abbreviation_std: Option<String>,
    /// Daylight saving time abbreviation, e.g. `"CEST"`. Only present if one
    /// exists.
    #[serde(rename(deserialize = "abbreviation_DST"))]
    pub abbreviation_dst: Option<String>,
}

/// How well a [`GeocodingResult`] matches the request.
///
/// Reverse geocoding results usually only have `importance` and
/// `popularity`. The `confidence*` fields and `match_type` are mainly
/// returned by forward geocoding.
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "snake_case", serialize = "camelCase"))]
pub struct Rank {
    /// Undocumented by Geoapify. Appears to be the OpenStreetMap/Nominatim
    /// importance of the place.
    pub importance: Option<f64>,
    /// Undocumented by Geoapify. Appears to reflect how many points of
    /// interest are around the place.
    pub popularity: Option<f64>,
    /// Overall confidence, from 0 to 1.
    pub confidence: Option<f64>,
    /// Confidence that the city is correct, from 0 to 1.
    pub confidence_city_level: Option<f64>,
    /// Confidence that the street is correct, from 0 to 1.
    pub confidence_street_level: Option<f64>,
    /// Confidence that the building position is correct, from 0 to 1.
    pub confidence_building_level: Option<f64>,
    /// Which address level the result matched at.
    pub match_type: Option<MatchType>,
}

/// The address level a [`GeocodingResult`] matched the request at.
///
/// This says whether the result is at the requested level, not whether every
/// address component is right; use [`Rank::confidence`] for that.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchType {
    FullMatch,
    InnerPart,
    MatchByBuilding,
    MatchByStreet,
    MatchByPostcode,
    /// Geoapify spells this `"match_by_city_or_disrict"`. The corrected
    /// spelling is also accepted in case they fix it.
    #[serde(
        rename = "match_by_city_or_disrict",
        alias = "match_by_city_or_district"
    )]
    MatchByCityOrDistrict,
    MatchByCountryOrState,
    /// A value this crate does not know.
    #[serde(other)]
    Unknown,
}

/// The source of a [`GeocodingResult`]'s data, and its licence.
///
/// If you show the data, the licence may require you to show
/// [`attribution`](Self::attribution).
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "snake_case", serialize = "camelCase"))]
pub struct Datasource {
    /// e.g. `"openstreetmap"`.
    pub sourcename: Option<String>,
    /// e.g. `"© OpenStreetMap contributors"`.
    pub attribution: Option<String>,
    /// e.g. `"Open Database License"`.
    pub license: Option<String>,
    /// A link to the licence or copyright page, e.g.
    /// `"https://www.openstreetmap.org/copyright"`.
    pub url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_type_unknown_values() {
        let parse = |s: &str| serde_json::from_str::<ResultType>(s).unwrap();
        assert_eq!(parse(r#""amenity""#), ResultType::Amenity);
        assert_eq!(parse(r#""amenity_name""#), ResultType::AmenityName);
        assert_eq!(parse(r#""unknown""#), ResultType::Unknown);
        assert_eq!(parse(r#""something_new""#), ResultType::Unknown);
    }

    #[test]
    fn match_type_accepts_both_spellings() {
        let parse = |s: &str| serde_json::from_str::<MatchType>(s).unwrap();
        assert_eq!(
            parse(r#""match_by_city_or_disrict""#),
            MatchType::MatchByCityOrDistrict
        );
        assert_eq!(
            parse(r#""match_by_city_or_district""#),
            MatchType::MatchByCityOrDistrict
        );
        assert_eq!(parse(r#""something_new""#), MatchType::Unknown);
    }

    #[test]
    fn unmodelled_fields_are_kept_in_other() {
        let json = r#"{"lat": 1.0, "lon": 2.0, "province": "Somewhere", "postcodes": ["1", "2"]}"#;
        let result: GeocodingResult = serde_json::from_str(json).unwrap();

        assert_eq!(result.other["province"], "Somewhere");
        assert_eq!(result.other["postcodes"], serde_json::json!(["1", "2"]));
        assert!(!result.other.contains_key("lat"));
    }

    #[test]
    fn serializes_to_camel_case_without_nones() {
        let json = r#"{"lat": 1.0, "lon": 2.0, "address_line1": "x", "NUTS_3": "DE40B", "iso3166_2": "DE-BB"}"#;
        let result: GeocodingResult = serde_json::from_str(json).unwrap();

        assert_eq!(
            serde_json::to_value(&result).unwrap(),
            serde_json::json!({
                "lat": 1.0,
                "lon": 2.0,
                "addressLine1": "x",
                "nuts3": "DE40B",
                "iso3166_2": "DE-BB",
            })
        );
    }
}
