use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

use cow_utils::CowUtils;
use rari_error::RariError;
use rustc_hash::FxHashMap;
use serde_json::Value;

use super::LayoutRenderContext;
use crate::server::{
    core::utils::component::{readable_component_id, short_hash},
    routing::{app_router::AppRouteMatch, types::ParamValue},
};

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

fn normalize_route_component_path(file_path: &str) -> String {
    let normalized = file_path.cow_replace('\\', "/").into_owned();
    if normalized.starts_with("src/") {
        normalized
    } else if normalized.starts_with("app/") {
        format!("src/{normalized}")
    } else {
        format!("src/app/{normalized}")
    }
}

pub fn normalize_route_component_path_public(file_path: &str) -> String {
    normalize_route_component_path(file_path)
}

pub fn create_component_id(file_path: &str) -> String {
    let project_relative_path = normalize_route_component_path(file_path);
    format!(
        "{}_{}",
        readable_component_id(&project_relative_path),
        short_hash(&project_relative_path)
    )
}

pub fn create_client_component_id(file_path: &str) -> String {
    let project_relative_path = normalize_route_component_path(file_path);
    project_relative_path
        .trim_end_matches(".tsx")
        .trim_end_matches(".ts")
        .trim_end_matches(".jsx")
        .trim_end_matches(".js")
        .to_string()
}

pub fn convert_route_path_to_dist_path(path: &str) -> String {
    let (base, ext) =
        if let Some(pos) = path.rfind('.') { (&path[..pos], &path[pos..]) } else { (path, "") };

    let converted_base = base
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '/' || c == '-' || c == '_' { c } else { '_' })
        .collect::<String>();

    format!("{converted_base}{ext}")
}

pub fn component_dist_path(
    base_path: &Path,
    file_path: &str,
    component_id: Option<&str>,
) -> PathBuf {
    if let Some(component_id) = component_id {
        return base_path.join(format!("{component_id}.js"));
    }

    let js_filename = file_path.cow_replace(".tsx", ".js").cow_replace(".ts", ".js").into_owned();
    let dist_filename = convert_route_path_to_dist_path(&js_filename);
    base_path.join("app").join(&dist_filename)
}

pub fn get_component_id(file_path: &str) -> String {
    let path = Path::new(file_path);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Unknown");

    let mut chars = stem.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let mut result = first.to_uppercase().collect::<String>();
            result.push_str(chars.as_str());
            result
        }
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

    let result = serde_json::json!({
        "params": params_value,
        "searchParams": search_params_value
    });
    Ok(result)
}

#[expect(
    clippy::implicit_hasher,
    reason = "FxHashMap is the specific hasher needed for LayoutRenderContext"
)]
pub fn create_layout_context(
    params: FxHashMap<String, ParamValue>,
    search_params: FxHashMap<String, Vec<String>>,
    headers: FxHashMap<String, String>,
    pathname: String,
) -> LayoutRenderContext {
    LayoutRenderContext { params, search_params, headers, pathname, metadata: None }
}
