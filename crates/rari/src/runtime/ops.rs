use deno_core::OpDecl;
use deno_core::OpState;
use deno_core::error::ResourceError;
use serde::Deserialize;
use std::cell::RefCell;
use std::rc::Rc;
use std::vec::Vec;
use tokio::sync::mpsc;
use tracing::error;

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum RscStreamOperation {
    #[serde(rename = "module")]
    ModuleReference {
        row_id: String,
        module_id: String,
        chunks: Vec<String>,
        name: String,
        #[serde(default)]
        async_module: bool,
    },
    #[serde(rename = "element")]
    ReactElement { row_id: String, element: serde_json::Value },
    #[serde(rename = "symbol")]
    Symbol { row_id: String, symbol_ref: String },
    #[serde(rename = "error")]
    Error {
        row_id: String,
        message: String,
        #[serde(default)]
        stack: Option<String>,
        #[serde(default)]
        phase: Option<String>,
        #[serde(default)]
        digest: Option<String>,
    },
    #[serde(rename = "complete")]
    Complete {
        #[serde(default)]
        #[allow(unused)]
        final_row_id: Option<String>,
    },
}

#[derive(Default)]
pub struct StreamOpState {
    pub chunk_sender: Option<mpsc::Sender<Result<Vec<u8>, String>>>,
    pub current_stream_id: Option<String>,
    pub row_counter: u32,
}

impl StreamOpState {
    pub fn get_next_row_id(&mut self) -> String {
        let id = self.row_counter.to_string();
        self.row_counter += 1;
        id
    }
}

#[allow(clippy::disallowed_methods)]
#[deno_core::op2(async)]
pub async fn op_send_chunk_to_rust(
    state: Rc<RefCell<OpState>>,
    #[string] operation_json: String,
) -> Result<(), ResourceError> {
    let operation: RscStreamOperation = match serde_json::from_str(&operation_json) {
        Ok(op) => op,
        Err(e) => {
            let err_msg = format!("Invalid JSON for RSC operation: {e}. JSON: {operation_json}");
            error!("{err_msg}");
            return Err(ResourceError::Other(err_msg));
        }
    };

    let sender_option = {
        let mut op_state_ref = state.borrow_mut();
        let stream_op_state = match op_state_ref.try_borrow_mut::<StreamOpState>() {
            Some(sos) => sos,
            None => return Err(ResourceError::Other("StreamOpState not found.".to_string())),
        };

        match &operation {
            RscStreamOperation::Complete { .. } | RscStreamOperation::Error { .. } => {
                stream_op_state.chunk_sender.take()
            }
            _ => stream_op_state.chunk_sender.as_ref().cloned(),
        }
    };

    match (sender_option, operation) {
        (
            Some(sender),
            RscStreamOperation::ModuleReference { row_id, module_id, chunks, name, async_module },
        ) => {
            let module_data = serde_json::json!({
                "id": module_id,
                "chunks": chunks,
                "name": name,
                "async": async_module
            });

            let rsc_row = format!("{row_id}:M{module_data}");

            if sender.send(Ok(rsc_row.into_bytes())).await.is_err() {
                error!("op_send_chunk_to_rust: receiver dropped for module reference.");
            }
        }
        (Some(sender), RscStreamOperation::ReactElement { row_id, element }) => {
            let rsc_row = format!("{row_id}:J{element}");

            if sender.send(Ok(rsc_row.into_bytes())).await.is_err() {
                error!("op_send_chunk_to_rust: receiver dropped for React element.");
            }
        }
        (Some(sender), RscStreamOperation::Symbol { row_id, symbol_ref }) => {
            let rsc_row = format!("{row_id}:S\"{symbol_ref}\"");

            if sender.send(Ok(rsc_row.into_bytes())).await.is_err() {
                error!("op_send_chunk_to_rust: receiver dropped for symbol reference.");
            }
        }
        (Some(sender), RscStreamOperation::Error { row_id, message, stack, phase, digest }) => {
            error!("Streaming error in row {row_id}: {message}");
            if let Some(stack_trace) = &stack {
                error!("Stack trace: {stack_trace}");
            }

            let error_data = serde_json::json!({
                "message": message,
                "stack": stack,
                "phase": phase,
                "digest": digest
            });

            let rsc_row = format!("{row_id}:E{error_data}");

            if sender.send(Ok(rsc_row.into_bytes())).await.is_err() {
                error!("op_send_chunk_to_rust: receiver dropped for error message.");
            }
        }
        (Some(_sender), RscStreamOperation::Complete { final_row_id: _ }) => {}
        (None, operation) => {
            error!("No sender available for operation: {operation:?}");
            return Err(ResourceError::Other("No chunk sender available".to_string()));
        }
    }

    Ok(())
}

