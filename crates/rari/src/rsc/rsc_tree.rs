use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub fn client_reference(id: &str, key: Option<&str>, props: FxHashMap<String, Value>) -> Self {
        RSCTree::ClientReference { id: id.to_string(), key: key.map(|k| k.to_string()), props }
    }

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
            key: key.map(|k| k.to_string()),
        }
    }

    pub fn text(content: &str) -> Self {
        RSCTree::Text(content.to_string())
    }

    pub fn fragment(children: Vec<RSCTree>, key: Option<&str>) -> Self {
        RSCTree::Fragment { children, key: key.map(|k| k.to_string()) }
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
            stack: stack.map(|s| s.to_string()),
        }
    }

    pub fn has_client_components(&self) -> bool {
        match self {
            RSCTree::ClientReference { .. } => true,
            RSCTree::ServerElement { children, .. } => children
                .as_ref()
                .is_some_and(|c| c.iter().any(|child| child.has_client_components())),
            RSCTree::Fragment { children, .. } => {
                children.iter().any(|child| child.has_client_components())
            }
            RSCTree::Array(elements) => {
                elements.iter().any(|element| element.has_client_components())
            }
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
            RSCTree::ServerElement { children: Some(children), .. } => {
                for child in children {
                    child.collect_client_component_ids_recursive(ids);
                }
            }
            RSCTree::ServerElement { children: None, .. } => {
                // No children to process
            }
            RSCTree::Fragment { children, .. } => {
                for child in children {
                    child.collect_client_component_ids_recursive(ids);
                }
            }
            RSCTree::Array(elements) => {
                for element in elements {
                    element.collect_client_component_ids_recursive(ids);
                }
            }
            _ => {}
        }
    }

    pub fn to_json(&self) -> Value {
        match self {
            RSCTree::ClientReference { id, key, props } => {
                #[allow(clippy::disallowed_methods)]
                {
                    serde_json::json!(["$", id, key, props])
                }
            }
            RSCTree::ServerElement { tag, props, children, key } => {
                #[allow(clippy::disallowed_methods)]
                let mut element = serde_json::json!({
                    "$$typeof": "react.element",
                    "type": tag,
                    "key": key,
                    "props": props.clone().unwrap_or_default()
                });

                if let Some(children) = children {
                    let children_json: Vec<Value> = children.iter().map(|c| c.to_json()).collect();
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
                let children_json: Vec<Value> = children.iter().map(|c| c.to_json()).collect();
                Value::Array(children_json)
            }
            RSCTree::Array(elements) => {
                let elements_json: Vec<Value> = elements.iter().map(|e| e.to_json()).collect();
                Value::Array(elements_json)
            }
            RSCTree::Null => Value::Null,
            RSCTree::Primitive(value) => value.clone(),
            RSCTree::Error { message, component_name, .. } => {
                #[allow(clippy::disallowed_methods)]
                {
                    serde_json::json!({
                        "$$typeof": "react.element",
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
                                    "$$typeof": "react.element",
                                    "type": "h3",
                                    "props": {
                                        "children": format!("Error in {}", component_name)
                                    }
                                },
                                {
                                    "$$typeof": "react.element",
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
    }

    pub fn from_json(value: &Value) -> Result<Self, String> {
        match value {
            Value::String(s) => Ok(RSCTree::Text(s.clone())),
            Value::Number(n) => Ok(RSCTree::Primitive(Value::Number(n.clone()))),
            Value::Bool(b) => Ok(RSCTree::Primitive(Value::Bool(*b))),
            Value::Null => Ok(RSCTree::Null),
            Value::Array(arr) => {
                if arr.len() == 4 && arr[0] == "$" {
                    let id = arr[1].as_str().ok_or("Invalid reference ID")?;

                    let is_client_reference =
                        id.starts_with("$L") || id.starts_with("client") || !is_html_tag(id);

                    if is_client_reference {
                        let key = arr[2].as_str().map(|s| s.to_string());
                        let props = arr[3]
                            .as_object()
                            .ok_or("Invalid client reference props")?
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect();

                        Ok(RSCTree::ClientReference { id: id.to_string(), key, props })
                    } else {
                        let tag = id;
                        let key = arr[2].as_str().map(|s| s.to_string());
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
                if obj.contains_key("$$typeof") && obj.contains_key("type") {
                    let tag =
                        obj.get("type").and_then(|t| t.as_str()).ok_or("Invalid element type")?;

                    let key = obj.get("key").and_then(|k| k.as_str()).map(|s| s.to_string());

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

const HTML_TAGS: &[&str] = &[
    "div",
    "span",
    "p",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "a",
    "img",
    "ul",
    "ol",
    "li",
    "table",
    "tr",
    "td",
    "th",
    "thead",
    "tbody",
    "form",
    "input",
    "button",
    "select",
    "option",
    "textarea",
    "label",
    "br",
    "hr",
    "strong",
    "em",
    "b",
    "i",
    "small",
    "code",
    "pre",
    "blockquote",
    "article",
    "section",
    "header",
    "footer",
    "nav",
    "main",
    "aside",
    "figure",
    "figcaption",
    "details",
    "summary",
    "mark",
    "time",
    "progress",
    "meter",
    "audio",
    "video",
    "source",
    "track",
    "canvas",
    "svg",
    "iframe",
    "embed",
    "object",
    "param",
    "script",
    "noscript",
    "style",
    "link",
    "meta",
    "title",
    "head",
    "body",
    "html",
];

fn is_html_tag(tag: &str) -> bool {
    HTML_TAGS.contains(&tag)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RSCRenderResult {
    pub tree: RSCTree,
    pub has_suspense: bool,
    pub client_components: Vec<String>,
    pub debug: Option<RSCRenderDebug>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    pub fn with_suspense(mut self) -> Self {
        self.has_suspense = true;
        if let Some(debug) = &mut self.debug {
            debug.resolved_from_suspense = true;
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
        let json_value = json!(["$", "Counter", null, {"count": 42}]);
        let tree = RSCTree::from_json(&json_value).unwrap();

        match tree {
            RSCTree::ClientReference { id, key, props } => {
                assert_eq!(id, "Counter");
                assert_eq!(key, None);
                assert_eq!(props.get("count"), Some(&json!(42)));
            }
            _ => panic!("Expected ClientReference"),
        }
    }
}
