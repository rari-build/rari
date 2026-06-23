#[macro_use]
extern crate napi_derive;

pub mod closure;
pub mod directive;
pub mod hoist;
pub mod id;
pub mod transform;

use napi::bindgen_prelude::*;

#[napi(object)]
pub struct TransformOptions {
    pub filename: String,
    pub hash_salt: Option<String>,
    pub cache_kinds: Option<Vec<String>>,
}

#[napi(object)]
pub struct TransformResult {
    pub code: String,
    pub needs_react_cache: bool,
    pub needs_cache_wrapper: bool,
    pub needs_register_ref: bool,
}

#[napi]
pub fn detect_use_cache(source: String) -> bool {
    directive::detect_use_cache(&source)
}

#[napi]
pub fn transform_use_cache(source: String, options: TransformOptions) -> Result<TransformResult> {
    let hash_salt = options
        .hash_salt
        .unwrap_or_else(|| "rari-use-cache-v1".to_string());
    let cache_kinds = options
        .cache_kinds
        .unwrap_or_else(|| vec!["default".to_string()]);

    let result = transform::transform_source(&source, &options.filename, &hash_salt, &cache_kinds)
        .map_err(Error::from_reason)?;

    Ok(TransformResult {
        code: result.code,
        needs_react_cache: result.needs_react_cache,
        needs_cache_wrapper: result.needs_cache_wrapper,
        needs_register_ref: result.needs_register_ref,
    })
}
