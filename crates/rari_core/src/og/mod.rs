//! OG (Open Graph) image generation types and cache infrastructure.
//!
//! The types module defines data structures shared across the OG image pipeline.
//! The cache module provides a two-tier (memory + disk) cache for generated OG images.

pub mod cache;
pub mod types;

pub use cache::OgImageCache;
pub use types::{JsxChild, JsxElement, OgImageEntry, OgImageParams, OgImageResult};
