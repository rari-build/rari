use crate::error::RariError;
use deno_core::{JsRuntime, PollEventLoopOptions, v8};
use serde_json::Value as JsonValue;

#[macro_export]
macro_rules! with_scope {
    ($runtime:expr, |$scope:ident| $body:expr) => {{
        use deno_core::v8;
        let context = $runtime.main_context();
        v8::scope_with_context!($scope, $runtime.v8_isolate(), context);
        $body
    }};
}

pub fn get_module_namespace_as_json(
    runtime: &mut JsRuntime,
    module_id: deno_core::ModuleId,
) -> Result<JsonValue, RariError> {
    match runtime.get_module_namespace(module_id) {
        Ok(namespace) => {
            let context = runtime.main_context();
            v8::scope_with_context!(scope, runtime.v8_isolate(), context);
            let local_namespace = v8::Local::new(scope, namespace);
            let local_value: v8::Local<v8::Value> = local_namespace.into();
            v8_to_json(scope, local_value)
        }
        Err(e) => Err(RariError::js_execution(format!("Failed to get module namespace: {e}"))),
    }
}

pub async fn run_event_loop_with_error_handling(
    runtime: &mut JsRuntime,
    context: &str,
) -> Result<(), RariError> {
    match runtime.run_event_loop(PollEventLoopOptions::default()).await {
        Ok(()) => Ok(()),
        Err(e) => {
            let error_str = e.to_string();
            if error_str.contains("assertion") || error_str.contains("panicked") {
                Err(RariError::js_runtime(format!(
                    "Critical runtime error in {context}: {error_str}"
                )))
            } else {
                Err(RariError::js_execution(format!("Event loop error in {context}: {error_str}")))
            }
        }
    }
}

fn extract_promise_metadata<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    value: v8::Local<'s, v8::Value>,
) -> Option<JsonValue> {
    if !is_promise(scope, value) {
        return None;
    }

    tracing::warn!(
        "Encountered Promise object during serialization, extracting metadata instead of full serialization"
    );

    let mut metadata = serde_json::Map::new();
    metadata.insert("__promise_placeholder".to_string(), serde_json::Value::Bool(true));
    metadata.insert("type".to_string(), serde_json::Value::String("Promise".to_string()));

    if let Ok(obj) = v8::Local::<v8::Object>::try_from(value) {
        if let Some(boundary_key) = v8::String::new(scope, "boundaryId")
            && let Some(boundary_val) = obj.get(scope, boundary_key.into())
            && let Some(boundary_str) = boundary_val.to_string(scope)
        {
            let boundary_id = boundary_str.to_rust_string_lossy(scope.as_ref());
            metadata.insert("boundaryId".to_string(), serde_json::Value::String(boundary_id));
        }

        if let Some(promise_key) = v8::String::new(scope, "promiseId")
            && let Some(promise_val) = obj.get(scope, promise_key.into())
            && let Some(promise_str) = promise_val.to_string(scope)
        {
            let promise_id = promise_str.to_rust_string_lossy(scope.as_ref());
            metadata.insert("promiseId".to_string(), serde_json::Value::String(promise_id));
        }
    }

    metadata.insert(
        "message".to_string(),
        serde_json::Value::String(
            "Promise object cannot be fully serialized, use metadata instead".to_string(),
        ),
    );

    Some(serde_json::Value::Object(metadata))
}

