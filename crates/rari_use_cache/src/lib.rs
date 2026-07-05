#[macro_use]
extern crate napi_derive;

pub mod closure;
pub mod directive;
pub mod hoist;
pub mod id;
pub mod transform;

use napi::bindgen_prelude::*;

#[non_exhaustive]
#[napi(object)]
pub struct TransformOptions {
    pub filename: String,
    pub hash_salt: Option<String>,
}

#[non_exhaustive]
#[napi(object)]
pub struct TransformResult {
    pub code: String,
    pub needs_react_cache: bool,
    pub needs_cache_wrapper: bool,
    pub needs_register_ref: bool,
}

#[napi]
#[allow(
    clippy::allow_attributes,
    clippy::needless_pass_by_value,
    reason = "NAPI macro interface requires this pattern"
)]
pub fn detect_use_cache(source: String) -> bool {
    directive::detect_use_cache(&source)
}

/// Transforms source code with use cache directives.
///
/// # Errors
///
/// Returns an error if transformation fails due to parsing or code generation errors.
#[napi]
#[allow(
    clippy::allow_attributes,
    clippy::needless_pass_by_value,
    reason = "NAPI macro interface requires this pattern"
)]
pub fn transform_use_cache(source: String, options: TransformOptions) -> Result<TransformResult> {
    let hash_salt = options.hash_salt.unwrap_or_else(|| "rari-use-cache-v1".to_string());

    let result = transform::transform_source(&source, &options.filename, &hash_salt)
        .map_err(Error::from_reason)?;

    Ok(TransformResult {
        code: result.code,
        needs_react_cache: result.needs_react_cache,
        needs_cache_wrapper: result.needs_cache_wrapper,
        needs_register_ref: result.needs_register_ref,
    })
}
