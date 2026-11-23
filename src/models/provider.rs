//! A trait representing a generic API provider.
//!
//! Mostly for typing.

use super::rate_limit::RateLimit;

/// Represents a generic API provider.
pub trait Provider {
    /// Return the name of the provider.
    fn name<'s>() -> &'s str;

    /// Return the rate limit information for the provider.
    fn rate_limit() -> Option<RateLimit>;
}

impl RateLimit {
    /// Load the default rate limit for the given provider.
    pub fn default_for<P: Provider>() -> Option<RateLimit> {
        P::rate_limit()
    }
}
