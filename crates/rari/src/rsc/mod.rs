pub mod component;
pub mod dependency_utils;
pub mod js_loader;
pub mod layout_renderer;
pub mod renderer;
pub mod rsc_html_renderer;
pub mod rsc_tree;
pub mod serializer;
pub mod streaming;
pub mod suspense;

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Array(Vec<ComponentValue>),
    Object(FxHashMap<String, ComponentValue>),
    Null,
}

pub use component::{
    ComponentContext, ComponentProp, ComponentRegistry, ComponentType, TransformedComponent,
};
pub use dependency_utils::extract_dependencies;
pub use js_loader::RscJsLoader;
pub use renderer::RscRenderer;
pub use rsc_html_renderer::RscHtmlRenderer;
pub use rsc_tree::{RSCRenderDebug, RSCRenderResult, RSCTree};
pub use serializer::{ElementType, ReactElement, RscSerializer, ServerComponentExecutor};
pub use streaming::{RscStream, RscStreamChunk};

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod compliance_tests {
    use super::*;
    use crate::rsc::serializer::RscSerializer;

    #[test]
    fn test_rsc_wire_format_compliance() {
        let mut serializer = RscSerializer::new();

        serializer.register_client_component("MyButton", "/components/MyButton.js", "default");

        let element = ReactElement {
            element_type: ElementType::ClientComponent("MyButton".to_string()),
            props: Some({
                let mut props = FxHashMap::default();
                props.insert("onClick".to_string(), serde_json::json!("handleClick"));
                props.insert("children".to_string(), serde_json::json!("Click me"));
                props
            }),
            key: None,
            ref_: None,
        };

        let output = serializer.serialize_to_rsc_format(&element);

        let lines: Vec<&str> = output.lines().collect();
        assert!(lines.len() >= 2, "Should have at least module and element rows");

        assert!(lines[0].contains(":I"), "First row should be module import");
        assert!(lines[0].contains("MyButton"), "Module should reference MyButton");
        assert!(lines[0].contains("[\"/components/MyButton.js\""), "Module should specify path");
        assert!(lines[0].contains("\"default\""), "Module should specify export name");

        assert!(lines[1].contains("[\"$\""), "Element should start with React element format");
        assert!(lines[1].contains("\"$L"), "Element should reference module with $L prefix");
    }

    #[test]
    fn test_suspense_boundary_compliance() {
        let mut serializer = RscSerializer::new();

        let fallback = ReactElement {
            element_type: ElementType::HtmlTag("div".to_string()),
            props: Some(FxHashMap::default()),
            key: None,
            ref_: None,
        };

        let boundary_id = serializer.emit_suspense_boundary(&fallback, "boundary-1");
        assert_eq!(boundary_id, "boundary-1");

        let output = serializer.output_lines.join("\n");
        assert!(output.contains("react.suspense"), "Should contain Suspense element");
        assert!(output.contains("\"fallback\""), "Should contain fallback prop");
        assert!(output.contains("@boundary-1"), "Should reference boundary ID");
    }

    #[test]
    fn test_client_component_reference_format() {
        let mut serializer = RscSerializer::new();

        serializer.register_client_component("Counter", "/app/components/Counter.tsx", "default");

        let element = ReactElement {
            element_type: ElementType::ClientComponent("Counter".to_string()),
            props: Some({
                let mut props = FxHashMap::default();
                props.insert("initialValue".to_string(), serde_json::json!(42));
                props.insert("onIncrement".to_string(), serde_json::json!("handleIncrement"));
                props
            }),
            key: None,
            ref_: None,
        };

        let output = serializer.serialize_to_rsc_format(&element);

        assert!(output.contains(":I"), "Should contain module import");
        assert!(output.contains("Counter.tsx"), "Should reference correct file");
        assert!(
            output.contains("[\"$\",\"$L"),
            "Should use proper element format with $L reference"
        );
        assert!(output.contains("\"initialValue\":42"), "Should serialize props correctly");
    }
}