fn deserialize_composition_result<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    value: v8::Local<'s, v8::Value>,
) -> Result<JsonValue, RariError> {
    if is_promise(scope, value) {
        tracing::warn!(
            "Composition result is a Promise object, extracting metadata instead of full serialization"
        );

        if let Some(metadata) = extract_promise_metadata(scope, value) {
            return Ok(metadata);
        }
    }

    let v8_type_str = value.type_of(scope).to_rust_string_lossy(scope.as_ref());
    tracing::debug!("Attempting to serialize V8 value of type: {}", v8_type_str);

    if value.is_object()
        && let Ok(obj) = v8::Local::<v8::Object>::try_from(value)
        && let Some(keys) = obj.get_own_property_names(scope, v8::GetPropertyNamesArgs::default())
    {
        let key_count = keys.length();
        tracing::debug!("Object has {} keys", key_count);

        for i in 0..std::cmp::min(key_count, 10) {
            if let Some(key) = keys.get_index(scope, i)
                && let Some(key_str) = key.to_string(scope)
            {
                let key_name = key_str.to_rust_string_lossy(scope.as_ref());
                if let Some(val) = obj.get(scope, key) {
                    let val_type = val.type_of(scope).to_rust_string_lossy(scope.as_ref());
                    tracing::debug!("  Key '{}': type = {}", key_name, val_type);
                }
            }
        }
    }

    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        deno_core::serde_v8::from_v8(scope, value)
    })) {
        Ok(Ok(json_value)) => Ok(json_value),
        Ok(Err(err)) => {
            let err_str = err.to_string();
            tracing::error!("Serialization error for V8 type '{}': {}", v8_type_str, err);

            if err_str.contains("Promise") || err_str.contains("promise") {
                tracing::warn!(
                    "Serialization failed due to Promise object, using fallback extraction: {}",
                    err
                );

                if let Some(metadata) = extract_promise_metadata(scope, value) {
                    return Ok(metadata);
                }
            }

            extract_composition_result_manually(scope, value, err)
        }
        Err(_panic) => {
            tracing::error!(
                "V8 serialization panicked for type '{}', attempting manual extraction",
                v8_type_str
            );

            if let Some(metadata) = extract_promise_metadata(scope, value) {
                return Ok(metadata);
            }

            extract_composition_result_manually_from_panic(scope, value)
        }
    }
}

fn extract_composition_result_manually<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    value: v8::Local<'s, v8::Value>,
    original_error: deno_core::serde_v8::Error,
) -> Result<JsonValue, RariError> {
    let try_json_stringify =
        |scope: &mut v8::PinScope, value: v8::Local<v8::Value>| -> Option<JsonValue> {
            let context = scope.get_current_context();
            let global = context.global(scope);
            let json_key = v8::String::new(scope, "JSON")?;
            let json_obj = global.get(scope, json_key.into())?.to_object(scope)?;
            let stringify_key = v8::String::new(scope, "stringify")?;
            let stringify_value = json_obj.get(scope, stringify_key.into())?;
            let stringify_fn = stringify_value.to_object(scope)?.cast::<v8::Function>();

            let args = [value];
            let result = stringify_fn.call(scope, json_obj.into(), &args)?;
            let json_string = result.to_string(scope)?.to_rust_string_lossy(scope.as_ref());

            serde_json::from_str(&json_string).ok()
        };

    if let Some(json_value) = try_json_stringify(scope, value) {
        tracing::warn!("Used JSON.stringify fallback for serialization");
        return Ok(json_value);
    }

    let v8_type_str = value.type_of(scope).to_rust_string_lossy(scope.as_ref());
    let detailed_err_msg = format!(
        "Failed to convert V8 value of type '{}' to JSON: {}. V8 value details: {}",
        v8_type_str,
        original_error,
        value
            .to_detail_string(scope)
            .map(|s| s.to_rust_string_lossy(scope.as_ref()))
            .unwrap_or_else(|| "<unable to get detailed string for V8 value>".to_string())
    );

    tracing::error!("Serialization error: {}", detailed_err_msg);
    Err(RariError::js_execution(detailed_err_msg))
}

