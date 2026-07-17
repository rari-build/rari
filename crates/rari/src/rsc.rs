//! Backend-specific React Server Component (Flight) registry extension.
//!
//! This module re-exports the protocol-agnostic [`ComponentRegistry`] and
//! provides a [`ClientReferenceRegistry`] side-map for Flight-specific
//! client-reference metadata that is intentionally kept out of the base
//! registy to keep it reusable across backends (TanStack Start, etc.).

use cow_utils::CowUtils;
use rustc_hash::FxHashMap;

pub use rari_core::component_registry::*;

// ---- Flight-specific client-reference registry ----------------------------

/// Metadata for a component that has been designated as a Flight client
/// reference (i.e. a "use client" boundary).
#[derive(Debug, Clone)]
#[expect(
    clippy::exhaustive_structs,
    reason = "Known set of fields; backends construct exhaustively"
)]
pub struct ClientRefInfo {
    pub file_path: String,
    pub export_name: String,
}

/// A side-map for client-reference metadata.
///
/// Flight protocol requires tracking which components are "client references"
/// and their corresponding file/export identity.  This information lives here
/// rather than on [`TransformedComponent`] so the base registry stays
/// protocol-agnostic.
pub struct ClientReferenceRegistry {
    refs: FxHashMap<String, ClientRefInfo>,
}

impl ClientReferenceRegistry {
    pub fn new() -> Self {
        Self { refs: FxHashMap::default() }
    }

    /// Normalize backslashes to forward slashes for consistent keys.
    fn normalize_id(id: &str) -> String {
        id.cow_replace('\\', "/").into_owned()
    }

    /// Register a component as a client reference.
    pub fn register(&mut self, id: &str, file_path: &str, export_name: &str) {
        let normalized_id = Self::normalize_id(id);
        let normalized_file_path = Self::normalize_id(file_path);
        self.refs.insert(
            normalized_id,
            ClientRefInfo {
                file_path: normalized_file_path,
                export_name: export_name.to_string(),
            },
        );
    }

    /// Check whether the given component is a client reference.
    pub fn is_reference(&self, id: &str) -> bool {
        let normalized_id = Self::normalize_id(id);
        self.refs.contains_key(&normalized_id)
    }

    /// Look up the file-path and export name for a client reference.
    pub fn get_info(&self, id: &str) -> Option<&ClientRefInfo> {
        let normalized_id = Self::normalize_id(id);
        self.refs.get(&normalized_id)
    }
}

impl Default for ClientReferenceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---- Tests -----------------------------------------------------------------

#[cfg(test)]
#[expect(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use smallvec::SmallVec;

    use super::*;

    #[test]
    fn test_client_reference_functionality() {
        let mut client_refs = ClientReferenceRegistry::new();

        // Component must be registered in the base registry as well for a
        // realistic scenario, but the side-map operates independently.
        let mut registry = ComponentRegistry::new();
        registry
            .register_component(
                "MyComponent",
                "function MyComponent() { return <div>Hello</div>; }",
                "transformed code".to_string(),
                SmallVec::new(),
            )
            .expect("Failed to register MyComponent");

        assert!(!client_refs.is_reference("MyComponent"));
        assert!(client_refs.get_info("MyComponent").is_none());

        client_refs.register("MyComponent", "/components/MyComponent.tsx", "default");

        assert!(client_refs.is_reference("MyComponent"));

        let info = client_refs.get_info("MyComponent");
        assert!(info.is_some());
        assert_eq!(info.unwrap().file_path, "/components/MyComponent.tsx");
        assert_eq!(info.unwrap().export_name, "default");

        assert!(!client_refs.is_reference("NonExistent"));
        assert!(client_refs.get_info("NonExistent").is_none());
    }

    #[test]
    fn test_client_reference_normalization() {
        let mut client_refs = ClientReferenceRegistry::new();

        let mut registry = ComponentRegistry::new();
        registry
            .register_component("ui\\Button", "source", "transformed".to_string(), SmallVec::new())
            .expect("Failed to register component");

        client_refs.register("ui\\Button", "src\\components\\Button.tsx", "default");

        assert!(client_refs.is_reference("ui/Button"));
        assert!(client_refs.is_reference("ui\\Button"));

        let info = client_refs.get_info("ui/Button").unwrap();
        assert_eq!(info.file_path, "src/components/Button.tsx");
        assert_eq!(info.export_name, "default");

        let info2 = client_refs.get_info("ui\\Button").unwrap();
        assert_eq!(info2.file_path, "src/components/Button.tsx");
        assert_eq!(info2.export_name, "default");
    }
}
