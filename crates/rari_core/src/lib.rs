//! Protocol-agnostic runtime infrastructure shared across rari backends.
//!
//! This crate owns the pieces that are reusable regardless of the framework
//! protocol layered on top: HTTP server scaffolding, the embedded V8 runtime,
//! module loading, byte-level cache handlers, image/OG optimization, and
//! supporting utilities. Backend-specific concerns (routing conventions,
//! rendering/composition, server-action protocols) live in the backend crates.
pub mod action_state;
pub mod cache;
pub mod client;
pub mod component_registry;
pub mod compression;
pub mod config;
pub mod error_response;
pub mod http;
pub mod image;
pub mod metadata;
pub mod middleware;
pub mod og;
pub mod rendering;
pub mod request_context;
pub mod routing;
pub mod runtime;
pub mod sanitize;
pub mod state;
pub mod static_assets;
pub mod utils;
pub mod vite;