fn extract_composition_result_manually_from_panic<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    value: v8::Local<'s, v8::Value>,
) -> Result<JsonValue, RariError> {
    let try_json_stringify =
        |scope: &mut v8::PinScope, value: v8::Local<v8::Value>| -> Option<JsonValue> {
            let context = scope.get_current_context();
            let global = context.global(scope);
            let json_key = v8::String::new(scope, "JSON")?;
            let json_obj = global.get(scope, json_key.into())?.to_object(scope)?;
            let stringify_key = v8::String::new(scope, "stringify")?;
            let stringify_value = json_obj.get(scope, stringify_key.into())?;
            let stringify_fn = stringify_value.to_object(scope)?.cast::<v8::Function>();

            let args = [value];
            let result = stringify_fn.call(scope, json_obj.into(), &args)?;
            let json_string = result.to_string(scope)?.to_rust_string_lossy(scope.as_ref());

            serde_json::from_str(&json_string).ok()
        };

    if let Some(json_value) = try_json_stringify(scope, value) {
        tracing::warn!("Used JSON.stringify fallback after panic");
        return Ok(json_value);
    }

    let v8_type_str = value.type_of(scope).to_rust_string_lossy(scope.as_ref());
    let fallback_msg = format!(
        "V8 serialization panicked for type '{}', using fallback. V8 value details: {}",
        v8_type_str,
        value
            .to_detail_string(scope)
            .map(|s| s.to_rust_string_lossy(scope.as_ref()))
            .unwrap_or_else(|| "<unable to get detailed string for V8 value>".to_string())
    );

    tracing::warn!("Serialization panic fallback: {}", fallback_msg);

    let mut error_obj = serde_json::Map::new();
    error_obj.insert("__serialization_error".to_string(), serde_json::Value::Bool(true));
    error_obj.insert(
        "error".to_string(),
        serde_json::Value::String("V8 value could not be serialized".to_string()),
    );
    error_obj.insert("type".to_string(), serde_json::Value::String(v8_type_str));
    error_obj.insert("details".to_string(), serde_json::Value::String(fallback_msg));
    Ok(serde_json::Value::Object(error_obj))
}

pub fn v8_to_json<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    value: v8::Local<'s, v8::Value>,
) -> Result<JsonValue, RariError> {
    deserialize_composition_result(scope, value)
}

pub fn is_promise(scope: &mut v8::PinScope, value: v8::Local<v8::Value>) -> bool {
    if !value.is_object() {
        return false;
    }

    if let Some(string_rep) = value.to_string(scope) {
        let string_val = string_rep.to_rust_string_lossy(scope.as_ref());

        if string_val == "[object Promise]"
            && let Ok(obj) = v8::Local::<v8::Object>::try_from(value)
        {
            let then_key = match v8::String::new(scope, "then") {
                Some(key) => key.into(),
                None => return false,
            };
            let catch_key = match v8::String::new(scope, "catch") {
                Some(key) => key.into(),
                None => return false,
            };

            if let Some(then_val) = obj.get(scope, then_key)
                && let Some(catch_val) = obj.get(scope, catch_key)
            {
                let result = then_val.is_function() && catch_val.is_function();
                return result;
            }
        }
    }

    false
}

fn check_promise_completion(runtime: &mut JsRuntime) -> Result<bool, RariError> {
    let check_script = r#"
        (function() {
            return globalThis.__promise_resolution_complete === true;
        })()
    "#;

    match runtime.execute_script("promise_completion_check", check_script.to_string()) {
        Ok(result_val) => {
            let context = runtime.main_context();
            let result = {
                v8::scope_with_context!(scope, runtime.v8_isolate(), context);
                let local_v8_val = v8::Local::new(scope, result_val);
                let boolean_val = local_v8_val.to_boolean(scope);
                boolean_val.is_true()
            };
            Ok(result)
        }
        Err(_) => Ok(false),
    }
}

pub async fn run_event_loop_with_promise_timeout(
    runtime: &mut JsRuntime,
    script_name: &str,
    timeout_ms: u64,
) -> Result<(), RariError> {
    let timeout_duration = std::time::Duration::from_millis(timeout_ms);
    let start_time = std::time::Instant::now();
    let check_interval = std::time::Duration::from_millis(5);

    while start_time.elapsed() < timeout_duration {
        run_event_loop_with_error_handling(
            runtime,
            &format!("promise resolution iteration for '{script_name}'"),
        )
        .await?;

        if let Ok(is_complete) = check_promise_completion(runtime)
            && is_complete
        {
            break;
        }

        tokio::time::sleep(check_interval).await;
    }
    Ok(())
}
