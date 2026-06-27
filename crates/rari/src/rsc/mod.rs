pub mod actions;
pub mod components;
pub mod flight;
pub mod rendering;
pub mod types;
pub mod utils;

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum RscElement {
    Component {
        tag: String,
        key: Option<String>,
        props: FxHashMap<String, JsonValue>,
    },
    Suspense {
        fallback_ref: String,
        children_ref: String,
        boundary_id: String,
        props: FxHashMap<String, JsonValue>,
    },
    Promise {
        promise_id: String,
    },
    ModuleImport {
        module_path: String,
        export_name: String,
    },
    Reference(String),
    Text(String),
    Fragment {
        children: Vec<RscElement>,
    },
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SuspenseBoundary {
    pub boundary_id: String,
    pub fallback_ref: String,
    pub children_ref: String,
    pub has_promise: bool,
    pub promise_ids: Vec<String>,
    pub row_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ComponentValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Array(Vec<ComponentValue>),
    Object(FxHashMap<String, ComponentValue>),
    Null,
}

pub use actions::{handle_form_action, handle_server_action, validate_redirect_url};
pub use components::{
    ComponentContext, ComponentProp, ComponentRegistry, ComponentType, TransformedComponent,
};
pub use flight::{
    parser::RscWireFormatParser,
    serializer::{ElementType, RscSerializer, SerializedReactElement, ServerComponentExecutor},
};
pub use rendering::{
    core::{RscJsLoader, RscRenderer},
    html::RscHtmlRenderer,
    streaming::{RscStream, RscStreamChunk},
};
pub use types::{
    elements::{ReactElement as LoadingReactElement, ReactElement},
    tree::{RSCRenderDebug, RSCRenderResult, RSCTree},
};
pub use utils::extract_dependencies;
