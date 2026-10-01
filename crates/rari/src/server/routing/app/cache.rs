use std::{
    io::{Cursor, Error},
    sync::Arc,
};

use async_compression::tokio::bufread::{BrotliDecoder, GzipDecoder, ZstdDecoder};
use axum::http::{HeaderMap, HeaderValue};
use bytes::Bytes;
use rustc_hash::FxHashMap;

use crate::server::{
    ServerState,
    actions::{has_action_form_state_cookie, response_cache_cookie_partition},
    cache::response,
    compression::CompressionEncoding,
    host::utils::http::merge_vary_with_accept,
};

pub fn request_cookie_header(headers: &HeaderMap) -> Option<&str> {
    headers.get("cookie").and_then(|value| value.to_str().ok()).filter(|value| !value.is_empty())
}

pub fn route_query_params_for_cache(
    query_params: &FxHashMap<String, String>,
) -> Option<FxHashMap<String, String>> {
    let filtered: FxHashMap<String, String> = query_params
        .iter()
        .filter(|(key, _)| !is_cache_noise_query_param(key))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    if filtered.is_empty() { None } else { Some(filtered) }
}

pub fn is_cache_noise_query_param(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key.starts_with("utm_")
        || matches!(
            key.as_str(),
            "fbclid"
                | "gclid"
                | "gbraid"
                | "wbraid"
                | "msclkid"
                | "mc_cid"
                | "mc_eid"
                | "_ga"
                | "_gl"
                | "ref"
        )
}

pub fn static_html_vary_header(cookie_header: Option<&str>) -> String {
    let mut parts = vec!["Accept", "Accept-Encoding"];
    if cookie_header.is_some() {
        parts.push("Cookie");
    }
    parts.join(", ")
}

/// Static fast-cache entries are keyed without cookies because only cookie-independent
/// HTML is stored. Skip the fast path when action form state is present since that
/// cookie is injected into SSR before render.
pub fn can_use_static_fast_cache(cookie_header: Option<&str>) -> bool {
    !has_action_form_state_cookie(cookie_header)
}

pub fn response_cache_key(
    path: &str,
    query_params_ref: Option<&FxHashMap<String, String>>,
    render_mode: Option<&str>,
    cookie_header: Option<&str>,
) -> String {
    let cache_cookie = response_cache_cookie_partition(cookie_header);
    response::ResponseCache::generate_cache_key_with_mode(
        path,
        query_params_ref,
        render_mode,
        cache_cookie.as_deref(),
    )
}

pub fn rsc_vary_header(cookie_header: Option<&str>, router_state_sensitive: bool) -> String {
    let mut headers = HeaderMap::new();
    let mut parts: Vec<&str> = Vec::new();
    if cookie_header.is_some() {
        parts.push("Cookie");
    }
    if router_state_sensitive {
        parts.push("rari-router-state");
    }
    if !parts.is_empty() {
        if let Ok(value) = HeaderValue::from_str(&parts.join(", ")) {
            headers.insert("vary", value);
        }
    }
    merge_vary_with_accept(headers.get("vary"))
}

pub fn insert_response_cache_vary_header(
    headers: &mut HeaderMap,
    cookie_header: Option<&str>,
    html: bool,
) {
    let merged = if html {
        static_html_vary_header(cookie_header)
    } else {
        rsc_vary_header(cookie_header, false)
    };

    if html || cookie_header.is_some() {
        if let Ok(value) = HeaderValue::from_str(&merged) {
            headers.insert("vary", value);
        }
    }
}

pub async fn should_store_response_cache(
    state: &ServerState,
    cache_policy: &response::RouteCachePolicy,
) -> bool {
    if !cache_policy.enabled || !state.response_cache.config.enabled {
        return false;
    }

    let runtime = {
        let renderer = state.renderer.lock().await;
        Arc::clone(&renderer.runtime)
    };

    !runtime.is_dynamic_render().await.unwrap_or(true)
}

