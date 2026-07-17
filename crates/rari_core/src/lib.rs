//! Protocol-agnostic runtime infrastructure shared across rari backends.
//!
//! This crate owns the pieces that are reusable regardless of the framework
//! protocol layered on top: HTTP server scaffolding, the embedded V8 runtime,
//! module loading, byte-level cache handlers, image/OG optimization, and
//! supporting utilities. Backend-specific concerns (routing conventions,
//! rendering/composition, server-action protocols) live in the backend crates.
pub mod utils;
pub mod compression;
