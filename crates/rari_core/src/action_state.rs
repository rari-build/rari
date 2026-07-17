//! Helpers for syncing request-scoped state into the V8 runtime before a render.

use serde_json::Value;

/// Build the JS snippet that syncs/clears the action form-state global before render.
pub fn action_form_state_sync_script(form_state: Option<&Value>) -> String {
    match form_state {
        Some(state) => format!(
            "globalThis['~rari'] = globalThis['~rari'] || {{}}; globalThis['~rari'].actionFormState = {state};"
        ),
        None => "if (globalThis['~rari']) delete globalThis['~rari'].actionFormState;".to_string(),
    }
}
