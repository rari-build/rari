use crate::error::RariError;
#[allow(unused_imports)]
use crate::rsc::elements::ReactElement;
use crate::rsc::rsc_tree::RSCTree;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
use serde_json::Value;
use tracing::error;

use smallvec::SmallVec;
use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Debug, Clone, PartialEq)]
pub enum ModuleReferenceType {
    ClientComponent,
}

#[derive(Debug, Clone)]
pub struct ModuleReference {
    pub id: String,
    pub path: String,
    pub reference_type: ModuleReferenceType,
    pub exports: SmallVec<[String; 3]>,
    pub metadata: FxHashMap<String, String>,
}

impl ModuleReference {
    pub fn new(id: String, path: String, reference_type: ModuleReferenceType) -> Self {
        Self { id, path, reference_type, exports: SmallVec::new(), metadata: FxHashMap::default() }
    }

    pub fn with_export(mut self, export: String) -> Self {
        self.exports.push(export);
        self
    }

    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
}

#[derive(Debug, Clone)]
pub struct PropValidationError {
    pub field_path: String,
    pub error_type: PropValidationErrorType,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum PropValidationErrorType {
    NonSerializable,
    CircularReference,
    FunctionFound,
    UnsupportedType,
}

pub struct RscSerializer {
    pub module_map: FxHashMap<String, ModuleReference>,
    pub chunk_counter: AtomicU32,
    pub row_counter: AtomicU32,
    pub output_lines: Vec<String>,
    serialized_modules: FxHashMap<String, String>,
    pub server_component_executor: Option<Box<dyn ServerComponentExecutor>>,
}

pub trait ServerComponentExecutor: Send + Sync {
    fn execute_server_component(
        &self,
        component_name: &str,
        props: Option<&FxHashMap<String, Value>>,
    ) -> Result<Value, RariError>;
}

#[derive(Debug, Clone)]
pub struct SerializedReactElement {
    pub element_type: ElementType,
    pub props: Option<FxHashMap<String, Value>>,
    pub key: Option<String>,
    pub ref_: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ElementType {
    HtmlTag(String),
    ClientComponent(String),
    ServerComponent(String),
    Text(String),
    Fragment,
}

impl Default for RscSerializer {
    fn default() -> Self {
        Self::new()
    }
}

impl RscSerializer {
    pub fn new() -> Self {
        Self {
            module_map: FxHashMap::default(),
            chunk_counter: AtomicU32::new(1),
            row_counter: AtomicU32::new(0),
            output_lines: Vec::new(),
            serialized_modules: FxHashMap::default(),
            server_component_executor: None,
        }
    }

    pub fn with_server_component_executor(
        mut self,
        executor: Box<dyn ServerComponentExecutor>,
    ) -> Self {
        self.server_component_executor = Some(executor);
        self
    }

    pub fn set_server_component_executor(&mut self, executor: Box<dyn ServerComponentExecutor>) {
        self.server_component_executor = Some(executor);
    }

    pub fn register_client_component(
        &mut self,
        component_id: &str,
        file_path: &str,
        export_name: &str,
    ) {
        let chunk_name = format!("client{}", self.chunk_counter.fetch_add(1, Ordering::Relaxed));

        let module_ref = ModuleReference::new(
            component_id.to_string(),
            file_path.to_string(),
            ModuleReferenceType::ClientComponent,
        )
        .with_export(export_name.to_string())
        .with_metadata("chunk", &chunk_name);

        self.module_map.insert(component_id.to_string(), module_ref);
    }

    pub fn is_client_component_registered(&self, component_id: &str) -> bool {
        self.module_map
            .get(component_id)
            .map(|module_ref| module_ref.reference_type == ModuleReferenceType::ClientComponent)
            .unwrap_or(false)
    }

    pub fn serialize_to_rsc_format(&mut self, element: &SerializedReactElement) -> String {
        self.output_lines.clear();
        self.serialized_modules.clear();

        self.add_module_import_lines();

        let element_id = self.get_next_row_id();
        let element_data = self.serialize_element_to_standard_format(element);
        let element_line = format!("{element_id}:{element_data}");
        self.output_lines.push(element_line);

        self.output_lines.join("\n")
    }

    pub fn serialize_rsc_tree(&mut self, tree: &RSCTree) -> String {
        self.output_lines.clear();
        self.serialized_modules.clear();

        self.collect_client_components_from_rsc_tree(tree);
        self.add_module_import_lines();

        let element_id = self.get_next_row_id();
        let element_data = self.serialize_rsc_tree_to_format(tree);
        let element_line = format!("{element_id}:{element_data}");
        self.output_lines.push(element_line);

        self.output_lines.join("\n")
    }

    pub fn serialize_rsc_json(&mut self, rsc_data: &serde_json::Value) -> Result<String, String> {
        let rsc_tree = crate::rsc::rsc_tree::RSCTree::from_json(rsc_data)
            .map_err(|e| format!("Failed to parse RSC tree from JSON: {e}"))?;

        Ok(self.serialize_rsc_tree(&rsc_tree))
    }

