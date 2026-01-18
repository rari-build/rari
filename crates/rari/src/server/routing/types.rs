use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RouteSegmentType {
    Static,
    Dynamic,
    CatchAll,
    OptionalCatchAll,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteSegment {
    #[serde(rename = "type")]
    pub segment_type: RouteSegmentType,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum ParamValue {
    Single(String),
    Multiple(Vec<String>),
}

impl fmt::Display for ParamValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParamValue::Single(s) => write!(f, "{}", s),
            ParamValue::Multiple(v) => write!(f, "{}", v.join("/")),
        }
    }
}

impl ParamValue {
    pub fn as_string(&self) -> Option<&String> {
        match self {
            ParamValue::Single(s) => Some(s),
            ParamValue::Multiple(_) => None,
        }
    }

    pub fn as_vec(&self) -> Option<&Vec<String>> {
        match self {
            ParamValue::Single(_) => None,
            ParamValue::Multiple(v) => Some(v),
        }
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use rustc_hash::FxHashMap;

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
        assert_eq!(json.get("path").and_then(|v| v.as_array()).map(|a| a.len()), Some(2));
    }

    #[test]
    fn test_param_value_display() {
        let single = ParamValue::Single("hello".to_string());
        let multiple = ParamValue::Multiple(vec!["a".to_string(), "b".to_string()]);

        assert_eq!(single.to_string(), "hello");
        assert_eq!(multiple.to_string(), "a/b");
    }
}
