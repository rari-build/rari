use std::{env, future::Future, sync::Arc};

use rari_error::RariError;
use rustc_hash::FxHashMap;
use serde_json::Value;
use tokio::sync::mpsc::{Sender, UnboundedReceiver};

pub mod ext;
pub mod factory;
pub mod module_loader;
pub mod ops;
pub mod transpile;

use factory::{JsRuntimePool, PooledRuntime};

use crate::server::{
    middleware::request_context::RequestContext, rendering::metadata, routing::types::ParamValue,
};

pub const DEFAULT_JS_POOL_SIZE: usize = 1;

pub struct JsExecutionRuntime {
    pool: Arc<JsRuntimePool>,
}

impl Default for JsExecutionRuntime {
    fn default() -> Self {
        Self::new(None)
    }
}

fn parse_string_array_value(value: &Value) -> Vec<String> {
    if let Some(items) = value.as_array() {
        return items.iter().filter_map(|item| item.as_str().map(ToString::to_string)).collect();
    }

    if let Some(text) = value.as_str() {
        return serde_json::from_str(text).unwrap_or_default();
    }

    Vec::new()
}

fn pool_size_from_env() -> usize {
    env::var("RARI_JS_POOL_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|&n| n >= 1)
        .unwrap_or(DEFAULT_JS_POOL_SIZE)
}

#[expect(clippy::missing_errors_doc)]
impl JsExecutionRuntime {
    pub fn new(env_vars: Option<FxHashMap<String, String>>) -> Self {
        Self::with_pool_size(env_vars, pool_size_from_env())
    }

    pub fn with_pool_size(env_vars: Option<FxHashMap<String, String>>, pool_size: usize) -> Self {
        let pool_size = pool_size.max(1);
        #[expect(
            clippy::expect_used,
            reason = "JsRuntimePool::new only fails when size is 0, which is prevented above"
        )]
        let pool = JsRuntimePool::new(pool_size, env_vars)
            .expect("JS runtime pool construction cannot fail for size >= 1");

        Self { pool }
    }

    pub fn pool(&self) -> &Arc<JsRuntimePool> {
        &self.pool
    }

    pub fn pool_size(&self) -> usize {
        self.pool.size()
    }

    pub fn set_setup_mode(&self, on: bool) {
        self.pool.set_setup_mode(on);
    }

    pub fn set_post_rebuild_hook(&self, hook: factory::PostRebuildHook) {
        self.pool.set_post_rebuild_hook(hook);
    }

    pub async fn pick_runtime(&self) -> Result<PooledRuntime, RariError> {
        self.pool.pick_runtime().await
    }

    pub async fn pick_runtime_for_streaming(
        &self,
    ) -> Result<(PooledRuntime, factory::StreamingSlotGuard), RariError> {
        self.pool.pick_runtime_for_streaming().await
    }

    pub fn stream_load_at(&self, idx: usize) -> usize {
        self.pool.stream_load_at(idx)
    }

    pub async fn execute_script(
        &self,
        script_name: String,
        script_code: String,
    ) -> Result<Value, RariError> {
        self.pool.execute_script(script_name, script_code).await
    }

    pub async fn execute_script_batch(
        self: &Arc<Self>,
        scripts: Vec<(String, String)>,
    ) -> UnboundedReceiver<(usize, Result<Value, RariError>)> {
        self.pool.execute_script_batch(scripts).await
    }

    pub async fn execute_script_for_streaming(
        &self,
        stream_id: String,
        script_name: String,
        script_code: String,
        chunk_sender: Sender<Result<Vec<u8>, RariError>>,
    ) -> Result<(), RariError> {
        let (handle, _stream_lease) = self.pool.pick_runtime_for_streaming().await?;
        handle.execute_script_for_streaming(stream_id, script_name, script_code, chunk_sender).await
    }

    pub async fn collect_metadata(
        &self,
        layout_paths: Vec<String>,
        page_path: String,
        params: FxHashMap<String, ParamValue>,
        search_params: FxHashMap<String, Vec<String>>,
    ) -> Result<Value, RariError> {
        let data = serde_json::json!({
            "layoutPaths": layout_paths,
            "pagePath": page_path,
            "params": params,
            "searchParams": search_params,
        });

        let data_json =
            serde_json::to_string(&data).map_err(|e| RariError::serialization(e.to_string()))?;

        let script = format!(
            r"(function() {{
                const data = {data_json};
                return globalThis['~rari'].metadataCollector.collect(
                    data.layoutPaths,
                    data.pagePath,
                    data.params,
                    data.searchParams
                );
            }})()"
        );

        let metadata_list = self.execute_script("collect_metadata".to_string(), script).await?;

        let metadata_array = metadata_list.as_array().ok_or_else(|| {
            RariError::serialization("Expected metadata list to be an array".to_string())
        })?;

        let mut merged_metadata = serde_json::json!({});
        for metadata_item in metadata_array {
            merged_metadata = metadata::merge_metadata(&merged_metadata, metadata_item);
        }

        metadata::finalize_metadata(&mut merged_metadata);

        Ok(merged_metadata)
    }

    pub async fn collect_page_cache_tags(&self) -> Result<Vec<String>, RariError> {
        const SCRIPT: &str = r"(() => {
            const tags = new Set(
                globalThis['~rari']?.pageCacheTags ? [...globalThis['~rari'].pageCacheTags] : [],
            );
            const fromRegistry = globalThis.__rariGetActiveUseCacheTags?.() ?? [];
            for (const tag of fromRegistry)
                tags.add(tag);
            return [...tags];
        })()";

        let result =
            self.execute_script("collect_page_cache_tags".to_string(), SCRIPT.to_string()).await?;

        Ok(parse_string_array_value(&result))
    }

    pub async fn is_dynamic_render(&self) -> Result<bool, RariError> {
        const SCRIPT: &str = "((globalThis['~rari']?.useCacheDynamicDepth ?? 0) > 0)";

        let result =
            self.execute_script("is_dynamic_render".to_string(), SCRIPT.to_string()).await?;

        Ok(result.as_bool().unwrap_or(false))
    }

    pub async fn execute_function(
        &self,
        function_name: &str,
        args: Vec<Value>,
    ) -> Result<Value, RariError> {
        self.pool.execute_function(function_name, args).await
    }

    pub async fn load_and_evaluate_module(&self, specifier: &str) -> Result<(), RariError> {
        self.pool.broadcast_load_and_evaluate_module(specifier).await
    }

    pub async fn broadcast_script(
        &self,
        script_name: &str,
        script_code: &str,
    ) -> Result<(), RariError> {
        self.pool.broadcast_script(script_name, script_code).await
    }

    pub async fn add_module_to_loader(
        &self,
        specifier: &str,
        code: String,
    ) -> Result<(), RariError> {
        self.pool.broadcast_add_module_to_loader(specifier, &code).await
    }

    pub async fn clear_module_loader_caches(&self, component_id: &str) -> Result<(), RariError> {
        self.pool.broadcast_clear_module_loader_caches(component_id).await
    }

    pub async fn invalidate_component(&self, component_id: &str) -> Result<(), RariError> {
        match self.pool.invalidate_component_all(component_id).await {
            Ok(()) => Ok(()),
            Err(e) => {
                tracing::error!("Failed to invalidate component {}: {}", component_id, e);
                Err(RariError::js_runtime(format!(
                    "Failed to invalidate component {component_id}: {e}"
                )))
            }
        }
    }

    pub async fn load_component_code(
        &self,
        component_id: &str,
        component_code: &str,
    ) -> Result<(), RariError> {
        self.pool.load_component_code_all(component_id, component_code).await
    }

    pub async fn execute_script_with_request_context(
        self: &Arc<Self>,
        request_context: Arc<RequestContext>,
        script_name: String,
        script_code: String,
    ) -> Result<Value, RariError> {
        self.pool
            .with_request_context(request_context, move |runtime| async move {
                runtime.execute_script(script_name, script_code).await
            })
            .await
    }

    pub async fn with_request_context<F, Fut, T>(
        self: &Arc<Self>,
        request_context: Arc<RequestContext>,
        operation: F,
    ) -> Result<T, RariError>
    where
        T: Send + 'static,
        F: FnOnce(Arc<dyn factory::JsRuntimeInterface>) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, RariError>> + Send + 'static,
    {
        self.pool.with_request_context(request_context, operation).await
    }

    pub async fn acquire_request_runtime(
        self: &Arc<Self>,
        request_context: Arc<RequestContext>,
    ) -> Result<factory::LeasedRequestRuntime, RariError> {
        self.pool.acquire_request_runtime(request_context).await
    }
}

