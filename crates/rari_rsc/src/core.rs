use rustc_hash::FxHashMap;
use serde_json::Value;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum RscElement {
    Component {
        tag: String,
        key: Option<String>,
        props: FxHashMap<String, Value>,
    },
    Suspense {
        fallback_ref: String,
        children_ref: String,
        boundary_id: String,
        props: FxHashMap<String, Value>,
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
        children: Vec<Self>,
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

impl SuspenseBoundary {
    pub fn new(
        boundary_id: String,
        fallback_ref: String,
        children_ref: String,
        row_id: u32,
    ) -> Self {
        Self {
            boundary_id,
            fallback_ref,
            children_ref,
            has_promise: false,
            promise_ids: Vec::new(),
            row_id,
        }
    }
}

pub trait ServerComponentExecutor: Send + Sync {
    /// Executes a server component with the given name and props.
    ///
    /// # Errors
    ///
    /// Returns an error if the component execution fails or the component is not found.
    fn execute_server_component(
        &self,
        component_name: &str,
        props: Option<&FxHashMap<String, serde_json::Value>>,
    ) -> Result<serde_json::Value, rari_error::RariError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RscFlightTag {
    ModuleImport,
    Model,
    Error,
    Text,
    Hint,
    Debug,
    Console,
    StreamClose,
}

impl RscFlightTag {
    pub fn tag_char(&self) -> Option<char> {
        match self {
            Self::ModuleImport => Some('I'),
            Self::Model => None,
            Self::Error => Some('E'),
            Self::Text => Some('T'),
            Self::Hint => Some('H'),
            Self::Debug => Some('D'),
            Self::Console => Some('W'),
            Self::StreamClose => Some('C'),
        }
    }

    pub fn format_row(&self, row_id: u32, data: &str) -> String {
        match self.tag_char() {
            Some(tag) => format!("{row_id:x}:{tag}{data}\n"),
            None => format!("{row_id:x}:{data}\n"),
        }
    }
}
