use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

use bytes::Bytes;
use cow_utils::CowUtils;
use rari_error::RariError;
use rustc_hash::FxHashMap;
use serde_json::Value;
use tokio::sync::mpsc::Receiver;

use super::LayoutRenderContext;
use crate::server::{
    core::utils::component::{readable_component_id, short_hash},
    routing::{app_router::AppRouteMatch, types::ParamValue},
};

pub fn generate_cache_key(
    route_match: &AppRouteMatch,
    context: &LayoutRenderContext,
    cookie_header: Option<&str>,
) -> u64 {
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

    if let Some(cookie_header) = cookie_header.filter(|value| !value.is_empty()) {
        cookie_header.hash(&mut hasher);
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

pub fn component_dist_path(base_path: &Path, file_path: &str) -> PathBuf {
    base_path.join(format!("{}.js", create_component_id(file_path)))
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

pub fn format_action_post_url(
    pathname: &str,
    search_params: &FxHashMap<String, Vec<String>>,
) -> String {
    if search_params.is_empty() {
        return pathname.to_string();
    }

    let mut keys: Vec<_> = search_params.keys().collect();
    keys.sort_unstable();
    let mut query_pairs = Vec::new();
    for key in keys {
        let values = &search_params[key];
        for value in values {
            query_pairs.push(format!(
                "{}={}",
                urlencoding::encode(key),
                urlencoding::encode(value)
            ));
        }
    }

    format!("{pathname}?{}", query_pairs.join("&"))
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
    LayoutRenderContext {
        params,
        search_params,
        headers,
        pathname,
        template_navigation_id: None,
        metadata: None,
    }
}

pub fn template_key_json(context: &LayoutRenderContext) -> String {
    let template_key = if let Some(navigation_id) = context.template_navigation_id {
        format!("{}:{}", context.pathname, navigation_id)
    } else {
        context.pathname.clone()
    };

    serde_json::to_string(&template_key).unwrap_or_else(|_| "null".to_string())
}

pub async fn drain_chunked_stream(
    shell: Bytes,
    closing: Bytes,
    chunks: &mut Receiver<Result<Vec<u8>, RariError>>,
) -> Result<String, RariError> {
    let mut output = String::from_utf8_lossy(&shell).into_owned();

    while let Some(chunk_result) = chunks.recv().await {
        match chunk_result {
            Ok(data) => output.push_str(&String::from_utf8_lossy(&data)),
            Err(error) => return Err(error),
        }
    }

    output.push_str(&String::from_utf8_lossy(&closing));
    Ok(output)
}

pub fn sort_flight_protocol(flight_protocol: &str) -> String {
    let mut rows_with_ids: Vec<(u32, String)> = Vec::new();

    for row in flight_protocol.lines() {
        if let Some(colon_pos) = row.find(':') {
            if let Ok(row_id) = u32::from_str_radix(&row[..colon_pos], 16) {
                rows_with_ids.push((row_id, row.to_string()));
            } else {
                rows_with_ids.push((u32::MAX, row.to_string()));
            }
        } else {
            rows_with_ids.push((u32::MAX, row.to_string()));
        }
    }

    rows_with_ids.sort_by_key(|(id, _)| *id);

    let mut sorted =
        rows_with_ids.iter().map(|(_, row)| row.as_str()).collect::<Vec<_>>().join("\n");

    if !sorted.is_empty() && !sorted.ends_with('\n') {
        sorted.push('\n');
    }

    let has_row_0 = rows_with_ids.iter().any(|(id, row)| *id == 0 && row.starts_with("0:"));

    if !has_row_0
        && let Some((max_id, _)) =
            rows_with_ids.iter().filter(|(id, _)| *id != u32::MAX).max_by_key(|(id, _)| *id)
        && *max_id > 0
    {
        let row_0 = format!("0:\"${max_id:x}\"\n");
        sorted.insert_str(0, &row_0);
    }

    sorted
}

#[cfg(test)]
mod flight_tests {
    use super::sort_flight_protocol;

    #[test]
    fn test_sort_flight_protocol_orders_rows() {
        let input = "2:\"b\"\n1:\"a\"\n3:\"c\"";
        let sorted = sort_flight_protocol(input);
        assert_eq!(sorted, "0:\"$3\"\n1:\"a\"\n2:\"b\"\n3:\"c\"\n");
    }

    #[test]
    fn test_sort_flight_protocol_preserves_existing_row_0() {
        let input = "2:\"b\"\n0:\"$2\"\n1:\"a\"";
        let sorted = sort_flight_protocol(input);
        assert_eq!(sorted, "0:\"$2\"\n1:\"a\"\n2:\"b\"\n");
    }

    #[test]
    fn test_sort_flight_protocol_empty() {
        assert_eq!(sort_flight_protocol(""), "");
    }
}
