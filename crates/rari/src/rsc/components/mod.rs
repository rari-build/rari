use parking_lot::Mutex;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::time::Instant;

static CIRCULAR_DETECTION: std::sync::OnceLock<Mutex<FxHashSet<String>>> =
    std::sync::OnceLock::new();

type ComponentDependencies = SmallVec<[String; 4]>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentType {
    Client,
    Server,
    Shared,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentProp {
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentContext {
    pub values: FxHashMap<String, String>,
}

#[derive(Clone)]
pub struct TransformedComponent {
    pub id: String,
    pub source: String,
    pub transformed_source: String,
    pub dependencies: ComponentDependencies,
    pub is_loaded: bool,
    pub initially_loaded: bool,
    pub module_specifier: Option<String>,
    pub module_id: Option<usize>,
    pub is_client_reference: bool,
    pub client_reference_path: Option<String>,
    pub client_reference_export: Option<String>,
    pub is_stale: bool,
    pub last_reload_timestamp: Option<Instant>,
    pub reload_attempt_count: usize,
}

pub struct ComponentRegistry {
    components: FxHashMap<String, TransformedComponent>,
    dependency_graph: FxHashMap<String, FxHashSet<String>>,
    reverse_dependency_graph: FxHashMap<String, FxHashSet<String>>,
    specifier_to_id: FxHashMap<String, String>,
}

impl ComponentRegistry {
    pub fn new() -> Self {
        Self {
            components: FxHashMap::default(),
            dependency_graph: FxHashMap::default(),
            reverse_dependency_graph: FxHashMap::default(),
            specifier_to_id: FxHashMap::default(),
        }
    }

    pub fn register_component(
        &mut self,
        id: &str,
        source: &str,
        transformed_source: String,
        dependencies: ComponentDependencies,
    ) -> Result<(), String> {
        let component_id = id.to_string();
        let deps_set: FxHashSet<String> = dependencies.iter().cloned().collect();

        self.components.insert(
            component_id.clone(),
            TransformedComponent {
                id: component_id.clone(),
                source: source.to_string(),
                transformed_source,
                dependencies: dependencies.clone(),
                is_loaded: false,
                initially_loaded: false,
                module_specifier: None,
                module_id: None,
                is_client_reference: false,
                client_reference_path: None,
                client_reference_export: None,
                is_stale: false,
                last_reload_timestamp: None,
                reload_attempt_count: 0,
            },
        );

        self.dependency_graph.insert(component_id.clone(), deps_set.clone());

        for dep in deps_set {
            self.reverse_dependency_graph
                .entry(dep.clone())
                .or_default()
                .insert(component_id.clone());
        }

        Ok(())
    }

    pub fn mark_component_loaded(&mut self, id: &str) {
        if let Some(component) = self.components.get_mut(id) {
            component.is_loaded = true;
        }
    }

    pub fn mark_component_not_loaded(&mut self, id: &str) {
        if let Some(component) = self.components.get_mut(id) {
            component.is_loaded = false;
        }
    }

    pub fn mark_component_initially_loaded(&mut self, id: &str) {
        if let Some(component) = self.components.get_mut(id) {
            component.initially_loaded = true;
        }
    }

    pub fn mark_component_not_initially_loaded(&mut self, id: &str) {
        if let Some(component) = self.components.get_mut(id) {
            component.initially_loaded = false;
        }
    }

    pub fn has_been_initially_loaded(&self, id: &str) -> bool {
        self.components.get(id).is_some_and(|c| c.initially_loaded)
    }

    pub fn is_component_loaded(&self, id: &str) -> bool {
        self.components.get(id).is_some_and(|c| c.is_loaded)
    }

    pub fn get_component(&self, id: &str) -> Option<&TransformedComponent> {
        self.components.get(id)
    }

    pub fn get_component_mut(&mut self, id: &str) -> Option<&mut TransformedComponent> {
        self.components.get_mut(id)
    }

    pub fn get_all_components(&self) -> &FxHashMap<String, TransformedComponent> {
        &self.components
    }

    pub fn get_unloaded_components_in_order(&self) -> ComponentDependencies {
        let mut result = ComponentDependencies::new();
        let mut visited = FxHashSet::default();

        let unloaded: ComponentDependencies = self
            .components
            .iter()
            .filter(|(_, c)| !c.is_loaded)
            .map(|(id, _)| id.clone())
            .collect();

        for component_id in unloaded {
            self.topological_sort_helper(&component_id, &mut visited, &mut result);
        }

        result
    }

    fn topological_sort_helper(
        &self,
        component_id: &str,
        visited: &mut FxHashSet<String>,
        result: &mut ComponentDependencies,
    ) {
        if visited.contains(component_id) {
            return;
        }

        let has_circular_dependency = {
            let circular_detection =
                CIRCULAR_DETECTION.get_or_init(|| Mutex::new(FxHashSet::default()));
            let mut circular_set = circular_detection.lock();
            if circular_set.contains(component_id) {
                true
            } else {
                circular_set.insert(component_id.to_string());
                false
            }
        };

        if has_circular_dependency {
            eprintln!("Warning: Circular dependency detected for component: {component_id}");
            return;
        }

        visited.insert(component_id.to_string());

        if let Some(deps) = self.dependency_graph.get(component_id) {
            for dep in deps {
                if self.components.contains_key(dep) {
                    self.topological_sort_helper(dep, visited, result);
                }
            }
        }

        {
            let circular_detection =
                CIRCULAR_DETECTION.get_or_init(|| Mutex::new(FxHashSet::default()));
            let mut circular_set = circular_detection.lock();
            circular_set.remove(component_id);
        }

        if let Some(component) = self.components.get(component_id)
            && !component.is_loaded
        {
            result.push(component_id.to_string());
        }
    }

    pub fn get_dependencies(&self, component_id: &str) -> Option<ComponentDependencies> {
        self.dependency_graph.get(component_id).map(|deps| deps.iter().cloned().collect())
    }

    pub fn get_dependents(&self, component_id: &str) -> Option<ComponentDependencies> {
        self.reverse_dependency_graph.get(component_id).map(|deps| deps.iter().cloned().collect())
    }

    pub fn set_module_info(&mut self, id: &str, specifier: String, module_id: usize) {
        if let Some(component) = self.components.get_mut(id) {
            component.module_id = Some(module_id);
            component.module_specifier = Some(specifier.clone());
            component.is_loaded = true;

            self.specifier_to_id.insert(specifier, id.to_string());
        }
    }

    pub fn has_module_info(&self, id: &str) -> bool {
        self.components
            .get(id)
            .map(|c| c.module_id.is_some() && c.module_specifier.is_some())
            .unwrap_or(false)
    }

    pub fn get_module_id(&self, id: &str) -> Option<usize> {
        self.components.get(id).and_then(|c| c.module_id)
    }

    pub fn get_module_specifier(&self, id: &str) -> Option<&str> {
        self.components.get(id).and_then(|c| c.module_specifier.as_deref())
    }

    pub fn get_by_specifier(&self, specifier: &str) -> Option<TransformedComponent> {
        self.specifier_to_id.get(specifier).and_then(|id| self.components.get(id)).cloned()
    }

    pub fn create_module_specifier(&self, component_id: &str) -> String {
        format!("file:///components/{component_id}.js")
    }

    pub fn is_component_registered(&self, id: &str) -> bool {
        self.components.contains_key(id)
    }

    pub fn list_component_ids(&self) -> Vec<String> {
        self.components.keys().cloned().collect()
    }

    pub fn mark_module_stale(&mut self, id: &str) {
        if let Some(component) = self.components.get_mut(id) {
            component.is_stale = true;
        }
    }

    pub fn is_module_stale(&self, id: &str) -> bool {
        self.components.get(id).map(|c| c.is_stale).unwrap_or(false)
    }

    pub fn get_stale_modules(&self) -> Vec<String> {
        self.components
            .iter()
            .filter(|(_, component)| component.is_stale)
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn update_module_reload_timestamp(&mut self, id: &str, timestamp: Instant) {
        if let Some(component) = self.components.get_mut(id) {
            component.last_reload_timestamp = Some(timestamp);
            component.is_stale = false;
            component.reload_attempt_count += 1;
        }
    }

    pub fn get_module_reload_timestamp(&self, id: &str) -> Option<Instant> {
        self.components.get(id).and_then(|c| c.last_reload_timestamp)
    }

    pub fn register_client_reference(&mut self, id: &str, file_path: &str, export_name: &str) {
        if let Some(component) = self.components.get_mut(id) {
            component.is_client_reference = true;
            component.client_reference_path = Some(file_path.to_string());
            component.client_reference_export = Some(export_name.to_string());
        }
    }

    pub fn is_client_reference(&self, id: &str) -> bool {
        self.components.get(id).map(|c| c.is_client_reference).unwrap_or(false)
    }

    pub fn get_client_reference_info(&self, id: &str) -> Option<(String, String)> {
        self.components.get(id).and_then(|c| {
            if c.is_client_reference {
                c.client_reference_path
                    .as_ref()
                    .zip(c.client_reference_export.as_ref())
                    .map(|(path, export)| (path.clone(), export.clone()))
            } else {
                None
            }
        })
    }

    pub fn find_dependency_code(&self, id: &str) -> Option<String> {
        if let Some(component) = self.components.get(id) {
            return Some(component.source.clone());
        }

        let normalized_id = self.normalize_component_id(id);
        if let Some(component) = self.components.get(&normalized_id) {
            return Some(component.source.clone());
        }

        for (component_id, component) in &self.components {
            if component_id.ends_with(&normalized_id) {
                return Some(component.source.clone());
            }
        }

        None
    }

    pub fn remove_component(&mut self, id: &str) {
        if let Some(component) = self.components.remove(id) {
            self.dependency_graph.remove(id);
            for (_, dependents) in self.reverse_dependency_graph.iter_mut() {
                dependents.remove(id);
            }
            if let Some(specifier) = &component.module_specifier {
                self.specifier_to_id.remove(specifier);
            }
        }
    }

    fn normalize_component_id(&self, id: &str) -> String {
        let mut clean_id = id;
        let is_relative = id.starts_with("./") || id.starts_with("../");

        clean_id = clean_id.strip_prefix("./").unwrap_or(clean_id);
        clean_id = clean_id.strip_prefix("../").unwrap_or(clean_id);

        if is_relative && let Some(slash_idx) = clean_id.rfind('/') {
            clean_id = &clean_id[slash_idx + 1..];
        }

        if let Some(dot_idx) = clean_id.rfind('.') {
            clean_id[..dot_idx].to_string()
        } else {
            clean_id.to_string()
        }
    }
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;
    use smallvec::smallvec;

    #[test]
    fn test_component_type() {
        let client_type = ComponentType::Client;
        let server_type = ComponentType::Server;
        let shared_type = ComponentType::Shared;

        assert_eq!(format!("{client_type:?}"), "Client");
        assert_eq!(format!("{server_type:?}"), "Server");
        assert_eq!(format!("{shared_type:?}"), "Shared");
    }

    #[test]
    fn test_component_prop() {
        let prop = ComponentProp { value: "test value".to_string() };
        assert_eq!(prop.value, "test value");
    }

    #[test]
    fn test_component_registry() {
        let mut registry = ComponentRegistry::new();

        registry
            .register_component(
                "TestComponent",
                "function TestComponent() { return <div>Test</div>; }",
                "function TestComponent() { return React.createElement('div', null, 'Test'); }"
                    .to_string(),
                smallvec![],
            )
            .expect("Failed to register test component");

        let component =
            registry.get_component("TestComponent").expect("TestComponent should be registered");
        assert_eq!(component.id, "TestComponent");
        assert_eq!(
            component.transformed_source,
            "function TestComponent() { return React.createElement('div', null, 'Test'); }"
        );

        assert!(registry.get_component("NonExistentComponent").is_none());
    }

    #[test]
    fn test_dependency_resolution() {
        let mut registry = ComponentRegistry::new();

        registry
            .register_component(
                "ComponentA",
                "source A",
                "transformed A".to_string(),
                smallvec!["ComponentB".to_string(), "ComponentC".to_string()],
            )
            .expect("Failed to register ComponentA");

        registry
            .register_component(
                "ComponentB",
                "source B",
                "transformed B".to_string(),
                smallvec!["ComponentC".to_string()],
            )
            .expect("Failed to register ComponentB");

        registry
            .register_component("ComponentC", "source C", "transformed C".to_string(), smallvec![])
            .expect("Failed to register ComponentC");

        let order = registry.get_unloaded_components_in_order();

        assert!(
            order.iter().position(|id| id == "ComponentC").expect("ComponentC should be in order")
                < order
                    .iter()
                    .position(|id| id == "ComponentB")
                    .expect("ComponentB should be in order")
        );

        assert!(
            order.iter().position(|id| id == "ComponentB").expect("ComponentB should be in order")
                < order
                    .iter()
                    .position(|id| id == "ComponentA")
                    .expect("ComponentA should be in order")
        );
    }

    #[test]
    fn test_circular_dependency_resolution() {
        let mut registry = ComponentRegistry::new();

        registry
            .register_component(
                "ComponentX",
                "source X",
                "transformed X".to_string(),
                smallvec!["ComponentY".to_string()],
            )
            .expect("Failed to register ComponentX with dependency");

        registry
            .register_component(
                "ComponentY",
                "source Y",
                "transformed Y".to_string(),
                smallvec!["ComponentZ".to_string()],
            )
            .expect("Failed to register ComponentY with dependency");

        registry
            .register_component(
                "ComponentZ",
                "source Z",
                "transformed Z".to_string(),
                smallvec!["ComponentX".to_string()],
            )
            .expect("Failed to register ComponentZ with dependency");

        let order = registry.get_unloaded_components_in_order();

        assert_eq!(order.len(), 3);
        assert!(order.contains(&"ComponentX".to_string()));
        assert!(order.contains(&"ComponentY".to_string()));
        assert!(order.contains(&"ComponentZ".to_string()));
    }

    #[test]
    fn test_normalize_component_id() {
        let registry = ComponentRegistry::new();

        assert_eq!(registry.normalize_component_id("./component.tsx"), "component");
        assert_eq!(registry.normalize_component_id("../utils/helper.js"), "helper");
        assert_eq!(registry.normalize_component_id("Button"), "Button");
        assert_eq!(registry.normalize_component_id("components/Button.jsx"), "components/Button");
    }

    #[test]
    fn test_client_reference_functionality() {
        let mut registry = ComponentRegistry::new();

        registry
            .register_component(
                "MyComponent",
                "function MyComponent() { return <div>Hello</div>; }",
                "transformed code".to_string(),
                SmallVec::new(),
            )
            .expect("Failed to register MyComponent");

        assert!(!registry.is_client_reference("MyComponent"));
        assert!(registry.get_client_reference_info("MyComponent").is_none());

        registry.register_client_reference("MyComponent", "/components/MyComponent.tsx", "default");

        assert!(registry.is_client_reference("MyComponent"));

        let info = registry.get_client_reference_info("MyComponent");
        assert!(info.is_some());
        let (path, export) = info.expect("Component info should be available");
        assert_eq!(path, "/components/MyComponent.tsx");
        assert_eq!(export, "default");

        assert!(!registry.is_client_reference("NonExistent"));
        assert!(registry.get_client_reference_info("NonExistent").is_none());
    }
}