#[allow(clippy::disallowed_methods)]
pub fn create_module_operation(
    row_id: &str,
    module_id: &str,
    chunks: &[&str],
    name: &str,
) -> String {
    serde_json::json!({
        "type": "module",
        "row_id": row_id,
        "module_id": module_id,
        "chunks": chunks,
        "name": name,
        "async_module": false
    })
    .to_string()
}

#[allow(clippy::disallowed_methods)]
pub fn create_element_operation(row_id: &str, element: &serde_json::Value) -> String {
    serde_json::json!({
        "type": "element",
        "row_id": row_id,
        "element": element
    })
    .to_string()
}

#[allow(clippy::disallowed_methods)]
pub fn create_symbol_operation(row_id: &str, symbol_ref: &str) -> String {
    serde_json::json!({
        "type": "symbol",
        "row_id": row_id,
        "symbol_ref": symbol_ref
    })
    .to_string()
}

#[allow(clippy::disallowed_methods)]
pub fn create_error_operation(
    row_id: &str,
    message: &str,
    stack: Option<&str>,
    phase: Option<&str>,
    digest: Option<&str>,
) -> String {
    serde_json::json!({
        "type": "error",
        "row_id": row_id,
        "message": message,
        "stack": stack,
        "phase": phase,
        "digest": digest
    })
    .to_string()
}

pub fn get_streaming_ops() -> Vec<OpDecl> {
    vec![op_send_chunk_to_rust(), op_internal_log()]
}

#[deno_core::op2(fast)]
pub fn op_internal_log(#[string] message: &str) {
    error!("[RARI_INTERNAL_LOG] {message}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[test]
    fn test_stream_op_state_operations() {
        let mut stream_state = StreamOpState::default();

        let row_id_1 = stream_state.get_next_row_id();
        let row_id_2 = stream_state.get_next_row_id();

        assert_eq!(row_id_1, "0");
        assert_eq!(row_id_2, "1");
        assert_eq!(stream_state.row_counter, 2);

        let (sender, _receiver) = mpsc::channel::<Result<Vec<u8>, String>>(32);
        stream_state.chunk_sender = Some(sender);

        assert!(stream_state.chunk_sender.is_some());
    }

    #[test]
    fn test_operation_creation() {
        let module_op = create_module_operation("0", "Button", &["chunk1", "chunk2"], "default");
        assert!(module_op.contains("\"type\":\"module\""));
        assert!(module_op.contains("\"row_id\":\"0\""));
        assert!(module_op.contains("\"module_id\":\"Button\""));

        let element_op = create_element_operation("1", &serde_json::json!({"type": "div"}));
        assert!(element_op.contains("\"type\":\"element\""));
        assert!(element_op.contains("\"row_id\":\"1\""));

        let symbol_op = create_symbol_operation("2", "Symbol.for('react.element')");
        assert!(symbol_op.contains("\"type\":\"symbol\""));
        assert!(symbol_op.contains("\"row_id\":\"2\""));

        let error_op = create_error_operation("3", "Test error", Some("stack trace"), None, None);
        assert!(error_op.contains("\"type\":\"error\""));
        assert!(error_op.contains("\"message\":\"Test error\""));
        assert!(error_op.contains("\"stack\":\"stack trace\""));
    }
}