#[cfg(test)]
#[expect(clippy::expect_used)]
mod overlapping_stream_tests {
    use std::{
        sync::Arc,
        time::{Duration, Instant},
    };

    use rari_error::RariError;
    use tokio::sync::mpsc;

    use super::JsExecutionRuntime;

    #[tokio::test]
    async fn overlapping_streams_on_one_isolate_finish_near_max_delay() {
        let runtime = Arc::new(JsExecutionRuntime::with_pool_size(None, 1));
        let delay_ms = 200u64;
        let stream_count = 4usize;

        let start = Instant::now();
        let mut join_handles = Vec::with_capacity(stream_count);

        for i in 0..stream_count {
            let runtime = Arc::clone(&runtime);
            let stream_id = format!("overlap-{i}");
            let (tx, mut rx) = mpsc::channel::<Result<Vec<u8>, RariError>>(8);
            let script = format!(
                r#"(async function() {{
                    await new Promise((resolve) => setTimeout(resolve, {delay_ms}));
                    await Deno.core.ops.op_fizz_chunk("{stream_id}", "chunk-{i}");
                    Deno.core.ops.op_fizz_done("{stream_id}");
                }})()"#
            );

            join_handles.push(tokio::spawn(async move {
                let exec = runtime.execute_script_for_streaming(
                    stream_id.clone(),
                    format!("overlap_stream_{i}"),
                    script,
                    tx,
                );
                let drain = async {
                    let mut got = Vec::new();
                    while let Some(chunk) = rx.recv().await {
                        got.push(chunk);
                    }
                    got
                };
                let (exec_result, chunks) = tokio::join!(exec, drain);
                (exec_result, chunks)
            }));
        }

        let mut successes = 0usize;
        for handle in join_handles {
            let (exec_result, chunks) = handle.await.expect("join");
            exec_result.expect("stream execute");
            assert!(
                chunks.iter().any(|c| c.as_ref().is_ok_and(|b| !b.is_empty())),
                "expected at least one chunk"
            );
            successes += 1;
        }

        let elapsed = start.elapsed();
        assert_eq!(successes, stream_count);
        // Serial would be ~800ms; overlapped should be near 200ms (+ runtime overhead).
        assert!(
            elapsed < Duration::from_millis(delay_ms * stream_count as u64 / 2 + 400),
            "expected overlapped streams, elapsed={elapsed:?}"
        );
        assert!(
            elapsed >= Duration::from_millis(delay_ms.saturating_sub(50)),
            "elapsed unexpectedly fast: {elapsed:?}"
        );
    }

    #[tokio::test]
    async fn pool_size_two_broadcast_reaches_every_slot() {
        let runtime = Arc::new(JsExecutionRuntime::with_pool_size(None, 2));
        assert_eq!(runtime.pool_size(), 2);

        runtime
            .broadcast_script("pool_init_marker", "globalThis.__rariPoolMarker = 0")
            .await
            .expect("broadcast init");
        runtime
            .broadcast_script(
                "pool_inc_marker",
                "globalThis.__rariPoolMarker = (globalThis.__rariPoolMarker || 0) + 1",
            )
            .await
            .expect("broadcast increment");

        let first = runtime.pick_runtime().await.expect("pick 0");
        let second = runtime.pick_runtime().await.expect("pick 1");
        assert_ne!(first.idx(), second.idx(), "round-robin should yield distinct slots");

        let v0 = first
            .execute_script("read_marker".into(), "globalThis.__rariPoolMarker".into())
            .await
            .expect("read slot 0");
        let v1 = second
            .execute_script("read_marker".into(), "globalThis.__rariPoolMarker".into())
            .await
            .expect("read slot 1");

        assert_eq!(v0.as_i64(), Some(1));
        assert_eq!(v1.as_i64(), Some(1));
    }
}
