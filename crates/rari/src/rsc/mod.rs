pub mod components;
pub mod rendering;
pub mod suspense;
pub mod types;
pub mod utils;
pub mod wire_format;

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

pub use components::{
    ComponentContext, ComponentProp, ComponentRegistry, ComponentType, TransformedComponent,
};
pub use rendering::core::{RscJsLoader, RscRenderer};
pub use rendering::html::RscHtmlRenderer;
pub use rendering::streaming::{RscStream, RscStreamChunk};
pub use types::elements::ReactElement as LoadingReactElement;
pub use types::elements::ReactElement;
pub use types::tree::{RSCRenderDebug, RSCRenderResult, RSCTree};
pub use types::{RscElement as ParsedRscElement, SuspenseBoundary};
pub use utils::dependency_utils::extract_dependencies;
pub use wire_format::parser::{PromiseRef, RscWireFormatParser, StreamingState};
pub use wire_format::serializer::{
    ElementType, RscSerializer, SerializedReactElement, ServerComponentExecutor,
};
