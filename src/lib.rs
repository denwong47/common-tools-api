//! Small async clients for third-party web APIs, one module per provider.
//!
//! Each provider is behind its own Cargo feature, and lives in
//! [`providers`]:
//!
//! - `provider-keycdn`: `providers::keycdn`, for KeyCDN's
//!   [IP Location Finder API](https://tools.keycdn.com/geo#ip-location-finder-api).
//! - `provider-geoapify`: `providers::geoapify`, for Geoapify's
//!   [Reverse Geocoding API](https://apidocs.geoapify.com/docs/geocoding/reverse-geocoding/).
//!
//! Every provider has a `Session`, which rate limits its own requests to stay
//! within that provider's limits. Shared types are in [`models`] and
//! [`traits`], and every function returns this crate's [`Result`].

pub mod error;
pub mod models;
pub mod providers;
pub mod traits;

pub use error::{Error, Result};

#[cfg(test)]
#[allow(unused)]
pub(crate) mod logging {
    pub use tracing_subscriber;
    use tracing_subscriber::EnvFilter;

    static INIT_LOG: std::sync::Once = std::sync::Once::new();

    const DEFAULT_LOG_LEVEL: &str = "debug";

    /// Placeholder for crates that are noisy and should be set to a higher log
    /// level than the default.
    pub const NOISY_CRATE_OVERRIDES: [&str; 0] = [];

    fn build_env_filter(default_level: &str) -> EnvFilter {
        let base = std::env::var("RUST_LOG").unwrap_or_else(|_| default_level.to_owned());
        EnvFilter::new(format!("{base},{}", NOISY_CRATE_OVERRIDES.join(",")))
    }

    pub fn init_test_tracing() {
        INIT_LOG.call_once(|| {
            tracing_subscriber::fmt()
                .with_env_filter(build_env_filter("trace"))
                .with_target(true)
                .with_level(true)
                .with_ansi(true)
                .with_writer(std::io::stderr)
                .try_init()
                .ok();
        });
    }
}
