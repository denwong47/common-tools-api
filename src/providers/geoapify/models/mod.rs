//! The data models for interacting with Geoapify services.

pub mod geocoding;
pub mod reverse_geocoding;

pub use geocoding::{GeocodingResponse, GeocodingResult};
pub use reverse_geocoding::{ReverseGeocodingQuery, ReverseGeocodingResponse};
