use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactElement {
    #[serde(rename = "type", alias = "tag")]
    pub tag: String,
    pub props: FxHashMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
}

impl ReactElement {
    pub fn new(tag: impl Into<String>) -> Self {
        Self { tag: tag.into(), props: FxHashMap::default(), key: None }
    }

    pub fn with_props(tag: impl Into<String>, props: FxHashMap<String, serde_json::Value>) -> Self {
        Self { tag: tag.into(), props, key: None }
    }

    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    pub fn add_prop(&mut self, name: impl Into<String>, value: serde_json::Value) {
        self.props.insert(name.into(), value);
    }

    pub fn get_prop(&self, name: &str) -> Option<&serde_json::Value> {
        self.props.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_create_react_element() {
        let element = ReactElement::new("div");
        assert_eq!(element.tag, "div");
        assert!(element.props.is_empty());
        assert!(element.key.is_none());
    }

    #[test]
    fn test_create_with_props() {
        let mut props = FxHashMap::default();
        props.insert("className".to_string(), json!("test-class"));
        props.insert("children".to_string(), json!("Hello"));

        let element = ReactElement::with_props("div", props);
        assert_eq!(element.tag, "div");
        assert_eq!(element.props.len(), 2);
        assert_eq!(element.get_prop("className"), Some(&json!("test-class")));
    }

    #[test]
    fn test_with_key() {
        let element = ReactElement::new("div").with_key("my-key");
        assert_eq!(element.key, Some("my-key".to_string()));
    }

    #[test]
    fn test_add_prop() {
        let mut element = ReactElement::new("div");
        element.add_prop("id", json!("my-id"));
        element.add_prop("className", json!("container"));

        assert_eq!(element.props.len(), 2);
        assert_eq!(element.get_prop("id"), Some(&json!("my-id")));
        assert_eq!(element.get_prop("className"), Some(&json!("container")));
    }

    #[test]
    fn test_serialize_deserialize() {
        let mut props = FxHashMap::default();
        props.insert("className".to_string(), json!("test"));
        props.insert("children".to_string(), json!("Content"));

        let element = ReactElement::with_props("div", props).with_key("test-key");

        let json_str = serde_json::to_string(&element).unwrap();

        let deserialized: ReactElement = serde_json::from_str(&json_str).unwrap();

        assert_eq!(deserialized.tag, "div");
        assert_eq!(deserialized.key, Some("test-key".to_string()));
        assert_eq!(deserialized.props.len(), 2);
        assert_eq!(deserialized.get_prop("className"), Some(&json!("test")));
    }

    #[test]
    fn test_suspense_element() {
        let mut props = FxHashMap::default();
        props.insert("fallback".to_string(), json!({"tag": "div", "props": {}}));
        props.insert("children".to_string(), json!({"tag": "div", "props": {}}));
        props.insert("~boundaryId".to_string(), json!("boundary-1"));

        let element = ReactElement::with_props("react.suspense", props);

        assert_eq!(element.tag, "react.suspense");
        assert!(element.get_prop("fallback").is_some());
        assert!(element.get_prop("children").is_some());
        assert_eq!(element.get_prop("~boundaryId"), Some(&json!("boundary-1")));
    }
}
