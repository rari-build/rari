use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ReactElement {
    #[serde(rename = "type", alias = "tag")]
    pub tag: String,
    pub props: FxHashMap<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
}

impl ReactElement {
    pub fn new(tag: impl Into<String>) -> Self {
        Self { tag: tag.into(), props: FxHashMap::default(), key: None }
    }

    pub fn with_props(tag: impl Into<String>, props: FxHashMap<String, Value>) -> Self {
        Self { tag: tag.into(), props, key: None }
    }

    #[must_use]
    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    pub fn add_prop(&mut self, name: impl Into<String>, value: Value) {
        self.props.insert(name.into(), value);
    }

    pub fn get_prop(&self, name: &str) -> Option<&Value> {
        self.props.get(name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RSCTree {
    ClientReference {
        id: String,
        key: Option<String>,
        props: FxHashMap<String, Value>,
    },
    ServerElement {
        tag: String,
        props: Option<FxHashMap<String, Value>>,
        children: Option<Vec<RSCTree>>,
        key: Option<String>,
    },
    Text(String),
    Fragment {
        children: Vec<RSCTree>,
        key: Option<String>,
    },
    Array(Vec<RSCTree>),
    Null,
    Primitive(Value),
    Error {
        message: String,
        component_name: String,
        stack: Option<String>,
    },
}

impl RSCTree {
    #[cfg(test)]
    pub fn client_reference(id: &str, key: Option<&str>, props: FxHashMap<String, Value>) -> Self {
        RSCTree::ClientReference { id: id.to_string(), key: key.map(ToString::to_string), props }
    }

    #[cfg(test)]
    pub fn server_element(
        tag: &str,
        props: Option<FxHashMap<String, Value>>,
        children: Option<Vec<RSCTree>>,
        key: Option<&str>,
    ) -> Self {
        RSCTree::ServerElement {
            tag: tag.to_string(),
            props,
            children,
            key: key.map(ToString::to_string),
        }
    }

    pub fn text(content: &str) -> Self {
        RSCTree::Text(content.to_string())
    }

    pub fn fragment(children: Vec<RSCTree>, key: Option<&str>) -> Self {
        RSCTree::Fragment { children, key: key.map(std::string::ToString::to_string) }
    }

    pub fn array(elements: Vec<RSCTree>) -> Self {
        RSCTree::Array(elements)
    }

    pub fn null() -> Self {
        RSCTree::Null
    }

    pub fn primitive(value: Value) -> Self {
        RSCTree::Primitive(value)
    }

    pub fn error(message: &str, component_name: &str, stack: Option<&str>) -> Self {
        RSCTree::Error {
            message: message.to_string(),
            component_name: component_name.to_string(),
            stack: stack.map(std::string::ToString::to_string),
        }
    }

    pub fn has_client_components(&self) -> bool {
        match self {
            RSCTree::ClientReference { .. } => true,
            RSCTree::ServerElement { children, .. } => {
                children.as_ref().is_some_and(|c| c.iter().any(RSCTree::has_client_components))
            }
            RSCTree::Fragment { children, .. } => {
                children.iter().any(RSCTree::has_client_components)
            }
            RSCTree::Array(elements) => elements.iter().any(RSCTree::has_client_components),
            _ => false,
        }
    }

    pub fn collect_client_component_ids(&self) -> Vec<String> {
        let mut ids = Vec::new();
        self.collect_client_component_ids_recursive(&mut ids);
        ids
    }

    fn collect_client_component_ids_recursive(&self, ids: &mut Vec<String>) {
        match self {
            RSCTree::ClientReference { id, .. } => {
                ids.push(id.clone());
            }
            RSCTree::ServerElement { children: Some(children), .. }
            | RSCTree::Fragment { children, .. } => {
                for child in children {
                    child.collect_client_component_ids_recursive(ids);
                }
            }
            RSCTree::Array(elements) => {
                for element in elements {
                    element.collect_client_component_ids_recursive(ids);
                }
            }
            RSCTree::ServerElement { children: None, .. } | _ => {}
        }
    }

    pub fn to_json(&self) -> Value {
        match self {
            RSCTree::ClientReference { id, key, props } => {
                serde_json::json!(["$", id, key, props])
            }
            RSCTree::ServerElement { tag, props, children, key } => {
                let mut element = serde_json::json!({
                    "$$typeof": "react.transitional.element",
                    "type": tag,
                    "key": key,
                    "props": props.clone().unwrap_or_default()
                });

                if let Some(children) = children {
                    let children_json: Vec<Value> = children.iter().map(RSCTree::to_json).collect();
                    if let Some(props) = element.get_mut("props")
                        && let Some(props_obj) = props.as_object_mut()
                    {
                        props_obj.insert("children".to_string(), Value::Array(children_json));
                    }
                }

                element
            }
            RSCTree::Text(content) => Value::String(content.clone()),
            RSCTree::Fragment { children, .. } => {
                let children_json: Vec<Value> = children.iter().map(RSCTree::to_json).collect();
                Value::Array(children_json)
            }
            RSCTree::Array(elements) => {
                let elements_json: Vec<Value> = elements.iter().map(RSCTree::to_json).collect();
                Value::Array(elements_json)
            }
            RSCTree::Null => Value::Null,
            RSCTree::Primitive(value) => value.clone(),
            RSCTree::Error { message, component_name, .. } => {
                serde_json::json!({
                    "$$typeof": "react.transitional.element",
                    "type": "div",
                    "props": {
                        "style": {
                            "color": "red",
                            "border": "1px solid red",
                            "padding": "10px",
                            "margin": "10px"
                        },
                        "children": [
                            {
                                "$$typeof": "react.transitional.element",
                                "type": "h3",
                                "props": {
                                    "children": format!("Error in {}", component_name)
                                }
                            },
                            {
                                "$$typeof": "react.transitional.element",
                                "type": "p",
                                "props": {
                                    "children": message
                                }
                            }
                        ]
                    }
                })
            }
        }
    }

    /// Parses an RSCTree from a JSON value.
    ///
    /// # Errors
    ///
    /// Returns an error if the JSON structure is invalid or cannot be parsed as an RSCTree.
    #[expect(clippy::too_many_lines, reason = "Complex JSON parsing logic with many variants")]
    pub fn from_json(value: &Value) -> Result<Self, String> {
        match value {
            Value::String(s) => Ok(RSCTree::Text(s.clone())),
            Value::Number(n) => Ok(RSCTree::Primitive(Value::Number(n.clone()))),
            Value::Bool(b) => Ok(RSCTree::Primitive(Value::Bool(*b))),
            Value::Null => Ok(RSCTree::Null),
            Value::Array(arr) => {
                if arr.len() == 4 && arr[0] == "$" {
                    let id = arr[1].as_str().ok_or("Invalid reference ID")?;
                    let is_client_reference = id.starts_with("$L") || id.contains('#');

                    if is_client_reference {
                        let key = arr[2].as_str().map(std::string::ToString::to_string);
                        let props = arr[3]
                            .as_object()
                            .ok_or("Invalid client reference props")?
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect();

                        Ok(RSCTree::ClientReference { id: id.to_string(), key, props })
                    } else {
                        let tag = id;
                        let key = arr[2].as_str().map(std::string::ToString::to_string);
                        let props_obj = arr[3].as_object().ok_or("Invalid element props")?;

                        let mut props: FxHashMap<String, Value> =
                            props_obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

                        let children = props
                            .remove("children")
                            .map(|c| {
                                if c.is_array() {
                                    c.as_array()
                                        .ok_or("Invalid children array")?
                                        .iter()
                                        .map(RSCTree::from_json)
                                        .collect::<Result<Vec<RSCTree>, String>>()
                                } else {
                                    Ok(vec![RSCTree::from_json(&c)?])
                                }
                            })
                            .transpose()?;

                        Ok(RSCTree::ServerElement {
                            tag: tag.to_string(),
                            props: if props.is_empty() { None } else { Some(props) },
                            children,
                            key,
                        })
                    }
                } else {
                    let elements: Result<Vec<RSCTree>, String> =
                        arr.iter().map(RSCTree::from_json).collect();
                    Ok(RSCTree::Array(elements?))
                }
            }
            Value::Object(obj) => {
                if obj.contains_key("~preSerializedSuspense") && obj.contains_key("rscArray") {
                    if let Some(rsc_array) = obj.get("rscArray") {
                        return RSCTree::from_json(rsc_array);
                    }
                    tracing::error!(
                        "Pre-serialized Suspense marker found but rscArray is missing or invalid"
                    );
                    return Ok(RSCTree::Primitive(value.clone()));
                }

                if obj.contains_key("$$typeof") && obj.contains_key("type") {
                    let tag =
                        obj.get("type").and_then(|t| t.as_str()).ok_or("Invalid element type")?;
                    let key = obj
                        .get("key")
                        .and_then(|k| k.as_str())
                        .map(std::string::ToString::to_string);
                    let props = obj
                        .get("props")
                        .and_then(|p| p.as_object())
                        .map(|p| p.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                        .unwrap_or_default();

                    let children = obj
                        .get("props")
                        .and_then(|p| p.get("children"))
                        .map(|c| {
                            if c.is_array() {
                                c.as_array()
                                    .ok_or("Invalid children array")?
                                    .iter()
                                    .map(RSCTree::from_json)
                                    .collect::<Result<Vec<RSCTree>, String>>()
                            } else {
                                Ok(vec![RSCTree::from_json(c)?])
                            }
                        })
                        .transpose()?;

                    Ok(RSCTree::ServerElement {
                        tag: tag.to_string(),
                        props: Some(props),
                        children,
                        key,
                    })
                } else {
                    Ok(RSCTree::Primitive(value.clone()))
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RSCRenderResult {
    pub tree: RSCTree,
    pub has_suspense: bool,
    pub client_components: Vec<String>,
    pub debug: Option<RSCRenderDebug>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RSCRenderDebug {
    pub component_id: String,
    pub success: bool,
    pub error: Option<String>,
    pub has_client_components: bool,
    pub resolved_from_suspense: bool,
    pub render_type: String,
}

impl RSCRenderResult {
    pub fn success(tree: RSCTree, component_id: &str) -> Self {
        let client_components = tree.collect_client_component_ids();
        let has_client_components = !client_components.is_empty();

        RSCRenderResult {
            tree,
            has_suspense: false,
            client_components,
            debug: Some(RSCRenderDebug {
                component_id: component_id.to_string(),
                success: true,
                error: None,
                has_client_components,
                resolved_from_suspense: false,
                render_type: "rsc_tree".to_string(),
            }),
        }
    }

    pub fn error(error: &str, component_id: &str) -> Self {
        RSCRenderResult {
            tree: RSCTree::error(error, component_id, None),
            has_suspense: false,
            client_components: vec![],
            debug: Some(RSCRenderDebug {
                component_id: component_id.to_string(),
                success: false,
                error: Some(error.to_string()),
                has_client_components: false,
                resolved_from_suspense: false,
                render_type: "error".to_string(),
            }),
        }
    }

    #[must_use]
    pub fn with_suspense(mut self) -> Self {
        self.has_suspense = true;
        if let Some(debug) = &mut self.debug {
            debug.resolved_from_suspense = true;
        }
        self
    }
}

#[cfg(test)]
#[allow(
    clippy::allow_attributes,
    clippy::unreadable_literal,
    clippy::needless_raw_string_hashes,
    clippy::panic,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::float_cmp,
    clippy::bool_assert_comparison,
    clippy::redundant_clone,
    clippy::redundant_closure_for_method_calls,
    clippy::single_char_pattern,
    clippy::approx_constant,
    clippy::uninlined_format_args,
    clippy::module_inception,
    clippy::return_self_not_must_use,
    clippy::disallowed_methods,
    clippy::clone_on_ref_ptr,
    clippy::get_unwrap
)]
mod tests {
    use serde_json::json;

    use super::*;

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

    #[test]
    fn test_client_reference_creation() {
        let mut props = FxHashMap::default();
        props.insert("onClick".to_string(), json!("handleClick"));

        let tree = RSCTree::client_reference("Counter", Some("key1"), props);
        match tree {
            RSCTree::ClientReference { id, key, props } => {
                assert_eq!(id, "Counter");
                assert_eq!(key, Some("key1".to_string()));
                assert_eq!(props.get("onClick"), Some(&json!("handleClick")));
            }
            _ => panic!("Expected ClientReference"),
        }
    }

    #[test]
    fn test_server_element_creation() {
        let children = vec![RSCTree::text("Hello World")];
        let mut props = FxHashMap::default();
        props.insert("className".to_string(), json!("container"));

        let tree = RSCTree::server_element("div", Some(props), Some(children), None);
        match tree {
            RSCTree::ServerElement { tag, props, children, .. } => {
                assert_eq!(tag, "div");
                assert_eq!(props.as_ref().unwrap().get("className"), Some(&json!("container")));
                assert_eq!(children.as_ref().unwrap().len(), 1);
            }
            _ => panic!("Expected ServerElement"),
        }
    }

    #[test]
    fn test_has_client_components() {
        let client_ref = RSCTree::client_reference("Counter", None, FxHashMap::default());
        assert!(client_ref.has_client_components());

        let text = RSCTree::text("Hello");
        assert!(!text.has_client_components());

        let server_with_client = RSCTree::server_element("div", None, Some(vec![client_ref]), None);
        assert!(server_with_client.has_client_components());
    }

    #[test]
    fn test_json_serialization() {
        let mut props = FxHashMap::default();
        props.insert("count".to_string(), json!(42));

        let tree = RSCTree::client_reference("Counter", None, props);
        let json_value = tree.to_json();

        assert_eq!(json_value[0], "$");
        assert_eq!(json_value[1], "Counter");
        assert_eq!(json_value[3]["count"], 42);
    }

    #[test]
    fn test_json_deserialization() {
        let json_value = json!(["$", "$L0", null, {"count": 42}]);
        let tree = RSCTree::from_json(&json_value).unwrap();

        match tree {
            RSCTree::ClientReference { id, key, props } => {
                assert_eq!(id, "$L0");
                assert_eq!(key, None);
                assert_eq!(props.get("count"), Some(&json!(42)));
            }
            _ => panic!("Expected ClientReference"),
        }
    }
}
