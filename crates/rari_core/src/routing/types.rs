use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum RouteSegmentType {
    Static,
    Dynamic,
    CatchAll,
    OptionalCatchAll,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::exhaustive_structs,
    reason = "cross-crate boundary type: backends construct this exhaustively; in-tree so no semver isolation"
)]
pub struct RouteSegment {
    #[serde(rename = "type")]
    pub segment_type: RouteSegmentType,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
#[non_exhaustive]
pub enum ParamValue {
    Single(String),
    Multiple(Vec<String>),
}

impl fmt::Display for ParamValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Single(s) => write!(f, "{s}"),
            Self::Multiple(v) => write!(f, "{}", v.join("/")),
        }
    }
}

impl ParamValue {
    pub fn as_string(&self) -> Option<&String> {
        match self {
            Self::Single(s) => Some(s),
            Self::Multiple(_) => None,
        }
    }

    pub fn as_vec(&self) -> Option<&Vec<String>> {
        match self {
            Self::Single(_) => None,
            Self::Multiple(v) => Some(v),
        }
    }
}

#[cfg(test)]
#[expect(clippy::unwrap_used)]
mod tests {
    use rustc_hash::FxHashMap;

    use super::*;

    #[test]
    fn test_param_value_serialization() {
        let single = ParamValue::Single("hello".to_string());
        let multiple = ParamValue::Multiple(vec!["a".to_string(), "b".to_string()]);

        let single_json = serde_json::to_string(&single).unwrap();
        let multiple_json = serde_json::to_string(&multiple).unwrap();

        assert_eq!(single_json, r#""hello""#);
        assert_eq!(multiple_json, r#"["a","b"]"#);
    }

    #[test]
    fn test_param_value_map_serialization() {
        let mut map = FxHashMap::default();
        map.insert("slug".to_string(), ParamValue::Single("hello".to_string()));
        map.insert(
            "path".to_string(),
            ParamValue::Multiple(vec!["a".to_string(), "b".to_string()]),
        );

        let json = serde_json::to_value(&map).unwrap();
        assert!(json.is_object());
        assert_eq!(json.get("slug").and_then(|v| v.as_str()), Some("hello"));
        assert_eq!(json.get("path").and_then(|v| v.as_array()).map(Vec::len), Some(2));
    }

    #[test]
    fn test_param_value_display() {
        let single = ParamValue::Single("hello".to_string());
        let multiple = ParamValue::Multiple(vec!["a".to_string(), "b".to_string()]);

        assert_eq!(single.to_string(), "hello");
        assert_eq!(multiple.to_string(), "a/b");
    }
}