    fn collect_client_components_from_rsc_tree(&mut self, tree: &RSCTree) {
        match tree {
            RSCTree::ClientReference { id, .. } => {
                // Handle client component registration
                if id.contains('#') {
                    let parts: Vec<&str> = id.split('#').collect();
                    if parts.len() == 2 {
                        let file_path = parts[0];
                        let export_name = parts[1];
                        let component_name = file_path
                            .split('/')
                            .next_back()
                            .and_then(|filename| filename.split('.').next())
                            .unwrap_or("UnknownComponent");

                        if !self.is_client_component_registered(component_name) {
                            self.register_client_component(component_name, file_path, export_name);
                        }
                    }
                }
            }
            RSCTree::ServerElement { children: Some(children), .. } => {
                for child in children {
                    self.collect_client_components_from_rsc_tree(child);
                }
            }
            RSCTree::ServerElement { children: None, .. } => {
                // No children to process
            }
            RSCTree::Fragment { children, .. } => {
                for child in children {
                    self.collect_client_components_from_rsc_tree(child);
                }
            }
            RSCTree::Array(elements) => {
                for element in elements {
                    self.collect_client_components_from_rsc_tree(element);
                }
            }
            _ => {}
        }
    }

    fn serialize_rsc_tree_to_format(&mut self, tree: &RSCTree) -> String {
        match tree {
            RSCTree::ClientReference { id, key, props } => {
                self.serialize_client_reference_rsc(id, key.as_deref(), props)
            }
            RSCTree::ServerElement { tag, props, children, key } => {
                let normalized_tag = if tag == "react.suspense" { "react.suspense" } else { tag };
                self.serialize_server_element_rsc(normalized_tag, props, children, key.as_deref())
            }
            RSCTree::Text(content) => serde_json::to_string(content).unwrap_or_default(),
            RSCTree::Fragment { children, .. } => self.serialize_fragment_rsc(children),
            RSCTree::Array(elements) => self.serialize_array_rsc(elements),
            RSCTree::Null => "null".to_string(),
            RSCTree::Primitive(value) => serde_json::to_string(value).unwrap_or_default(),
            RSCTree::Error { message, component_name, .. } => {
                self.serialize_error_rsc(message, component_name)
            }
        }
    }

