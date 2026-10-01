use rustc_hash::FxHashMap;

use super::types::ParamValue;

fn parse_decoded_path_segments(path: &str) -> Vec<String> {
    path.split('/')
        .filter(|s| !s.is_empty())
        .map(|s| urlencoding::decode(s).unwrap_or_else(|_| s.to_string().into()).into_owned())
        .collect()
}

pub fn match_route_pattern(
    route_path: &str,
    request_path: &str,
) -> Option<FxHashMap<String, ParamValue>> {
    let route_segments: Vec<&str> = route_path.split('/').filter(|s| !s.is_empty()).collect();
    let path_segments = parse_decoded_path_segments(request_path);

    let mut params = FxHashMap::default();
    let mut route_idx = 0;
    let mut path_idx = 0;

    while route_idx < route_segments.len() {
        let route_seg = route_segments[route_idx];

        if route_seg.starts_with("[[...") && route_seg.ends_with("]]") {
            let param_name = &route_seg[5..route_seg.len() - 2];

            if path_idx < path_segments.len() {
                let remaining: Vec<String> = path_segments[path_idx..].to_vec();
                params.insert(param_name.to_string(), ParamValue::Multiple(remaining));
            }

            return Some(params);
        }

        if route_seg.starts_with("[...") && route_seg.ends_with(']') {
            let param_name = &route_seg[4..route_seg.len() - 1];

            if path_idx >= path_segments.len() {
                return None;
            }

            let remaining: Vec<String> = path_segments[path_idx..].to_vec();
            params.insert(param_name.to_string(), ParamValue::Multiple(remaining));

            return Some(params);
        }

        if route_seg.starts_with('[') && route_seg.ends_with(']') {
            if path_idx >= path_segments.len() {
                return None;
            }

            let param_name = &route_seg[1..route_seg.len() - 1];
            params.insert(
                param_name.to_string(),
                ParamValue::Single(path_segments[path_idx].clone()),
            );

            path_idx += 1;
            route_idx += 1;
            continue;
        }

        if path_idx >= path_segments.len() || route_seg != path_segments[path_idx] {
            return None;
        }

        path_idx += 1;
        route_idx += 1;
    }

    if path_idx == path_segments.len() { Some(params) } else { None }
}

pub fn match_route_pattern_as_strings(
    route_path: &str,
    request_path: &str,
) -> Option<FxHashMap<String, String>> {
    let params = match_route_pattern(route_path, request_path)?;
    Some(params.into_iter().map(|(k, v)| (k, v.to_string())).collect())
}

#[cfg(test)]
#[expect(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn matches_static_and_dynamic() {
        assert!(match_route_pattern("/about", "/about").unwrap().is_empty());
        assert!(match_route_pattern("/about", "/other").is_none());

        let params = match_route_pattern("/blog/[slug]", "/blog/hello").unwrap();
        assert_eq!(params.get("slug"), Some(&ParamValue::Single("hello".into())));
    }

    #[test]
    fn matches_catch_all_and_optional() {
        let params = match_route_pattern("/docs/[...slug]", "/docs/a/b").unwrap();
        assert_eq!(params.get("slug"), Some(&ParamValue::Multiple(vec!["a".into(), "b".into()])));
        assert!(match_route_pattern("/docs/[...slug]", "/docs").is_none());

        let empty = match_route_pattern("/docs/[[...slug]]", "/docs").unwrap();
        assert!(!empty.contains_key("slug"));

        let filled = match_route_pattern("/docs/[[...slug]]", "/docs/a").unwrap();
        assert_eq!(filled.get("slug"), Some(&ParamValue::Multiple(vec!["a".into()])));
    }

    #[test]
    fn string_map_joins_catch_all() {
        let params = match_route_pattern_as_strings("/docs/[...slug]", "/docs/a/b").unwrap();
        assert_eq!(params.get("slug").map(String::as_str), Some("a/b"));
    }

    #[test]
    fn decodes_path_segments() {
        let params = match_route_pattern("/p/[id]", "/p/hello%20world").unwrap();
        assert_eq!(params.get("id"), Some(&ParamValue::Single("hello world".into())));
    }
}
