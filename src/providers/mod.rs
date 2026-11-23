//! Each API provider is represented by a module within this file, and
//! gated by their respective feature flags.

#[cfg(feature = "provider-keycdn")]
pub mod keycdn;

#[cfg(feature = "provider-geoapify")]
pub mod geoapify;