    fn serialize_client_reference_rsc(
        &mut self,
        id: &str,
        key: Option<&str>,
        props: &FxHashMap<String, Value>,
    ) -> String {
        let create_error_placeholder =
            |id: &str, key: Option<&str>, props: &FxHashMap<String, Value>| {
                let key_json = key
                    .map(|k| serde_json::to_string(k).unwrap_or_else(|_| "null".to_string()))
                    .unwrap_or_else(|| "null".to_string());

                let mut error_props = props.clone();
                error_props.insert(
                    "data-missing-client-component".to_string(),
                    serde_json::Value::String(id.to_string()),
                );
                error_props.insert(
                    "children".to_string(),
                    serde_json::Value::String(format!("Missing client component: {id}")),
                );

                let props_json =
                    serde_json::to_string(&error_props).unwrap_or_else(|_| "{}".to_string());
                format!(r#"["$","div",{key_json},{props_json}]"#)
            };

        let module_ref = if let Some(module_reference) = self.serialized_modules.get(id) {
            module_reference.clone()
        } else {
            let component_info = self.parse_and_register_component(id);
            match component_info {
                Some(component_name) => {
                    if let Some(module_reference) = self.serialized_modules.get(&component_name) {
                        module_reference.clone()
                    } else {
                        tracing::warn!(
                            "Client component '{}' has no registered module - treating as div placeholder",
                            id
                        );
                        return create_error_placeholder(id, key, props);
                    }
                }
                None => {
                    tracing::warn!(
                        "Client component '{}' has invalid format - treating as div placeholder",
                        id
                    );
                    return create_error_placeholder(id, key, props);
                }
            }
        };

        let key_json = key
            .map(|k| serde_json::to_string(k).unwrap_or_else(|_| "null".to_string()))
            .unwrap_or_else(|| "null".to_string());

        let props_json = serde_json::to_string(props).unwrap_or_else(|_| "{}".to_string());

        format!(r#"["$","{module_ref}",{key_json},{props_json}]"#)
    }

    fn parse_and_register_component(&mut self, id: &str) -> Option<String> {
        if !id.contains('#') {
            return None;
        }

        let parts: Vec<&str> = id.split('#').collect();
        if parts.len() != 2 {
            return None;
        }

        let file_path = parts[0];
        let export_name = parts[1];

        let component_name = file_path
            .split('/')
            .next_back()
            .and_then(|filename| filename.split('.').next())
            .unwrap_or("UnknownComponent");

        if let Some(_module_reference) = self.serialized_modules.get(component_name) {
            return Some(component_name.to_string());
        }

        if !self.is_client_component_registered(component_name) {
            self.register_client_component(component_name, file_path, export_name);
            if let Some(module_ref) = self.module_map.get(component_name).cloned() {
                self.emit_module_import_line(component_name, &module_ref);
                tracing::info!(
                    "Registered and emitted module import for client component: {} from {}",
                    component_name,
                    file_path
                );
            }
        }

        Some(component_name.to_string())
    }

    fn serialize_server_element_rsc(
        &mut self,
        tag: &str,
        props: &Option<FxHashMap<String, Value>>,
        children: &Option<Vec<RSCTree>>,
        key: Option<&str>,
    ) -> String {
        let mut element_props = props.clone().unwrap_or_default();

        let is_document_element = matches!(tag, "html" | "head" | "body");

        if tag == "react.suspense" {
            if let Some(fallback_value) = element_props.get("fallback") {
                return serde_json::to_string(fallback_value)
                    .unwrap_or_else(|_| "null".to_string());
            }
            return "null".to_string();
        }

        if let Some(children) = children {
            if children.len() == 1 {
                let child_data = self.serialize_rsc_tree_to_format(&children[0]);
                element_props.insert(
                    "children".to_string(),
                    serde_json::from_str(&child_data).unwrap_or(Value::String(child_data)),
                );
            } else if children.len() > 1 {
                let children_data: Vec<Value> = children
                    .iter()
                    .map(|child| {
                        let child_str = self.serialize_rsc_tree_to_format(child);
                        serde_json::from_str(&child_str).unwrap_or(Value::String(child_str))
                    })
                    .collect();
                element_props.insert("children".to_string(), Value::Array(children_data));
            }
        }

        let key_json = if is_document_element {
            "null".to_string()
        } else {
            key.map(|k| serde_json::to_string(k).unwrap_or_default()).unwrap_or("null".to_string())
        };

        let props_json = serde_json::to_string(&element_props).unwrap_or("{}".to_string());

        format!(r#"["$","{tag}",{key_json},{props_json}]"#)
    }

    fn serialize_fragment_rsc(&mut self, children: &[RSCTree]) -> String {
        let children_data: Vec<Value> = children
            .iter()
            .map(|child| {
                let child_str = self.serialize_rsc_tree_to_format(child);
                serde_json::from_str(&child_str).unwrap_or(Value::String(child_str))
            })
            .collect();

        serde_json::to_string(&children_data).unwrap_or("[]".to_string())
    }

    fn serialize_array_rsc(&mut self, elements: &[RSCTree]) -> String {
        let elements_data: Vec<Value> = elements
            .iter()
            .map(|element| {
                let element_str = self.serialize_rsc_tree_to_format(element);
                serde_json::from_str(&element_str).unwrap_or(Value::String(element_str))
            })
            .collect();

        serde_json::to_string(&elements_data).unwrap_or("[]".to_string())
    }

    fn serialize_error_rsc(&mut self, message: &str, component_name: &str) -> String {
        let error_element = format!(
            r#"["$","div",null,{{"style":{{"color":"red","border":"1px solid red","padding":"10px","margin":"10px"}},"children":[["$","h3",null,{{"children":"Error in {component_name}"}}],["$","p",null,{{"children":"{message}"}}]]}}]"#
        );
        error_element
    }

    fn get_next_row_id(&self) -> u32 {
        self.row_counter.fetch_add(1, Ordering::Relaxed)
    }

    fn add_module_import_lines(&mut self) {
        for (component_id, module_ref) in &self.module_map.clone() {
            if !self.serialized_modules.contains_key(component_id) {
                self.emit_module_import_line(component_id, module_ref);
            }
        }
    }

    #[allow(clippy::disallowed_methods)]
    fn emit_module_import_line(&mut self, component_id: &str, module_ref: &ModuleReference) {
        let module_id = self.get_next_row_id();

        let chunk_name = module_ref.metadata.get("chunk").map(|s| s.as_str()).unwrap_or("default");
        let export_name = module_ref.exports.first().map(|s| s.as_str()).unwrap_or("default");

        let module_data = serde_json::json!([module_ref.path, [chunk_name], export_name]);

        let import_line = format!("{module_id}:I{module_data}");
        self.output_lines.push(import_line);

        self.serialized_modules.insert(component_id.to_string(), format!("$L{module_id}"));
    }

    fn serialize_element_to_standard_format(&self, element: &SerializedReactElement) -> String {
        match &element.element_type {
            ElementType::HtmlTag(tag) => {
                self.serialize_html_element_standard(tag, element.props.as_ref())
            }
            ElementType::ClientComponent(component_id) => self
                .serialize_client_component_reference_standard(
                    component_id,
                    element.props.as_ref(),
                ),
            ElementType::ServerComponent(component_name) => {
                self.serialize_server_component_standard(component_name, element.props.as_ref())
            }
            ElementType::Text(text) => {
                serde_json::to_string(text).unwrap_or_else(|_| format!("\"{text}\""))
            }
            ElementType::Fragment => self.serialize_fragment_standard(element.props.as_ref()),
        }
    }

    fn create_react_element_json(
        &self,
        element_type: &str,
        props: Option<&FxHashMap<String, Value>>,
        key: Option<&str>,
    ) -> Value {
        let props_value = match props {
            Some(p) => {
                let serialized_props = self.serialize_props(p);
                if serialized_props.is_empty() {
                    Value::Null
                } else {
                    Value::Object(serialized_props.into_iter().collect())
                }
            }
            None => Value::Null,
        };

        let key_value = key.map(|k| Value::String(k.to_string())).unwrap_or(Value::Null);

        Value::Array(vec![
            Value::String("$".to_string()),
            Value::String(element_type.to_string()),
            key_value,
            props_value,
        ])
    }

    fn serialize_react_element_to_string(&self, element: &Value, fallback: &str) -> String {
        serde_json::to_string(element).unwrap_or_else(|_| fallback.to_string())
    }

    fn create_error_element(&self, error_message: &str, component_name: Option<&str>) -> Value {
        let mut error_props = serde_json::Map::new();
        let display_message = match component_name {
            Some(name) => format!("Error in {name}: {error_message}"),
            None => error_message.to_string(),
        };

        error_props.insert("children".to_string(), Value::String(display_message));
        error_props.insert(
            "style".to_string(),
            Value::Object({
                let mut style = serde_json::Map::new();
                style.insert("color".to_string(), Value::String("red".to_string()));
                style.insert("border".to_string(), Value::String("1px solid red".to_string()));
                style.insert("padding".to_string(), Value::String("10px".to_string()));
                style.insert("margin".to_string(), Value::String("10px".to_string()));
                style
            }),
        );

        Value::Array(vec![
            Value::String("$".to_string()),
            Value::String("div".to_string()),
            Value::Null,
            Value::Object(error_props),
        ])
    }

    fn serialize_html_element_standard(
        &self,
        tag: &str,
        props: Option<&FxHashMap<String, Value>>,
    ) -> String {
        let element = self.create_react_element_json(tag, props, None);
        self.serialize_react_element_to_string(&element, &format!("[\"$\",\"{tag}\",null,null]"))
    }

    fn serialize_client_component_reference_standard(
        &self,
        component_id: &str,
        props: Option<&FxHashMap<String, Value>>,
    ) -> String {
        if let Some(module_reference) = self.serialized_modules.get(component_id) {
            let element = self.create_react_element_json(module_reference, props, None);
            self.serialize_react_element_to_string(
                &element,
                &format!("[\"$\",\"{module_reference}\",null,null]"),
            )
        } else {
            format!(
                "[\"$\",\"div\",null,{{\"data-rsc-error\":\"Client component '{component_id}' not registered\"}}]"
            )
        }
    }

    #[allow(clippy::disallowed_methods)]
    fn serialize_server_component_standard(
        &self,
        component_name: &str,
        props: Option<&FxHashMap<String, Value>>,
    ) -> String {
        let executor = match &self.server_component_executor {
            Some(executor) => executor,
            None => {
                let error_message = format!("Error rendering {component_name}");
                let error_element = self.create_error_element(&error_message, None);
                return self.serialize_react_element_to_string(
                    &error_element,
                    &format!("[\"$\",\"div\",null,{{\"children\":\"{error_message}\"}}]"),
                );
            }
        };

        match executor.execute_server_component(component_name, props) {
            Ok(executed_result) => {
                self.handle_server_component_result(component_name, executed_result, props)
            }
            Err(_e) => {
                let error_message = format!("Error rendering {component_name}");
                let error_element = self.create_error_element(&error_message, None);
                self.serialize_react_element_to_string(
                    &error_element,
                    &format!("[\"$\",\"div\",null,{{\"children\":\"{error_message}\"}}]"),
                )
            }
        }
    }

    fn handle_server_component_result(
        &self,
        component_name: &str,
        result: Value,
        props: Option<&FxHashMap<String, Value>>,
    ) -> String {
        match result {
            Value::Array(ref arr) if arr.len() >= 3 && arr[0] == "$" => self
                .serialize_react_element_to_string(
                    &result,
                    &self.create_error_fallback_standard(component_name, "Serialization failed"),
                ),
            Value::Object(ref obj) if obj.contains_key("type") && obj.contains_key("props") => {
                let element_type = obj.get("type").and_then(|t| t.as_str()).unwrap_or("div");
                let element_props = obj.get("props").cloned().unwrap_or(Value::Null);

                let element = Value::Array(vec![
                    Value::String("$".to_string()),
                    Value::String(element_type.to_string()),
                    Value::Null,
                    element_props,
                ]);

                self.serialize_react_element_to_string(
                    &element,
                    &self.create_error_fallback_standard(component_name, "Serialization failed"),
                )
            }
            _ => self.create_execution_placeholder_standard(component_name, props),
        }
    }

    fn create_error_fallback_standard(&self, component_name: &str, error_message: &str) -> String {
        let error_element = self.create_error_element(error_message, Some(component_name));
        self.serialize_react_element_to_string(&error_element, &format!("[\"$\",\"div\",null,{{\"children\":\"Error in {component_name}: {error_message}\"}}]"))
    }

    fn create_execution_placeholder_standard(
        &self,
        component_name: &str,
        props: Option<&FxHashMap<String, Value>>,
    ) -> String {
        let mut placeholder_props = serde_json::Map::new();
        placeholder_props
            .insert("children".to_string(), Value::String(format!("Component: {component_name}")));

        if let Some(props_map) = props {
            let serialized_props = self.serialize_props(props_map);
            placeholder_props.insert(
                "data-component-props".to_string(),
                Value::Object(serialized_props.into_iter().collect()),
            );
        }

        let placeholder_element = Value::Array(vec![
            Value::String("$".to_string()),
            Value::String("div".to_string()),
            Value::Null,
            Value::Object(placeholder_props),
        ]);

        serde_json::to_string(&placeholder_element).unwrap_or_else(|_| {
            "[\"$\",\"div\",null,{\"children\":\"Component placeholder\"}]".to_string()
        })
    }

    fn serialize_fragment_standard(&self, props: Option<&FxHashMap<String, Value>>) -> String {
        let children =
            props.and_then(|p| p.get("children")).cloned().unwrap_or(Value::Array(vec![]));

        serde_json::to_string(&children).unwrap_or_else(|_| "[]".to_string())
    }

    fn serialize_props(&self, props: &FxHashMap<String, Value>) -> FxHashMap<String, Value> {
        let mut result = FxHashMap::default();
        let mut visited = FxHashSet::default();
        let mut validation_errors = Vec::new();

        for (key, value) in props {
            match Self::validate_and_serialize_prop(
                key,
                value,
                &mut visited,
                &mut validation_errors,
            ) {
                Ok(validated_value) => {
                    result.insert(key.clone(), validated_value);
                }
                Err(_) => {
                    error!("[RSC] Prop validation error for '{key}': {validation_errors:?}");
                    result.insert(key.clone(), Value::Null);
                }
            }
            visited.clear();
        }

        if !validation_errors.is_empty() {
            error!("[RSC] Props validation completed with {} errors", validation_errors.len());
            for error in &validation_errors {
                error!("[RSC] Validation error: {} - {}", error.field_path, error.message);
            }
        }

        result
    }

    fn is_likely_function_string(s: &str) -> bool {
        if s.contains("<") || s.contains("&lt;") {
            return false;
        }

        if s.len() > 500 {
            return false;
        }

        (s.starts_with("function") && s.contains("(") && s.contains(")"))
            || (s.starts_with("(") && s.contains("=>") && s.len() < 100)
            || (s.starts_with("async function") && s.contains("("))
    }

    fn validate_and_serialize_prop(
        field_path: &str,
        value: &Value,
        visited: &mut FxHashSet<*const Value>,
        errors: &mut Vec<PropValidationError>,
    ) -> Result<Value, PropValidationError> {
        let value_ptr = value as *const Value;
        if visited.contains(&value_ptr) {
            let error = PropValidationError {
                field_path: field_path.to_string(),
                error_type: PropValidationErrorType::CircularReference,
                message: format!("Circular reference detected in prop '{field_path}'"),
            };
            errors.push(error.clone());
            return Err(error);
        }

        match value {
            Value::String(s) if Self::is_likely_function_string(s) => {
                let error = PropValidationError {
                    field_path: field_path.to_string(),
                    error_type: PropValidationErrorType::FunctionFound,
                    message: format!("Functions are not serializable in RSC props: '{field_path}'"),
                };
                errors.push(error.clone());
                Err(error)
            }

            Value::Object(obj) => {
                visited.insert(value_ptr);

                let mut validated_object = serde_json::Map::new();
                for (key, nested_value) in obj {
                    let nested_path = if field_path.is_empty() {
                        key.clone()
                    } else {
                        format!("{field_path}.{key}")
                    };

                    match Self::validate_and_serialize_prop(
                        &nested_path,
                        nested_value,
                        visited,
                        errors,
                    ) {
                        Ok(validated_nested) => {
                            validated_object.insert(key.clone(), validated_nested);
                        }
                        Err(_) => {
                            validated_object.insert(key.clone(), Value::Null);
                        }
                    }
                }

                visited.remove(&value_ptr);
                Ok(Value::Object(validated_object))
            }

            Value::Array(arr) => {
                visited.insert(value_ptr);

                let mut validated_array = Vec::new();
                for (index, item) in arr.iter().enumerate() {
                    let item_path = format!("{field_path}[{index}]");
                    match Self::validate_and_serialize_prop(&item_path, item, visited, errors) {
                        Ok(validated_item) => {
                            validated_array.push(validated_item);
                        }
                        Err(_) => {
                            validated_array.push(Value::Null);
                        }
                    }
                }

                visited.remove(&value_ptr);
                Ok(Value::Array(validated_array))
            }

            Value::Null | Value::Bool(_) | Value::Number(_) => Ok(value.clone()),

            Value::String(s) => {
                if s.contains("Symbol(") || s.contains("Object [object") {
                    let error = PropValidationError {
                        field_path: field_path.to_string(),
                        error_type: PropValidationErrorType::NonSerializable,
                        message: format!(
                            "Non-serializable content detected in prop '{field_path}': {s}"
                        ),
                    };
                    errors.push(error.clone());
                    return Err(error);
                }
                Ok(value.clone())
            }
        }
    }

    #[allow(clippy::disallowed_methods)]
    pub fn emit_suspense_boundary(
        &mut self,
        fallback: &SerializedReactElement,
        boundary_id: &str,
    ) -> String {
        let boundary_row_id = self.get_next_row_id();

        let boundary_data = serde_json::json!([
            "$",
            "react.suspense",
            null,
            {
                "fallback": self.serialize_element_to_standard_format(fallback),
                "children": format!("@{}", boundary_id)
            }
        ]);

        let boundary_line = format!("{boundary_row_id}:{boundary_data}");
        self.output_lines.push(boundary_line);

        boundary_id.to_string()
    }

    pub fn emit_streamed_content(
        &mut self,
        boundary_id: &str,
        content: &SerializedReactElement,
    ) -> String {
        let content_data = self.serialize_element_to_standard_format(content);
        let content_line = format!("{boundary_id}:{content_data}");
        self.output_lines.push(content_line.clone());
        content_line
    }

    pub fn serialize_element(
        &mut self,
        element: &crate::rsc::elements::ReactElement,
    ) -> Result<String, RariError> {
        if element.tag == "react.suspense" {
            let fallback = element
                .props
                .get("fallback")
                .ok_or_else(|| RariError::internal("Suspense missing fallback prop"))?;

            let children = element
                .props
                .get("children")
                .ok_or_else(|| RariError::internal("Suspense missing children prop"))?;

            let boundary_id =
                element.props.get("__boundary_id").and_then(|v| v.as_str()).unwrap_or("default");

            let fallback_element: crate::rsc::elements::ReactElement =
                serde_json::from_value(fallback.clone()).map_err(|e| {
                    RariError::internal(format!("Failed to parse Suspense fallback: {}", e))
                })?;

            let children_element: crate::rsc::elements::ReactElement =
                serde_json::from_value(children.clone()).map_err(|e| {
                    RariError::internal(format!("Failed to parse Suspense children: {}", e))
                })?;

            let fallback_ref = self.serialize_element(&fallback_element)?;
            let children_ref = self.serialize_element(&children_element)?;

            self.emit_suspense_boundary_with_refs(&fallback_ref, &children_ref, boundary_id)
        } else {
            self.serialize_regular_element(element)
        }
    }

    fn serialize_regular_element(
        &mut self,
        element: &crate::rsc::elements::ReactElement,
    ) -> Result<String, RariError> {
        let element_id = self.get_next_row_id();

        let key_json = element
            .key
            .as_ref()
            .map(|k| serde_json::to_string(k).unwrap_or_else(|_| "null".to_string()))
            .unwrap_or_else(|| "null".to_string());

        let props_json = serde_json::to_string(&element.props).unwrap_or_else(|_| "{}".to_string());

        let element_data = format!(r#"["$","{}",{},{}]"#, element.tag, key_json, props_json);

        let element_line = format!("{element_id}:{element_data}");
        self.output_lines.push(element_line);

        Ok(format!("$L{}", element_id))
    }

    pub fn emit_suspense_boundary_with_refs(
        &mut self,
        fallback_ref: &str,
        children_ref: &str,
        boundary_id: &str,
    ) -> Result<String, RariError> {
        let boundary_row_id = self.get_next_row_id();

        #[allow(clippy::disallowed_methods)]
        let boundary_data = serde_json::json!([
            "$",
            "react.suspense",
            null,
            {
                "fallback": fallback_ref,
                "children": children_ref,
                "__boundary_id": boundary_id
            }
        ]);

        let boundary_line = format!(
            "{}:{}",
            boundary_row_id,
            serde_json::to_string(&boundary_data).map_err(|e| RariError::internal(format!(
                "Failed to serialize Suspense boundary: {}",
                e
            )))?
        );

        self.output_lines.push(boundary_line);

        Ok(format!("$L{}", boundary_row_id))
    }
}

impl SerializedReactElement {
    pub fn create_html_element(
        tag: &str,
        props: Option<FxHashMap<String, Value>>,
    ) -> SerializedReactElement {
        SerializedReactElement {
            element_type: ElementType::HtmlTag(tag.to_string()),
            props,
            key: None,
            ref_: None,
        }
    }

    pub fn create_client_component(
        component_id: &str,
        props: Option<FxHashMap<String, Value>>,
    ) -> SerializedReactElement {
        SerializedReactElement {
            element_type: ElementType::ClientComponent(component_id.to_string()),
            props,
            key: None,
            ref_: None,
        }
    }

    pub fn create_server_component(
        component_name: &str,
        props: Option<FxHashMap<String, Value>>,
    ) -> SerializedReactElement {
        SerializedReactElement {
            element_type: ElementType::ServerComponent(component_name.to_string()),
            props,
            key: None,
            ref_: None,
        }
    }

    pub fn create_text_element(text: &str) -> SerializedReactElement {
        SerializedReactElement {
            element_type: ElementType::Text(text.to_string()),
            props: None,
            key: None,
            ref_: None,
        }
    }

    pub fn create_fragment(children: Option<Vec<Value>>) -> SerializedReactElement {
        let mut props = FxHashMap::default();
        if let Some(children_vec) = children {
            props.insert("children".to_string(), Value::Array(children_vec));
        }

        SerializedReactElement {
            element_type: ElementType::Fragment,
            props: Some(props),
            key: None,
            ref_: None,
        }
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_serialize_html_element() {
        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert("className".to_string(), json!("test-class"));
        props.insert("children".to_string(), json!("Hello World"));

        let element = SerializedReactElement::create_html_element("div", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains(r#"["$","div",null,"#));
        assert!(result.contains("Hello World"));
    }

    #[test]
    fn test_serialize_client_component() {
        let mut serializer = RscSerializer::new();

        serializer.register_client_component("Button", "./components/Button.client.js", "default");

        let mut props = FxHashMap::default();
        props.insert("onClick".to_string(), json!("handleClick"));
        props.insert("children".to_string(), json!("Click me"));

        let element = SerializedReactElement::create_client_component("Button", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("./components/Button.client.js"));
        assert!(result.contains("$"));
        assert!(result.contains("Click me"));

        assert!(result.contains(":I"));
    }

    #[test]
    fn test_serialize_text_element() {
        let mut serializer = RscSerializer::new();

        let element = SerializedReactElement::create_text_element("Hello, RSC!");
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("Hello, RSC!"));
    }

    #[test]
    fn test_serialize_server_component() {
        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert("userId".to_string(), json!(123));

        let element = SerializedReactElement::create_server_component("UserProfile", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("UserProfile"));
    }

    #[test]
    fn test_serialize_fragment() {
        let mut serializer = RscSerializer::new();

        let children = vec![json!("First child"), json!("Second child")];

        let element = SerializedReactElement::create_fragment(Some(children));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("First child"));
        assert!(result.contains("Second child"));
    }

    #[test]
    fn test_module_deduplication() {
        let mut serializer = RscSerializer::new();

        serializer.register_client_component("Button", "./components/Button.client.js", "default");

        let element1 = SerializedReactElement::create_client_component("Button", None);
        let _result1 = serializer.serialize_to_rsc_format(&element1);

        serializer.output_lines.clear();
        serializer.serialized_modules.clear();

        let element2 = SerializedReactElement::create_client_component("Button", None);
        let result2 = serializer.serialize_to_rsc_format(&element2);

        assert!(result2.contains("./components/Button.client.js"));
    }

    #[test]
    fn test_complex_nested_structure() {
        let mut serializer = RscSerializer::new();

        serializer.register_client_component("Button", "./components/Button.client.js", "default");

        let mut button_props = FxHashMap::default();
        button_props.insert("children".to_string(), json!("Click me"));

        let mut div_props = FxHashMap::default();
        div_props.insert("className".to_string(), json!("container"));

        let _button_element =
            SerializedReactElement::create_client_component("Button", Some(button_props));
        let div_element = SerializedReactElement::create_html_element("div", Some(div_props));

        let result = serializer.serialize_to_rsc_format(&div_element);

        assert!(result.contains(r#"["$","div""#));
        assert!(result.contains("container"));
    }

    #[test]
    fn test_unregistered_client_component() {
        let mut serializer = RscSerializer::new();

        let element = SerializedReactElement::create_client_component("UnknownButton", None);
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("data-rsc-error"));
        assert!(result.contains("UnknownButton"));
    }

    #[test]
    fn test_empty_props() {
        let mut serializer = RscSerializer::new();

        let element = SerializedReactElement::create_html_element("br", None);
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains(r#"["$","br",null,null]"#));
    }

    #[test]
    fn test_complex_props_serialization() {
        let mut serializer = RscSerializer::new();

        let mut complex_props = FxHashMap::default();
        complex_props.insert("valid_string".to_string(), json!("Hello"));
        complex_props.insert("valid_number".to_string(), json!(42));
        complex_props.insert("valid_boolean".to_string(), json!(true));
        complex_props.insert("valid_null".to_string(), json!(null));
        complex_props.insert(
            "nested_object".to_string(),
            json!({
                "inner": "value",
                "count": 10
            }),
        );
        complex_props.insert("array_prop".to_string(), json!([1, 2, 3]));

        let element = SerializedReactElement::create_html_element("div", Some(complex_props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("Hello"));
        assert!(result.contains("42"));
        assert!(result.contains("true"));
        assert!(result.contains("inner"));
        assert!(result.contains("value"));
    }

    #[test]
    fn test_props_validation_function_rejection() {
        let mut serializer = RscSerializer::new();

        let mut props_with_function = FxHashMap::default();
        props_with_function
            .insert("onClick".to_string(), json!("function handleClick() { return true; }"));
        props_with_function.insert("valid_prop".to_string(), json!("valid value"));

        let element =
            SerializedReactElement::create_html_element("button", Some(props_with_function));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("null"));
        assert!(result.contains("valid value"));
        assert!(!result.contains("handleClick"));
    }

    #[test]
    fn test_props_validation_circular_reference() {
        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert("safe_prop".to_string(), json!("safe"));

        props.insert(
            "nested".to_string(),
            json!({
                "level1": {
                    "level2": {
                        "data": "deep value"
                    }
                }
            }),
        );

        let element = SerializedReactElement::create_html_element("div", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("safe"));
        assert!(result.contains("deep value"));
    }

    #[test]
    fn test_props_validation_non_serializable_detection() {
        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert("symbol_prop".to_string(), json!("Symbol(test)"));
        props.insert("object_prop".to_string(), json!("Object [object Object]"));
        props.insert("valid_prop".to_string(), json!("normal string"));

        let element = SerializedReactElement::create_html_element("div", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("normal string"));
        assert!(!result.contains("Symbol(test)"));
        assert!(!result.contains("Object [object"));
    }

    struct MockServerComponentExecutor;

    impl ServerComponentExecutor for MockServerComponentExecutor {
        fn execute_server_component(
            &self,
            component_name: &str,
            _props: Option<&FxHashMap<String, Value>>,
        ) -> Result<Value, RariError> {
            match component_name {
                "SuccessfulComponent" => {
                    Ok(json!(["$", "h1", null, {"children": "Server rendered content"}]))
                }
                "HTMLComponent" => Ok(json!("<p>HTML from server</p>")),
                "FailingComponent" => {
                    Err(RariError::js_execution("Component execution failed".to_string()))
                }
                _ => Ok(
                    json!({"type": "div", "props": {"children": format!("Component: {}", component_name)}}),
                ),
            }
        }
    }

    #[test]
    fn test_server_component_execution_successful() {
        let mut serializer = RscSerializer::new();
        serializer.set_server_component_executor(Box::new(MockServerComponentExecutor));

        let element = SerializedReactElement::create_server_component("SuccessfulComponent", None);
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("Server rendered content"));
        assert!(result.contains(r#"["$","h1",null"#));
    }

    #[test]
    fn test_server_component_execution_html_result() {
        let mut serializer = RscSerializer::new();
        serializer.set_server_component_executor(Box::new(MockServerComponentExecutor));

        let element = SerializedReactElement::create_server_component("HTMLComponent", None);
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("HTMLComponent"));
        assert!(result.contains(r#"["$","div",null"#));
    }

    #[test]
    fn test_server_component_execution_failure() {
        let mut serializer = RscSerializer::new();
        serializer.set_server_component_executor(Box::new(MockServerComponentExecutor));

        let element = SerializedReactElement::create_server_component("FailingComponent", None);
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("Error rendering FailingComponent"));
        assert!(result.contains(r#"["$","div",null"#));
    }

    #[test]
    fn test_server_component_no_executor() {
        let mut serializer = RscSerializer::new();

        let element = SerializedReactElement::create_server_component("TestComponent", None);
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("Error rendering TestComponent"));
        assert!(result.contains(r#"["$","div",null"#));
    }

    #[test]
    fn test_server_component_with_props() {
        let mut serializer = RscSerializer::new();
        serializer.set_server_component_executor(Box::new(MockServerComponentExecutor));

        let mut props = FxHashMap::default();
        props.insert("title".to_string(), json!("Test Title"));
        props.insert("count".to_string(), json!(5));

        let element =
            SerializedReactElement::create_server_component("GenericComponent", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("GenericComponent"));
        assert!(result.contains("Component: GenericComponent"));
    }

    #[test]
    fn test_serialize_element_with_suspense() {
        use crate::rsc::elements::ReactElement as LoadingReactElement;

        let mut serializer = RscSerializer::new();

        let mut fallback_props = FxHashMap::default();
        fallback_props.insert("children".to_string(), json!("Loading..."));
        let fallback = LoadingReactElement::with_props("div", fallback_props);

        let mut children_props = FxHashMap::default();
        children_props.insert("children".to_string(), json!("Content loaded"));
        let children = LoadingReactElement::with_props("div", children_props);

        let mut suspense_props = FxHashMap::default();
        suspense_props.insert("fallback".to_string(), serde_json::to_value(&fallback).unwrap());
        suspense_props.insert("children".to_string(), serde_json::to_value(&children).unwrap());
        suspense_props.insert("__boundary_id".to_string(), json!("test-boundary"));

        let suspense = LoadingReactElement::with_props("react.suspense", suspense_props);

        let result = serializer.serialize_element(&suspense).unwrap();

        assert!(result.starts_with("$L"), "Should return a reference to the Suspense boundary");

        let output = serializer.output_lines.join("\n");
        assert!(output.contains("react.suspense"), "Should contain Suspense tag");
        assert!(output.contains("test-boundary"), "Should contain boundary ID");
        assert!(output.contains("Loading..."), "Should contain fallback content");
        assert!(output.contains("Content loaded"), "Should contain children content");
    }

    #[test]
    fn test_serialize_element_regular() {
        use crate::rsc::elements::ReactElement as LoadingReactElement;

        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert("className".to_string(), json!("test-class"));
        props.insert("children".to_string(), json!("Hello World"));

        let element = LoadingReactElement::with_props("div", props).with_key("test-key");

        let result = serializer.serialize_element(&element).unwrap();

        assert!(result.starts_with("$L"), "Should return a reference");

        let output = serializer.output_lines.join("\n");
        assert!(output.contains(r#"["$","div""#), "Should contain div element");
        assert!(output.contains("test-class"), "Should contain className prop");
        assert!(output.contains("Hello World"), "Should contain children");
        assert!(output.contains("test-key"), "Should contain key");
    }

    #[test]
    fn test_emit_suspense_boundary_with_refs() {
        let mut serializer = RscSerializer::new();

        let result =
            serializer.emit_suspense_boundary_with_refs("$L1", "$L2", "boundary-123").unwrap();

        assert!(result.starts_with("$L"), "Should return a reference");

        let output = serializer.output_lines.join("\n");
        assert!(output.contains("react.suspense"), "Should contain Suspense tag");
        assert!(output.contains(r#""fallback":"$L1""#), "Should reference fallback");
        assert!(output.contains(r#""children":"$L2""#), "Should reference children");
        assert!(output.contains("boundary-123"), "Should contain boundary ID");
    }

    #[test]
    fn test_suspense_wire_format_structure() {
        use crate::rsc::elements::ReactElement as LoadingReactElement;

        let mut serializer = RscSerializer::new();

        let fallback = LoadingReactElement::with_props("div", {
            let mut props = FxHashMap::default();
            props.insert("className".to_string(), json!("loading-spinner"));
            props.insert("children".to_string(), json!("Loading..."));
            props
        });

        let children = LoadingReactElement::with_props("article", {
            let mut props = FxHashMap::default();
            props.insert("className".to_string(), json!("content"));
            props.insert("children".to_string(), json!("Article content"));
            props
        });

        let suspense = LoadingReactElement::with_props("react.suspense", {
            let mut props = FxHashMap::default();
            props.insert("fallback".to_string(), serde_json::to_value(&fallback).unwrap());
            props.insert("children".to_string(), serde_json::to_value(&children).unwrap());
            props.insert("__boundary_id".to_string(), json!("article-boundary"));
            props
        });

        let _result = serializer.serialize_element(&suspense).unwrap();

        let output = serializer.output_lines.join("\n");

        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 3, "Should have 3 rows in wire format");

        assert!(lines[0].contains(r#"["$","div""#), "First row should be fallback div");
        assert!(lines[0].contains("loading-spinner"), "Should contain fallback className");

        assert!(lines[1].contains(r#"["$","article""#), "Second row should be children article");
        assert!(lines[1].contains("content"), "Should contain children className");

        assert!(lines[2].contains("react.suspense"), "Third row should be Suspense boundary");
        assert!(lines[2].contains("$L0"), "Should reference fallback with $L0");
        assert!(lines[2].contains("$L1"), "Should reference children with $L1");
        assert!(lines[2].contains("article-boundary"), "Should contain boundary ID");
    }

    #[test]
    fn test_suspense_missing_fallback_error() {
        use crate::rsc::elements::ReactElement as LoadingReactElement;

        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert("children".to_string(), json!({"tag": "div", "props": {}}));
        props.insert("__boundary_id".to_string(), json!("test"));

        let suspense = LoadingReactElement::with_props("react.suspense", props);

        let result = serializer.serialize_element(&suspense);
        assert!(result.is_err(), "Should error when fallback is missing");
        assert!(
            result.unwrap_err().to_string().contains("fallback"),
            "Error should mention missing fallback"
        );
    }

    #[test]
    fn test_suspense_missing_children_error() {
        use crate::rsc::elements::ReactElement as LoadingReactElement;

        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert("fallback".to_string(), json!({"tag": "div", "props": {}}));
        props.insert("__boundary_id".to_string(), json!("test"));

        let suspense = LoadingReactElement::with_props("react.suspense", props);

        let result = serializer.serialize_element(&suspense);
        assert!(result.is_err(), "Should error when children is missing");
        assert!(
            result.unwrap_err().to_string().contains("children"),
            "Error should mention missing children"
        );
    }
}
