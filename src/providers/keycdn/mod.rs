//! KeyCDN API provider module.
//!
//! This module contains the data models and functionality specific to the KeyCDN API.

pub mod api;
pub mod session;

// Export commonly used types for convenience.
pub use api::location_lookup::LocationLookupResult;
pub use session::Session;
