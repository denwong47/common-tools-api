//! The data models for interacting with Geoapify services.

pub mod forward_geocoding;
pub mod geocoding;
pub mod reverse_geocoding;

pub use forward_geocoding::{ForwardGeocodingQuery, ForwardGeocodingResponse};
pub use geocoding::{GeocodingResponse, GeocodingResult};
pub use reverse_geocoding::{ReverseGeocodingQuery, ReverseGeocodingResponse};