pub async fn decompress_bytes(data: &Bytes, encoding: CompressionEncoding) -> Result<Bytes, Error> {
    use tokio::io::AsyncReadExt;

    let data = data.clone();
    match encoding {
        CompressionEncoding::Gzip => {
            let mut decoder = GzipDecoder::new(Cursor::new(&data[..]));
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed).await?;
            Ok(Bytes::from(decompressed))
        }
        CompressionEncoding::Brotli => {
            let mut decoder = BrotliDecoder::new(Cursor::new(&data[..]));
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed).await?;
            Ok(Bytes::from(decompressed))
        }
        CompressionEncoding::Zstd => {
            let mut decoder = ZstdDecoder::new(Cursor::new(&data[..]));
            let mut decompressed = Vec::new();
            decoder.read_to_end(&mut decompressed).await?;
            Ok(Bytes::from(decompressed))
        }
        CompressionEncoding::Identity => Ok(data),
    }
}

#[cfg(test)]
#[expect(clippy::expect_used)]
mod tests {
    use rustc_hash::FxHashMap;

    use super::*;
    use crate::server::{cache::response, host::utils::http::extract_search_params};

    #[test]
    fn test_route_query_params_for_cache_strips_tracking() {
        let mut params = FxHashMap::default();
        params.insert("utm_source".to_string(), "twitter".to_string());
        params.insert("UTM_SOURCE".to_string(), "Ads".to_string());
        params.insert("page".to_string(), "2".to_string());
        params.insert("fbclid".to_string(), "abc".to_string());
        params.insert("FbClId".to_string(), "mixed".to_string());
        params.insert("gclid".to_string(), "xyz".to_string());

        let filtered = route_query_params_for_cache(&params).expect("page should remain");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered.get("page").map(String::as_str), Some("2"));
        assert!(!filtered.contains_key("UTM_SOURCE"));
        assert!(!filtered.contains_key("FbClId"));

        let key =
            response::ResponseCache::generate_static_fast_cache_key("/", Some(&filtered), None);
        assert_eq!(key, "/?page=2");
    }

    #[test]
    fn test_route_query_params_for_cache_all_noise_is_none() {
        let mut params = FxHashMap::default();
        params.insert("utm_campaign".to_string(), "launch".to_string());
        params.insert("ref".to_string(), "nav".to_string());
        assert!(route_query_params_for_cache(&params).is_none());
    }

    #[test]
    fn test_filtered_tracking_params_share_cache_key_and_render_inputs() {
        let mut request_a = FxHashMap::default();
        request_a.insert("page".to_string(), "2".to_string());
        request_a.insert("utm_source".to_string(), "twitter".to_string());
        request_a.insert("UTM_SOURCE".to_string(), "Twitter".to_string());

        let mut request_b = FxHashMap::default();
        request_b.insert("page".to_string(), "2".to_string());
        request_b.insert("utm_source".to_string(), "newsletter".to_string());
        request_b.insert("fbclid".to_string(), "abc".to_string());
        request_b.insert("FbClId".to_string(), "ABC".to_string());

        let filtered_a = route_query_params_for_cache(&request_a);
        let filtered_b = route_query_params_for_cache(&request_b);

        let key_a =
            response::ResponseCache::generate_static_fast_cache_key("/", filtered_a.as_ref(), None);
        let key_b =
            response::ResponseCache::generate_static_fast_cache_key("/", filtered_b.as_ref(), None);
        assert_eq!(key_a, key_b);
        assert_eq!(key_a, "/?page=2");

        let render_a = extract_search_params(filtered_a.unwrap_or_default());
        let render_b = extract_search_params(filtered_b.unwrap_or_default());
        assert_eq!(render_a, render_b);
        assert_eq!(render_a.get("page"), Some(&vec!["2".to_string()]));
        assert!(!render_a.contains_key("utm_source"));
        assert!(!render_a.contains_key("UTM_SOURCE"));
        assert!(!render_b.contains_key("fbclid"));
        assert!(!render_b.contains_key("FbClId"));
    }
}
