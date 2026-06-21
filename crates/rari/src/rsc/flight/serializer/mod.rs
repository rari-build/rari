use crate::rsc::flight::escape::escape_rsc_value;
use crate::rsc::rendering::streaming::types::RscWireFormatTag;
use crate::rsc::types::tree::RSCTree;
use cow_utils::CowUtils;
use rari_error::RariError;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
use serde_json::Value;
use tracing::error;

use smallvec::SmallVec;
use std::sync::atomic::{AtomicU32, Ordering};

fn base64_encode(bytes: &[u8]) -> String {
    use base64::{Engine, engine::general_purpose::STANDARD};
    STANDARD.encode(bytes)
}

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
    pub row_counter: AtomicU32,
    pub output_lines: Vec<String>,
    serialized_modules: FxHashMap<String, String>,
    pub server_component_executor: Option<Box<dyn ServerComponentExecutor>>,
    suspense_symbol_row_id: Option<u32>,
    pub pending_lazy_promises: Vec<LazyPromiseInfo>,
    pub seen_lazy_promise_ids: FxHashSet<String>,
    module_registration_order: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LazyPromiseInfo {
    pub promise_id: String,
    pub lazy_row_id: u32,
    pub component_id: String,
    pub loading_id: String,
}

#[derive(Debug, Clone)]
struct LazyMarker {
    promise_id: String,
    component_id: String,
    loading_id: String,
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
            row_counter: AtomicU32::new(0),
            output_lines: Vec::new(),
            serialized_modules: FxHashMap::default(),
            server_component_executor: None,
            suspense_symbol_row_id: None,
            pending_lazy_promises: Vec::new(),
            seen_lazy_promise_ids: FxHashSet::default(),
            module_registration_order: Vec::new(),
        }
    }

    pub fn with_server_component_executor(
        mut self,
        executor: Box<dyn ServerComponentExecutor>,
    ) -> Self {
        self.server_component_executor = Some(executor);
        self
    }

    #[cfg(test)]
    pub fn set_server_component_executor(&mut self, executor: Box<dyn ServerComponentExecutor>) {
        self.server_component_executor = Some(executor);
    }

    pub fn clear_module_state(&mut self) {
        self.serialized_modules.clear();
    }

    pub fn reset_for_new_request(&mut self) {
        self.serialized_modules.clear();
        self.module_registration_order.clear();
        self.module_map.clear();
        self.row_counter.store(0, Ordering::Relaxed);
    }

    pub fn register_client_component(
        &mut self,
        component_id: &str,
        file_path: &str,
        export_name: &str,
    ) {
        let chunk_name = "main".to_string();

        let normalized_component_id = component_id.cow_replace('\\', "/").into_owned();
        let normalized_file_path = file_path.cow_replace('\\', "/").into_owned();

        let module_ref = ModuleReference::new(
            normalized_component_id.clone(),
            normalized_file_path,
            ModuleReferenceType::ClientComponent,
        )
        .with_export(export_name.to_string())
        .with_metadata("chunk", &chunk_name);

        if !self.module_map.contains_key(&normalized_component_id) {
            self.module_registration_order.push(normalized_component_id.clone());
        }

        self.module_map.insert(normalized_component_id, module_ref);
    }

    pub fn is_client_component_registered(&self, component_id: &str) -> bool {
        let normalized_component_id = component_id.cow_replace('\\', "/");
        self.module_map
            .get(normalized_component_id.as_ref())
            .map(|module_ref| module_ref.reference_type == ModuleReferenceType::ClientComponent)
            .unwrap_or(false)
    }

    pub fn serialize_to_rsc_format(&mut self, element: &SerializedReactElement) -> String {
        self.output_lines.clear();
        self.serialized_modules.clear();

        let _reserved_row_0 = self.get_next_row_id();

        self.add_module_import_lines();

        let element_id = self.get_next_row_id();
        let element_data = self.serialize_element_to_standard_format(element);
        let element_line = format!("{:x}:{}", element_id, element_data);
        self.output_lines.push(element_line);

        let root_ref = format!("0:\"${:x}\"", element_id);
        self.output_lines.insert(0, root_ref);

        self.output_lines.join("\n")
    }

    pub fn serialize_rsc_tree(&mut self, tree: &RSCTree) -> String {
        self.output_lines.clear();
        self.serialized_modules.clear();
        self.module_registration_order.clear();

        let _reserved_row_0 = self.get_next_row_id();

        self.collect_client_components_from_rsc_tree(tree);
        self.add_module_import_lines();

        let element_id = self.get_next_row_id();
        let element_data = self.serialize_rsc_tree_to_format(tree);
        let element_line = format!("{:x}:{}", element_id, element_data);
        self.output_lines.push(element_line);

        let root_ref = format!("0:\"${:x}\"", element_id);
        self.output_lines.insert(0, root_ref);

        self.output_lines.join("\n")
    }

    pub fn serialize_rsc_json(&mut self, rsc_data: &serde_json::Value) -> Result<String, String> {
        let rsc_tree = crate::rsc::types::tree::RSCTree::from_json(rsc_data)
            .map_err(|e| format!("Failed to parse RSC tree from JSON: {e}"))?;

        let has_suspense = self.tree_contains_suspense(&rsc_tree);

        let current_counter = self.row_counter.load(Ordering::Relaxed);
        let is_lazy_resolution = current_counter > 0;

        self.output_lines.clear();

        if !is_lazy_resolution {
            let _reserved_row_0 = self.get_next_row_id();

            self.serialized_modules.clear();
            self.module_registration_order.clear();
        }

        self.collect_client_components_from_rsc_tree(&rsc_tree);

        let suspense_symbol_row_id = if has_suspense {
            let row_id = self.get_next_row_id();
            let symbol_line = format!("{:x}:\"$Sreact.suspense\"", row_id);
            self.output_lines.push(symbol_line);
            self.suspense_symbol_row_id = Some(row_id);
            Some(row_id)
        } else {
            None
        };

        self.add_module_import_lines();

        let element_id = self.get_next_row_id();
        let element_data = self.serialize_rsc_tree_to_format(&rsc_tree);
        let element_line = format!("{:x}:{}", element_id, element_data);
        self.output_lines.push(element_line);

        if !is_lazy_resolution {
            let root_ref = format!("0:\"${:x}\"", element_id);
            self.output_lines.insert(0, root_ref);
        }

        if let Some(row_id) = suspense_symbol_row_id {
            let searches = vec!["\"$Sreact.suspense\"", "\"react.suspense\""];
            let replace = format!("\"${:x}\"", row_id);
            let symbol_declaration = format!("{:x}:\"$Sreact.suspense\"", row_id);

            for i in 1..self.output_lines.len() {
                if self.output_lines[i] == symbol_declaration {
                    continue;
                }

                for search in &searches {
                    if self.output_lines[i].contains(search) {
                        self.output_lines[i] =
                            self.output_lines[i].cow_replace(search, &replace).into_owned();
                    }
                }
            }
        }

        Ok(self.output_lines.join("\n"))
    }

    fn tree_contains_suspense(&self, tree: &RSCTree) -> bool {
        fn check_tree(tree: &RSCTree) -> bool {
            match tree {
                RSCTree::ServerElement { tag, children, .. } => {
                    if tag == "$Sreact.suspense" || tag == "react.suspense" {
                        return true;
                    }
                    if let Some(children) = children {
                        for child in children {
                            if check_tree(child) {
                                return true;
                            }
                        }
                    }
                    false
                }
                RSCTree::Fragment { children, .. } | RSCTree::Array(children) => {
                    children.iter().any(check_tree)
                }
                _ => false,
            }
        }
        check_tree(tree)
    }

    fn collect_client_components_from_rsc_tree(&mut self, tree: &RSCTree) {
        match tree {
            RSCTree::ClientReference { id, .. }
                if id.contains('#') && !self.is_client_component_registered(id) =>
            {
                let normalized_id = id.cow_replace('\\', "/");
                let id = normalized_id.as_ref();

                let parts: Vec<&str> = id.split('#').collect();
                if parts.len() == 2 {
                    let file_path = parts[0];
                    let export_name = parts[1];
                    self.register_client_component(id, file_path, export_name);
                }
            }
            RSCTree::ServerElement { children: Some(children), .. } => {
                for child in children {
                    self.collect_client_components_from_rsc_tree(child);
                }
            }
            RSCTree::ServerElement { children: None, .. } => {}
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
            RSCTree::Primitive(Value::Object(obj))
                if obj.get("~rari_lazy").and_then(|v| v.as_bool()) == Some(true) => {}
            _ => {}
        }
    }

    fn serialize_rsc_tree_to_format(&mut self, tree: &RSCTree) -> String {
        match tree {
            RSCTree::ClientReference { id, key, props } => {
                self.serialize_client_reference_rsc(id, key.as_deref(), props)
            }
            RSCTree::ServerElement { tag, props, children, key } => {
                self.serialize_server_element_rsc(tag, props, children, key.as_deref())
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
        let normalized_id = id.cow_replace('\\', "/");
        let id = normalized_id.as_ref();

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

                let props_value = Value::Object(
                    error_props.into_iter().collect::<serde_json::Map<String, Value>>(),
                );
                let escaped_props = escape_rsc_value(&props_value);
                let props_json =
                    serde_json::to_string(&escaped_props).unwrap_or_else(|_| "{}".to_string());
                format!(r#"["$","div",{key_json},{props_json}]"#)
            };

        let module_ref = if let Some(module_reference) = self.serialized_modules.get(id) {
            module_reference.clone()
        } else {
            let component_info = self.parse_and_register_component(id);
            match component_info {
                Some(component_name) => {
                    if let Some(module_ref) = self.module_map.get(&component_name).cloned() {
                        self.emit_module_import_line(&component_name, &module_ref);
                        if let Some(module_reference) = self.serialized_modules.get(&component_name)
                        {
                            module_reference.clone()
                        } else {
                            return create_error_placeholder(id, key, props);
                        }
                    } else {
                        return create_error_placeholder(id, key, props);
                    }
                }
                None => {
                    return create_error_placeholder(id, key, props);
                }
            }
        };

        let key_json = key
            .map(|k| serde_json::to_string(k).unwrap_or_else(|_| "null".to_string()))
            .unwrap_or_else(|| "null".to_string());

        let processed_props: FxHashMap<String, Value> = props
            .iter()
            .map(|(k, v)| {
                let processed_value = self.process_prop_value(v);
                (k.clone(), processed_value)
            })
            .collect();

        let props_value =
            Value::Object(processed_props.into_iter().collect::<serde_json::Map<String, Value>>());
        let escaped_props = escape_rsc_value(&props_value);
        let props_json = serde_json::to_string(&escaped_props).unwrap_or_else(|_| "{}".to_string());

        format!(r#"["$","{module_ref}",{key_json},{props_json}]"#)
    }

    fn process_prop_value(&mut self, value: &Value) -> Value {
        if let Some(arr) = value.as_array() {
            if arr.len() == 4
                && arr.first().and_then(|v| v.as_str()) == Some("$")
                && let Ok(rsc_tree) = RSCTree::from_json(value)
            {
                let serialized = self.serialize_rsc_tree_to_format(&rsc_tree);
                if let Ok(parsed) = serde_json::from_str::<Value>(&serialized) {
                    return parsed;
                }
                return Value::String(serialized);
            }
            let processed: Vec<Value> = arr.iter().map(|v| self.process_prop_value(v)).collect();
            return Value::Array(processed);
        }

        if let Some(obj) = value.as_object() {
            let processed: serde_json::Map<String, Value> =
                obj.iter().map(|(k, v)| (k.clone(), self.process_prop_value(v))).collect();
            return Value::Object(processed);
        }

        value.clone()
    }

    fn parse_and_register_component(&mut self, id: &str) -> Option<String> {
        let normalized_id = id.cow_replace('\\', "/");
        let id = normalized_id.as_ref();

        if !id.contains('#') {
            return None;
        }

        if let Some(_module_reference) = self.serialized_modules.get(id) {
            return Some(id.to_string());
        }

        let parts: Vec<&str> = id.split('#').collect();
        if parts.len() != 2 {
            return None;
        }

        let file_path = parts[0].cow_replace('\\', "/");
        let export_name = parts[1];

        if !self.is_client_component_registered(id) {
            self.register_client_component(id, &file_path, export_name);
        }

        if !self.serialized_modules.contains_key(id)
            && let Some(module_ref) = self.module_map.get(id).cloned()
        {
            self.emit_module_import_line(id, &module_ref);
        }

        Some(id.to_string())
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

        if let Some(children) = children {
            if children.len() == 1 {
                if let Some(lazy_info) = self.extract_lazy_marker(&children[0]) {
                    if self.seen_lazy_promise_ids.contains(&lazy_info.promise_id) {
                        if let Some(original) = self
                            .pending_lazy_promises
                            .iter()
                            .find(|p| p.promise_id == lazy_info.promise_id)
                        {
                            element_props.insert(
                                "children".to_string(),
                                Value::String(format!("${:x}", original.lazy_row_id)),
                            );
                        } else {
                            element_props.insert("children".to_string(), Value::Null);
                        }
                    } else {
                        self.seen_lazy_promise_ids.insert(lazy_info.promise_id.clone());

                        let lazy_row_id = self.get_next_row_id();

                        self.pending_lazy_promises.push(LazyPromiseInfo {
                            promise_id: lazy_info.promise_id.clone(),
                            lazy_row_id,
                            component_id: lazy_info.component_id.clone(),
                            loading_id: lazy_info.loading_id.clone(),
                        });

                        element_props.insert(
                            "children".to_string(),
                            Value::String(format!("${:x}", lazy_row_id)),
                        );
                    }
                } else {
                    let child_data = self.serialize_rsc_tree_to_format(&children[0]);
                    element_props.insert(
                        "children".to_string(),
                        serde_json::from_str(&child_data).unwrap_or(Value::String(child_data)),
                    );
                }
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
            key.map(|k| serde_json::to_string(k).unwrap_or_else(|_| "null".to_string()))
                .unwrap_or_else(|| "null".to_string())
        };

        let props_value =
            Value::Object(element_props.into_iter().collect::<serde_json::Map<String, Value>>());
        let escaped_props = escape_rsc_value(&props_value);
        let props_json = serde_json::to_string(&escaped_props).unwrap_or_else(|_| "{}".to_string());

        format!(r#"["$","{tag}",{key_json},{props_json}]"#)
    }

    fn extract_lazy_marker(&self, tree: &RSCTree) -> Option<LazyMarker> {
        if let RSCTree::Primitive(Value::Object(obj)) = tree
            && obj.get("~rari_lazy").and_then(|v| v.as_bool()) == Some(true)
        {
            let promise_id = obj.get("~rari_promise_id")?.as_str()?.to_string();
            let component_id = obj.get("~rari_component_id")?.as_str()?.to_string();
            let loading_id = obj.get("~rari_loading_id")?.as_str()?.to_string();

            return Some(LazyMarker { promise_id, component_id, loading_id });
        }
        None
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
        for component_id in self.module_registration_order.clone() {
            if !self.serialized_modules.contains_key(&component_id)
                && let Some(module_ref) = self.module_map.get(&component_id).cloned()
            {
                self.emit_module_import_line(&component_id, &module_ref);
            }
        }
    }

    #[allow(clippy::disallowed_methods)]
    fn emit_module_import_line(&mut self, component_id: &str, module_ref: &ModuleReference) {
        let module_id = self.get_next_row_id();

        let export_name = module_ref.exports.first().map(|s| s.as_str()).unwrap_or("default");

        let module_data = serde_json::json!({
            "id": module_ref.path,
            "chunks": [],
            "name": export_name
        });

        let import_line =
            RscWireFormatTag::ModuleImport.format_row(module_id, &module_data.to_string());
        self.output_lines.push(import_line.trim_end().to_string());

        self.serialized_modules.insert(component_id.to_string(), format!("$L{:x}", module_id));
    }

    fn serialize_element_to_standard_format(&mut self, element: &SerializedReactElement) -> String {
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
        &mut self,
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
        &mut self,
        tag: &str,
        props: Option<&FxHashMap<String, Value>>,
    ) -> String {
        let element = self.create_react_element_json(tag, props, None);
        self.serialize_react_element_to_string(&element, &format!("[\"$\",\"{tag}\",null,null]"))
    }

    fn serialize_client_component_reference_standard(
        &mut self,
        component_id: &str,
        props: Option<&FxHashMap<String, Value>>,
    ) -> String {
        let normalized_component_id = component_id.cow_replace('\\', "/");

        if let Some(module_reference) =
            self.serialized_modules.get(normalized_component_id.as_ref()).cloned()
        {
            let element = self.create_react_element_json(&module_reference, props, None);
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
        &mut self,
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
        &mut self,
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
        &mut self,
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

    fn serialize_props(&mut self, props: &FxHashMap<String, Value>) -> FxHashMap<String, Value> {
        let mut result = FxHashMap::default();
        let mut visited = FxHashSet::default();
        let mut validation_errors = Vec::new();

        for (key, value) in props {
            let processed_value = self.process_special_values_with_outlining(value);

            match Self::validate_and_serialize_prop(
                key,
                &processed_value,
                &mut visited,
                &mut validation_errors,
            ) {
                Ok(validated_value) => {
                    result.insert(key.clone(), validated_value);
                }
                Err(_) => {
                    error!("[rari] RSC: Prop validation error for '{key}': {validation_errors:?}");
                    result.insert(key.clone(), Value::Null);
                }
            }
            visited.clear();
        }

        if !validation_errors.is_empty() {
            error!(
                "[rari] RSC: Props validation completed with {} errors",
                validation_errors.len()
            );
            for error in &validation_errors {
                error!("[rari] RSC: Validation error: {} - {}", error.field_path, error.message);
            }
        }

        result
    }

    fn process_special_values_with_outlining(&mut self, value: &Value) -> Value {
        match value {
            Value::Number(n) => {
                if let Some(f) = n.as_f64() {
                    if f.is_nan() {
                        return Value::String("$NaN".to_string());
                    } else if f.is_infinite() {
                        if f.is_sign_positive() {
                            return Value::String("$Infinity".to_string());
                        } else {
                            return Value::String("$-Infinity".to_string());
                        }
                    } else if f == 0.0 && f.is_sign_negative() {
                        return Value::String("$-0".to_string());
                    }
                }
                value.clone()
            }

            Value::Object(obj) => {
                if let Some(date_str) = obj.get("$date").and_then(|v| v.as_str()) {
                    return Value::String(format!("$D{}", date_str));
                }

                if let Some(bigint_str) = obj.get("$bigint").and_then(|v| v.as_str()) {
                    return Value::String(format!("$n{}", bigint_str));
                }

                if let Some(map_entries) = obj.get("$map") {
                    return self.outline_map(map_entries);
                }

                if let Some(set_entries) = obj.get("$set") {
                    return self.outline_set(set_entries);
                }

                if let Some(formdata_entries) = obj.get("$formdata") {
                    return self.outline_formdata(formdata_entries);
                }

                if let Some(promise_data) = obj.get("$promise") {
                    return self.outline_promise(promise_data);
                }

                if let Some(function_data) = obj.get("$function") {
                    return self.outline_server_function(function_data);
                }

                if let Some(temp_ref) = obj.get("$temp").and_then(|v| v.as_str()) {
                    return Value::String(format!("$T{}", temp_ref));
                }

                if let Some(symbol_name) = obj.get("$symbol").and_then(|v| v.as_str()) {
                    return Value::String(format!("$S{}", symbol_name));
                }

                if let Some(deferred_data) = obj.get("$deferred") {
                    return self.outline_deferred(deferred_data);
                }

                if let Some(iterator_data) = obj.get("$iterator") {
                    return self.outline_iterator(iterator_data);
                }

                if let Some(typedarray_data) = obj.get("$typedarray") {
                    return self.outline_typedarray(typedarray_data);
                }

                if let Some(blob_data) = obj.get("$blob") {
                    return self.outline_blob(blob_data);
                }

                if let Some(stream_data) = obj.get("$stream") {
                    return self.outline_stream(stream_data);
                }

                let mut processed_obj = serde_json::Map::new();
                for (k, v) in obj {
                    processed_obj.insert(k.clone(), self.process_special_values_with_outlining(v));
                }
                Value::Object(processed_obj)
            }

            Value::Array(arr) => Value::Array(
                arr.iter().map(|v| self.process_special_values_with_outlining(v)).collect(),
            ),

            Value::String(s) if s == "$undefined" => value.clone(),

            _ => value.clone(),
        }
    }

    fn outline_map(&mut self, entries: &Value) -> Value {
        let chunk_id = self.get_next_row_id();

        let processed_entries = self.process_special_values_with_outlining(entries);

        let entries_json =
            serde_json::to_string(&processed_entries).unwrap_or_else(|_| "[]".to_string());
        let chunk_line = format!("{:x}:{}", chunk_id, entries_json);
        self.output_lines.push(chunk_line);

        Value::String(format!("$Q{:x}", chunk_id))
    }

    fn outline_set(&mut self, entries: &Value) -> Value {
        let chunk_id = self.get_next_row_id();

        let processed_entries = self.process_special_values_with_outlining(entries);

        let entries_json =
            serde_json::to_string(&processed_entries).unwrap_or_else(|_| "[]".to_string());
        let chunk_line = format!("{:x}:{}", chunk_id, entries_json);
        self.output_lines.push(chunk_line);

        Value::String(format!("$W{:x}", chunk_id))
    }

    fn outline_formdata(&mut self, entries: &Value) -> Value {
        let chunk_id = self.get_next_row_id();

        let processed_entries = self.process_special_values_with_outlining(entries);

        let entries_json =
            serde_json::to_string(&processed_entries).unwrap_or_else(|_| "[]".to_string());
        let chunk_line = format!("{:x}:{}", chunk_id, entries_json);
        self.output_lines.push(chunk_line);

        Value::String(format!("$K{:x}", chunk_id))
    }

    fn outline_promise(&mut self, promise_data: &Value) -> Value {
        let chunk_id = self.get_next_row_id();

        let processed_data = self.process_special_values_with_outlining(promise_data);

        let data_json =
            serde_json::to_string(&processed_data).unwrap_or_else(|_| "null".to_string());
        let chunk_line = format!("{:x}:{}", chunk_id, data_json);
        self.output_lines.push(chunk_line);

        Value::String(format!("$@{:x}", chunk_id))
    }

    fn outline_server_function(&mut self, function_data: &Value) -> Value {
        let chunk_id = self.get_next_row_id();

        let processed_data = self.process_special_values_with_outlining(function_data);

        let data_json = serde_json::to_string(&processed_data).unwrap_or_else(|_| "{}".to_string());
        let chunk_line = format!("{:x}:{}", chunk_id, data_json);
        self.output_lines.push(chunk_line);

        Value::String(format!("$F{:x}", chunk_id))
    }

    fn outline_deferred(&mut self, deferred_data: &Value) -> Value {
        let chunk_id = self.get_next_row_id();

        let processed_data = self.process_special_values_with_outlining(deferred_data);

        let data_json =
            serde_json::to_string(&processed_data).unwrap_or_else(|_| "null".to_string());
        let chunk_line = format!("{:x}:{}", chunk_id, data_json);
        self.output_lines.push(chunk_line);

        Value::String(format!("$Y{:x}", chunk_id))
    }

    fn outline_iterator(&mut self, iterator_data: &Value) -> Value {
        let chunk_id = self.get_next_row_id();

        let processed_data = self.process_special_values_with_outlining(iterator_data);

        let data_json = serde_json::to_string(&processed_data).unwrap_or_else(|_| "[]".to_string());
        let chunk_line = format!("{:x}:{}", chunk_id, data_json);
        self.output_lines.push(chunk_line);

        Value::String(format!("$i{:x}", chunk_id))
    }

    fn outline_typedarray(&mut self, typedarray_data: &Value) -> Value {
        let chunk_id = self.get_next_row_id();

        let type_name =
            typedarray_data.get("type").and_then(|v| v.as_str()).unwrap_or("Uint8Array");

        let data = typedarray_data.get("data").and_then(|v| v.as_array());

        if let Some(data_array) = data {
            let tag = match type_name {
                "ArrayBuffer" => "A",
                "Int8Array" => "O",
                "Uint8Array" => "o",
                "Uint8ClampedArray" => "U",
                "Int16Array" => "S",
                "Uint16Array" => "s",
                "Int32Array" => "L",
                "Uint32Array" => "l",
                "Float32Array" => "G",
                "Float64Array" => "g",
                "BigInt64Array" => "M",
                "BigUint64Array" => "m",
                "DataView" => "V",
                _ => "o",
            };

            let bytes: Vec<u8> =
                data_array.iter().filter_map(|v| v.as_u64().map(|n| n as u8)).collect();

            let base64_data = base64_encode(&bytes);
            let chunk_line = format!("{:x}:{}{:x},{}", chunk_id, tag, bytes.len(), base64_data);
            self.output_lines.push(chunk_line);

            Value::String(format!("${:x}", chunk_id))
        } else {
            Value::Null
        }
    }

    #[allow(clippy::disallowed_methods)]
    fn outline_blob(&mut self, blob_data: &Value) -> Value {
        let chunk_id = self.get_next_row_id();

        let blob_type =
            blob_data.get("type").and_then(|v| v.as_str()).unwrap_or("application/octet-stream");

        let data = blob_data.get("data").and_then(|v| v.as_array());

        if let Some(data_array) = data {
            let bytes: Vec<u8> =
                data_array.iter().filter_map(|v| v.as_u64().map(|n| n as u8)).collect();

            let base64_data = base64_encode(&bytes);
            let blob_model = serde_json::json!([blob_type, base64_data]);
            let blob_json = serde_json::to_string(&blob_model).unwrap_or_else(|_| "[]".to_string());
            let chunk_line = format!("{:x}:{}", chunk_id, blob_json);
            self.output_lines.push(chunk_line);

            Value::String(format!("$B{:x}", chunk_id))
        } else {
            Value::Null
        }
    }

    fn outline_stream(&mut self, stream_data: &Value) -> Value {
        let chunk_id = self.get_next_row_id();

        let is_byte_stream =
            stream_data.get("byteStream").and_then(|v| v.as_bool()).unwrap_or(false);

        let chunks = stream_data.get("chunks").and_then(|v| v.as_array());

        if let Some(chunks_array) = chunks {
            let start_tag = if is_byte_stream { "r" } else { "R" };
            let start_line = format!("{:x}:{}", chunk_id, start_tag);
            self.output_lines.push(start_line);

            for chunk in chunks_array {
                let processed_chunk = self.process_special_values_with_outlining(chunk);
                let chunk_json = serde_json::to_string(&processed_chunk).unwrap_or_default();
                let chunk_line = format!("{:x}:{}", chunk_id, chunk_json);
                self.output_lines.push(chunk_line);
            }

            let complete_line = format!("{:x}:C", chunk_id);
            self.output_lines.push(complete_line);

            Value::String(format!("${:x}", chunk_id))
        } else {
            Value::Null
        }
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
            "$Sreact.suspense",
            null,
            {
                "fallback": self.serialize_element_to_standard_format(fallback),
                "children": format!("@{}", boundary_id)
            }
        ]);

        let boundary_line = format!("{:x}:{}", boundary_row_id, boundary_data);
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
        element: &crate::rsc::types::elements::ReactElement,
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
                element.props.get("~boundaryId").and_then(|v| v.as_str()).unwrap_or("default");

            let fallback_element: crate::rsc::types::elements::ReactElement =
                serde_json::from_value(fallback.clone()).map_err(|e| {
                    RariError::internal(format!("Failed to parse Suspense fallback: {}", e))
                })?;

            let children_element: crate::rsc::types::elements::ReactElement =
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
        element: &crate::rsc::types::elements::ReactElement,
    ) -> Result<String, RariError> {
        let element_id = self.get_next_row_id();

        let key_json = element
            .key
            .as_ref()
            .map(|k| serde_json::to_string(k).unwrap_or_else(|_| "null".to_string()))
            .unwrap_or_else(|| "null".to_string());

        let props_json = serde_json::to_string(&element.props).unwrap_or_else(|_| "{}".to_string());

        let element_data = format!(r#"["$","{}",{},{}]"#, element.tag, key_json, props_json);

        let element_line = format!("{:x}:{}", element_id, element_data);
        self.output_lines.push(element_line);

        Ok(format!("$L{:x}", element_id))
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
            "$Sreact.suspense",
            null,
            {
                "fallback": fallback_ref,
                "children": children_ref,
                "~boundaryId": boundary_id
            }
        ]);

        let boundary_line = format!(
            "{:x}:{}",
            boundary_row_id,
            serde_json::to_string(&boundary_data).map_err(|e| RariError::internal(format!(
                "Failed to serialize Suspense boundary: {}",
                e
            )))?
        );

        self.output_lines.push(boundary_line);

        Ok(format!("$L{:x}", boundary_row_id))
    }
}

impl SerializedReactElement {
    #[cfg(test)]
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

    #[cfg(test)]
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

    #[cfg(test)]
    pub fn create_text_element(text: &str) -> SerializedReactElement {
        SerializedReactElement {
            element_type: ElementType::Text(text.to_string()),
            props: None,
            key: None,
            ref_: None,
        }
    }

    #[cfg(test)]
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
#[allow(clippy::module_inception)]
mod tests {
    use super::*;
    use crate::rsc::ServerComponentExecutor;
    use rari_error::RariError;
    use rustc_hash::FxHashMap;
    use serde_json::Value;
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
        serializer.serialize_to_rsc_format(&element1);

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

        let mut div_props = FxHashMap::default();
        div_props.insert("className".to_string(), json!("container"));

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

    #[allow(dead_code)]
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
        use crate::rsc::types::elements::ReactElement as LoadingReactElement;

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
        suspense_props.insert("~boundaryId".to_string(), json!("test-boundary"));

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
        use crate::rsc::types::elements::ReactElement as LoadingReactElement;

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
    fn test_serialize_element_regular_hex_reference() {
        use crate::rsc::types::elements::ReactElement as LoadingReactElement;

        let mut serializer = RscSerializer::new();

        for _ in 0..10 {
            serializer.get_next_row_id();
        }

        let mut props = FxHashMap::default();
        props.insert("children".to_string(), json!("Test Content"));

        let element = LoadingReactElement::with_props("span", props);

        let result = serializer.serialize_element(&element).unwrap();

        assert_eq!(result, "$La", "Reference should use hexadecimal format");

        let output = serializer.output_lines.join("\n");
        assert!(output.contains("a:["), "Wire format row should use hex ID 'a'");
        assert!(output.contains("Test Content"), "Should contain the element content");

        let element2 = LoadingReactElement::with_props("div", FxHashMap::default());
        let result2 = serializer.serialize_element(&element2).unwrap();
        assert_eq!(result2, "$Lb", "Next reference should be $Lb");

        for _ in 0..4 {
            serializer.get_next_row_id();
        }
        let element3 = LoadingReactElement::with_props("p", FxHashMap::default());
        let result3 = serializer.serialize_element(&element3).unwrap();
        assert_eq!(result3, "$L10", "Reference for ID 16 should be $L10 in hex");
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
    fn test_serialize_map_object() {
        use serde_json::json;

        let mut serializer = RscSerializer::new();
        let value = json!({"$map": [["key1", "value1"], ["key2", "value2"]]});
        let processed = serializer.process_special_values_with_outlining(&value);

        assert!(
            processed.as_str().unwrap().starts_with("$Q"),
            "Expected $Q prefix, got: {:?}",
            processed
        );

        assert_eq!(processed, "$Q0");

        assert!(!serializer.output_lines.is_empty(), "Expected output lines to be emitted");
        assert_eq!(serializer.output_lines.len(), 1);

        let chunk = &serializer.output_lines[0];

        assert!(chunk.starts_with("0:"));
        assert!(chunk.contains("key1"));
        assert!(chunk.contains("value1"));
        assert!(chunk.contains("key2"));
        assert!(chunk.contains("value2"));
    }

    #[test]
    fn test_serialize_set_object() {
        use serde_json::json;

        let mut serializer = RscSerializer::new();
        let value = json!({"$set": ["value1", "value2", "value3"]});
        let processed = serializer.process_special_values_with_outlining(&value);

        assert!(processed.as_str().unwrap().starts_with("$W"));
        assert_eq!(processed, "$W0");

        assert!(!serializer.output_lines.is_empty());
        assert_eq!(serializer.output_lines.len(), 1);

        let chunk = &serializer.output_lines[0];
        assert!(chunk.starts_with("0:"));
        assert!(chunk.contains("value1"));
        assert!(chunk.contains("value2"));
        assert!(chunk.contains("value3"));
    }

    #[test]
    fn test_serialize_formdata_object() {
        use serde_json::json;

        let mut serializer = RscSerializer::new();
        let value = json!({"$formdata": [["field1", "value1"], ["field2", "value2"]]});
        let processed = serializer.process_special_values_with_outlining(&value);

        assert!(processed.as_str().unwrap().starts_with("$K"));
        assert_eq!(processed, "$K0");

        assert!(!serializer.output_lines.is_empty());
        assert_eq!(serializer.output_lines.len(), 1);

        let chunk = &serializer.output_lines[0];
        assert!(chunk.starts_with("0:"));
        assert!(chunk.contains("field1"));
        assert!(chunk.contains("value1"));
        assert!(chunk.contains("field2"));
        assert!(chunk.contains("value2"));
    }

    #[test]
    fn test_serialize_nested_collections() {
        use serde_json::json;

        let mut serializer = RscSerializer::new();
        let value = json!({
            "map": {"$map": [["key1", "value1"]]},
            "set": {"$set": ["item1", "item2"]},
            "nested": {
                "formdata": {"$formdata": [["field", "value"]]}
            }
        });

        let processed = serializer.process_special_values_with_outlining(&value);

        assert_eq!(serializer.output_lines.len(), 3);

        assert!(processed["map"].as_str().unwrap().starts_with("$Q"));
        assert!(processed["set"].as_str().unwrap().starts_with("$W"));
        assert!(processed["nested"]["formdata"].as_str().unwrap().starts_with("$K"));
    }

    #[test]
    fn test_serialize_map_with_special_values() {
        use serde_json::json;

        let mut serializer = RscSerializer::new();
        let value = json!({
            "$map": [
                ["date", {"$date": "2025-12-09T18:00:00.000Z"}],
                ["bigint", {"$bigint": "123"}],
                ["nan", "$NaN"]
            ]
        });

        let processed = serializer.process_special_values_with_outlining(&value);

        assert!(processed.as_str().unwrap().starts_with("$Q"));
        assert_eq!(serializer.output_lines.len(), 1);

        let chunk = &serializer.output_lines[0];

        assert!(chunk.contains("$D2025-12-09T18:00:00.000Z"));
        assert!(chunk.contains("$n123"));
        assert!(chunk.contains("$NaN"));
    }

    #[test]
    fn test_serialize_multiple_maps_unique_ids() {
        use serde_json::json;

        let mut serializer = RscSerializer::new();
        let value = json!({
            "map1": {"$map": [["a", "1"]]},
            "map2": {"$map": [["b", "2"]]},
            "map3": {"$map": [["c", "3"]]}
        });

        let processed = serializer.process_special_values_with_outlining(&value);

        assert_eq!(serializer.output_lines.len(), 3);

        let ref1 = processed["map1"].as_str().unwrap();
        let ref2 = processed["map2"].as_str().unwrap();
        let ref3 = processed["map3"].as_str().unwrap();

        assert!(ref1.starts_with("$Q"));
        assert!(ref2.starts_with("$Q"));
        assert!(ref3.starts_with("$Q"));

        assert_ne!(ref1, ref2);
        assert_ne!(ref2, ref3);
        assert_ne!(ref1, ref3);
    }

    #[test]
    fn test_serialize_promise_object() {
        use serde_json::json;

        let mut serializer = RscSerializer::new();
        let value = json!({"$promise": {"status": "pending", "value": null}});
        let processed = serializer.process_special_values_with_outlining(&value);

        assert!(processed.as_str().unwrap().starts_with("$@"));
        assert_eq!(processed, "$@0");

        assert!(!serializer.output_lines.is_empty());
        assert_eq!(serializer.output_lines.len(), 1);

        let chunk = &serializer.output_lines[0];
        assert!(chunk.starts_with("0:"));
        assert!(chunk.contains("pending"));
    }

    #[test]
    fn test_serialize_server_function() {
        use serde_json::json;

        let mut serializer = RscSerializer::new();
        let value = json!({
            "$function": {
                "id": "actions/todo-actions#addTodo",
                "bound": null
            }
        });
        let processed = serializer.process_special_values_with_outlining(&value);

        assert!(processed.as_str().unwrap().starts_with("$F"));
        assert_eq!(processed, "$F0");

        assert!(!serializer.output_lines.is_empty());
        let chunk = &serializer.output_lines[0];
        assert!(chunk.contains("addTodo"));
    }

    #[test]
    fn test_serialize_temporary_reference() {
        use serde_json::json;

        let mut serializer = RscSerializer::new();
        let value = json!({"$temp": "ref_123"});
        let processed = serializer.process_special_values_with_outlining(&value);

        assert_eq!(processed, "$Tref_123");

        assert!(serializer.output_lines.is_empty());
    }

    #[test]
    fn test_serialize_symbol_reference() {
        use serde_json::json;

        let mut serializer = RscSerializer::new();
        let value = json!({"$symbol": "iterator"});
        let processed = serializer.process_special_values_with_outlining(&value);

        assert_eq!(processed, "$Siterator");

        assert!(serializer.output_lines.is_empty());
    }

    #[test]
    fn test_serialize_deferred_object() {
        use serde_json::json;

        let mut serializer = RscSerializer::new();
        let value = json!({"$deferred": {"type": "debug", "data": "some data"}});
        let processed = serializer.process_special_values_with_outlining(&value);

        assert!(processed.as_str().unwrap().starts_with("$Y"));
        assert_eq!(processed, "$Y0");

        assert!(!serializer.output_lines.is_empty());
        let chunk = &serializer.output_lines[0];
        assert!(chunk.contains("debug"));
    }

    #[test]
    fn test_serialize_iterator_object() {
        use serde_json::json;

        let mut serializer = RscSerializer::new();
        let value = json!({"$iterator": ["value1", "value2", "value3"]});
        let processed = serializer.process_special_values_with_outlining(&value);

        assert!(processed.as_str().unwrap().starts_with("$i"));
        assert_eq!(processed, "$i0");

        assert!(!serializer.output_lines.is_empty());
        let chunk = &serializer.output_lines[0];
        assert!(chunk.contains("value1"));
        assert!(chunk.contains("value2"));
    }

    #[test]
    fn test_serialize_mixed_advanced_markers() {
        use serde_json::json;

        let mut serializer = RscSerializer::new();
        let value = json!({
            "promise": {"$promise": {"status": "fulfilled", "value": 42}},
            "function": {"$function": {"id": "myAction"}},
            "temp": {"$temp": "temp_ref"},
            "symbol": {"$symbol": "toStringTag"},
            "deferred": {"$deferred": {"data": "lazy"}},
            "iterator": {"$iterator": [1, 2, 3]}
        });

        let processed = serializer.process_special_values_with_outlining(&value);

        assert!(processed["promise"].as_str().unwrap().starts_with("$@"));
        assert!(processed["function"].as_str().unwrap().starts_with("$F"));
        assert_eq!(processed["temp"], "$Ttemp_ref");
        assert_eq!(processed["symbol"], "$StoStringTag");
        assert!(processed["deferred"].as_str().unwrap().starts_with("$Y"));
        assert!(processed["iterator"].as_str().unwrap().starts_with("$i"));

        assert_eq!(serializer.output_lines.len(), 4);
    }

    #[test]
    fn test_serialize_nested_advanced_markers() {
        use serde_json::json;

        let mut serializer = RscSerializer::new();
        let value = json!({
            "$promise": {
                "status": "fulfilled",
                "value": {
                    "map": {"$map": [["key", "value"]]},
                    "date": {"$date": "2025-12-09T18:00:00.000Z"}
                }
            }
        });

        let processed = serializer.process_special_values_with_outlining(&value);

        assert!(processed.as_str().unwrap().starts_with("$@"));

        assert_eq!(serializer.output_lines.len(), 2);

        let promise_chunk = serializer
            .output_lines
            .iter()
            .find(|line| line.contains("fulfilled"))
            .expect("Promise chunk not found");

        assert!(promise_chunk.contains("$Q"));
        assert!(promise_chunk.contains("$D"));
    }

    #[test]
    fn test_suspense_wire_format_structure() {
        use crate::rsc::types::elements::ReactElement as LoadingReactElement;

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
            props.insert("~boundaryId".to_string(), json!("article-boundary"));
            props
        });

        serializer.serialize_element(&suspense).unwrap();

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
        use crate::rsc::types::elements::ReactElement as LoadingReactElement;

        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert("children".to_string(), json!({"tag": "div", "props": {}}));
        props.insert("~boundaryId".to_string(), json!("test"));

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
        use crate::rsc::types::elements::ReactElement as LoadingReactElement;

        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert("fallback".to_string(), json!({"tag": "div", "props": {}}));
        props.insert("~boundaryId".to_string(), json!("test"));

        let suspense = LoadingReactElement::with_props("react.suspense", props);

        let result = serializer.serialize_element(&suspense);
        assert!(result.is_err(), "Should error when children is missing");
        assert!(
            result.unwrap_err().to_string().contains("children"),
            "Error should mention missing children"
        );
    }

    #[test]
    fn test_serialize_typedarray_uint8() {
        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert(
            "buffer".to_string(),
            json!({
                "$typedarray": {
                    "type": "Uint8Array",
                    "data": [1, 2, 3, 4, 5]
                }
            }),
        );

        let element = SerializedReactElement::create_html_element("div", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("$"), "Should contain by-value reference");

        assert!(result.contains(":o"), "Should contain Uint8Array tag");
    }

    #[test]
    fn test_serialize_typedarray_int32() {
        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert(
            "data".to_string(),
            json!({
                "$typedarray": {
                    "type": "Int32Array",
                    "data": [100, 200, 300]
                }
            }),
        );

        let element = SerializedReactElement::create_html_element("div", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("$"), "Should contain by-value reference");

        assert!(result.contains(":L"), "Should contain Int32Array tag");
    }

    #[test]
    fn test_serialize_typedarray_float64() {
        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert(
            "floats".to_string(),
            json!({
                "$typedarray": {
                    "type": "Float64Array",
                    "data": [1, 2, 3]
                }
            }),
        );

        let element = SerializedReactElement::create_html_element("div", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("$"), "Should contain by-value reference");

        assert!(result.contains(":g"), "Should contain Float64Array tag");
    }

    #[test]
    fn test_serialize_blob() {
        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert(
            "file".to_string(),
            json!({
                "$blob": {
                    "type": "image/png",
                    "data": [137, 80, 78, 71]
                }
            }),
        );

        let element = SerializedReactElement::create_html_element("div", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("$B"), "Should contain Blob reference");

        assert!(result.contains("image/png"), "Should contain blob type");
    }

    #[test]
    fn test_serialize_blob_default_type() {
        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert(
            "data".to_string(),
            json!({
                "$blob": {
                    "data": [1, 2, 3, 4]
                }
            }),
        );

        let element = SerializedReactElement::create_html_element("div", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("$B"), "Should contain Blob reference");

        assert!(result.contains("application/octet-stream"), "Should use default blob type");
    }

    #[test]
    fn test_serialize_readable_stream() {
        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert(
            "stream".to_string(),
            json!({
                "$stream": {
                    "chunks": [
                        [1, 2, 3],
                        [4, 5, 6],
                        [7, 8, 9]
                    ]
                }
            }),
        );

        let element = SerializedReactElement::create_html_element("div", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("$"), "Should contain by-value reference");

        assert!(result.contains(":R"), "Should contain stream start marker");

        assert!(result.contains(":C"), "Should contain stream complete marker");

        assert!(result.contains("[1,2,3]"), "Should contain first chunk");
        assert!(result.contains("[4,5,6]"), "Should contain second chunk");
        assert!(result.contains("[7,8,9]"), "Should contain third chunk");
    }

    #[test]
    fn test_serialize_byte_stream() {
        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert(
            "byteStream".to_string(),
            json!({
                "$stream": {
                    "byteStream": true,
                    "chunks": [
                        [65, 66, 67],
                        [68, 69, 70]
                    ]
                }
            }),
        );

        let element = SerializedReactElement::create_html_element("div", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("$"), "Should contain by-value reference");

        assert!(result.contains(":r"), "Should contain byte stream start marker");

        assert!(result.contains(":C"), "Should contain stream complete marker");
    }

    #[test]
    fn test_serialize_nested_binary_types() {
        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert(
            "data".to_string(),
            json!({
                "buffer": {
                    "$typedarray": {
                        "type": "Uint8Array",
                        "data": [1, 2, 3]
                    }
                },
                "file": {
                    "$blob": {
                        "type": "text/plain",
                        "data": [72, 101, 108, 108, 111]
                    }
                }
            }),
        );

        let element = SerializedReactElement::create_html_element("div", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("$"), "Should contain references");
        assert!(result.contains("$B"), "Should contain Blob reference");
        assert!(result.contains(":o"), "Should contain Uint8Array tag");
        assert!(result.contains("text/plain"), "Should contain blob type");
    }

    #[test]
    fn test_serialize_typedarray_all_types() {
        let types = vec![
            ("ArrayBuffer", "A"),
            ("Int8Array", "O"),
            ("Uint8Array", "o"),
            ("Uint8ClampedArray", "U"),
            ("Int16Array", "S"),
            ("Uint16Array", "s"),
            ("Int32Array", "L"),
            ("Uint32Array", "l"),
            ("Float32Array", "G"),
            ("Float64Array", "g"),
            ("BigInt64Array", "M"),
            ("BigUint64Array", "m"),
            ("DataView", "V"),
        ];

        for (type_name, expected_tag) in types {
            let mut serializer = RscSerializer::new();

            let mut props = FxHashMap::default();
            props.insert(
                "data".to_string(),
                json!({
                    "$typedarray": {
                        "type": type_name,
                        "data": [1, 2, 3]
                    }
                }),
            );

            let element = SerializedReactElement::create_html_element("div", Some(props));
            let result = serializer.serialize_to_rsc_format(&element);

            assert!(
                result.contains(&format!(":{}", expected_tag)),
                "Type {} should have tag {}",
                type_name,
                expected_tag
            );
        }
    }

    #[test]
    fn test_serialize_empty_stream() {
        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert(
            "stream".to_string(),
            json!({
                "$stream": {
                    "chunks": []
                }
            }),
        );

        let element = SerializedReactElement::create_html_element("div", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains(":R"), "Should contain stream start marker");
        assert!(result.contains(":C"), "Should contain stream complete marker");
    }

    #[test]
    fn test_serialize_binary_with_special_values() {
        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert(
            "mixed".to_string(),
            json!({
                "nan": "$NaN",
                "buffer": {
                    "$typedarray": {
                        "type": "Uint8Array",
                        "data": [1, 2, 3]
                    }
                },
                "date": {
                    "$date": "2025-12-09T18:00:00.000Z"
                }
            }),
        );

        let element = SerializedReactElement::create_html_element("div", Some(props));
        let result = serializer.serialize_to_rsc_format(&element);

        assert!(result.contains("$NaN"), "Should contain NaN marker");
        assert!(result.contains("$D"), "Should contain Date marker");
        assert!(result.contains(":o"), "Should contain TypedArray tag");
    }

    #[test]
    fn test_row_id_collision_check() {
        let mut serializer = RscSerializer::new();

        let mut props = FxHashMap::default();
        props.insert(
            "data".to_string(),
            json!({
                "map": {"$map": [["key1", "value1"], ["key2", "value2"]]},
                "set": {"$set": ["a", "b", "c"]},
                "promise": {"$promise": {"status": "pending", "value": null}}
            }),
        );
        props.insert("className".to_string(), json!("container"));

        let parent_element = SerializedReactElement::create_html_element("div", Some(props));

        let result = serializer.serialize_to_rsc_format(&parent_element);

        let mut row_ids = rustc_hash::FxHashSet::default();

        for line in result.lines() {
            if let Some(colon_pos) = line.find(':') {
                let row_id = line[..colon_pos].to_string();
                assert!(
                    row_ids.insert(row_id.clone()),
                    "Row ID collision detected: '{}' appears multiple times in output",
                    row_id
                );
            }
        }

        assert!(
            row_ids.len() >= 3,
            "Expected multiple unique row IDs for outlined payload, got {}",
            row_ids.len()
        );
    }
}
