use crate::error::RariError;
use crate::server::routing::app_router::AppRouteMatch;
use cow_utils::CowUtils;
use rustc_hash::FxHashMap;
use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::LayoutRenderContext;

pub fn generate_cache_key(route_match: &AppRouteMatch, context: &LayoutRenderContext) -> u64 {
    let mut hasher = DefaultHasher::new();

    route_match.route.path.hash(&mut hasher);

    let mut params: Vec<_> = context.params.iter().collect();
    params.sort_by_key(|(k, _)| *k);
    for (k, v) in params {
        k.hash(&mut hasher);
        v.hash(&mut hasher);
    }

    let mut search_params: Vec<_> = context.search_params.iter().collect();
    search_params.sort_by_key(|(k, _)| *k);
    for (k, v) in search_params {
        k.hash(&mut hasher);
        v.hash(&mut hasher);
    }

    hasher.finish()
}

pub fn create_component_id(file_path: &str) -> String {
    let normalized = file_path
        .cow_replace(".tsx", "")
        .cow_replace(".ts", "")
        .cow_replace("[", "_")
        .cow_replace("]", "_")
        .into_owned();
    format!("app/{}", normalized)
}

pub fn get_component_id(file_path: &str) -> String {
    let path = std::path::Path::new(file_path);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Unknown");

    let mut chars = stem.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

pub fn create_page_props(
    route_match: &AppRouteMatch,
    context: &LayoutRenderContext,
) -> Result<Value, RariError> {
    let params_value = if route_match.params.is_empty() {
        Value::Object(serde_json::Map::new())
    } else {
        serde_json::to_value(&route_match.params)?
    };

    let search_params_value = if context.search_params.is_empty() {
        Value::Object(serde_json::Map::new())
    } else {
        serde_json::to_value(&context.search_params)?
    };

    #[allow(clippy::disallowed_methods)]
    let result = serde_json::json!({
        "params": params_value,
        "searchParams": search_params_value
    });
    Ok(result)
}

pub fn create_layout_context(
    params: FxHashMap<String, String>,
    search_params: FxHashMap<String, Vec<String>>,
    headers: FxHashMap<String, String>,
    pathname: String,
) -> LayoutRenderContext {
    LayoutRenderContext { params, search_params, headers, pathname, metadata: None }
}
