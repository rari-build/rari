pub mod actions;
pub mod components;
pub mod flight;
pub mod rendering;
pub mod types;
pub mod utils;

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

pub use actions::{handle_form_action, handle_server_action, validate_redirect_url};
pub use components::{
    ComponentContext, ComponentProp, ComponentRegistry, ComponentType, TransformedComponent,
};
pub use flight::parser::{PromiseRef, RscWireFormatParser, StreamingState};
pub use flight::serializer::{
    ElementType, RscSerializer, SerializedReactElement, ServerComponentExecutor,
};
pub use rendering::core::{RscJsLoader, RscRenderer};
pub use rendering::html::RscHtmlRenderer;
pub use rendering::streaming::{RscStream, RscStreamChunk};
pub use types::elements::ReactElement as LoadingReactElement;
pub use types::elements::ReactElement;
pub use types::tree::{RSCRenderDebug, RSCRenderResult, RSCTree};
pub use types::{RscElement as ParsedRscElement, SuspenseBoundary};
pub use utils::dependencies::extract_dependencies;
