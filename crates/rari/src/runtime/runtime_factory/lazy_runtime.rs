use crate::error::RariError;
use crate::runtime::runtime_factory::deno_runtime::DenoRuntime;
use crate::runtime::runtime_factory::interface::JsRuntimeInterface;
use rustc_hash::FxHashMap;
use serde_json::Value as JsonValue;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

type LazyRuntimeInner = Arc<Mutex<Option<Box<dyn JsRuntimeInterface>>>>;

struct LazyRuntime {
    inner: LazyRuntimeInner,
    env_vars: Option<FxHashMap<String, String>>,
}

impl LazyRuntime {
    fn new(inner: LazyRuntimeInner, env_vars: Option<FxHashMap<String, String>>) -> Self {
        Self { inner, env_vars }
    }

    async fn with_runtime<F, R>(&self, operation: F) -> Result<R, RariError>
    where
        F: FnOnce(&Box<dyn JsRuntimeInterface>) -> R + Send,
        R: Send,
    {
        self.ensure_initialized().await?;
        let runtime = self.inner.lock().await;
        if let Some(runtime) = &*runtime {
            Ok(operation(runtime))
        } else {
            Err(RariError::js_execution("Runtime not initialized".to_string()))
        }
    }

    async fn ensure_initialized(&self) -> Result<(), RariError> {
        let needs_init = {
            let guard = self.inner.lock().await;
            guard.is_none()
        };

        if needs_init {
            let runtime = DenoRuntime::new(self.env_vars.clone());
            let mut guard = self.inner.lock().await;
            if guard.is_none() {
                *guard = Some(Box::new(runtime));
            }
        }
        Ok(())
    }
}

impl JsRuntimeInterface for LazyRuntime {
    fn execute_script(
        &self,
        script_name: String,
        script_code: String,
    ) -> Pin<Box<dyn Future<Output = Result<JsonValue, RariError>> + Send>> {
        let inner = self.inner.clone();
        let env_vars = self.env_vars.clone();

        Box::pin(async move {
            let self_copy = LazyRuntime::new(inner, env_vars);
            self_copy
                .with_runtime(|runtime| runtime.execute_script(script_name, script_code))
                .await?
                .await
        })
    }

    fn execute_function(
        &self,
        function_name: &str,
        args: Vec<JsonValue>,
    ) -> Pin<Box<dyn Future<Output = Result<JsonValue, RariError>> + Send + 'static>> {
        let inner = self.inner.clone();
        let function_name = function_name.to_string();
        let env_vars = self.env_vars.clone();

        Box::pin(async move {
            let initialized_result = {
                let this = LazyRuntime::new(inner.clone(), env_vars);
                this.ensure_initialized().await
            };

            initialized_result?;

            let runtime = inner.lock().await;

            if let Some(runtime) = &*runtime {
                runtime.execute_function(&function_name, args).await
            } else {
                Err(RariError::js_execution("Runtime not initialized".to_string()))
            }
        })
    }

    fn execute_script_for_streaming(
        &self,
        script_name: String,
        script_code: String,
        chunk_sender: mpsc::Sender<Result<Vec<u8>, String>>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        let inner = self.inner.clone();
        let env_vars = self.env_vars.clone();

        Box::pin(async move {
            let initialized_result = {
                let this = LazyRuntime::new(inner.clone(), env_vars);
                this.ensure_initialized().await
            };

            initialized_result?;

            let runtime = inner.lock().await;

            if let Some(runtime) = &*runtime {
                runtime.execute_script_for_streaming(script_name, script_code, chunk_sender).await
            } else {
                Err(RariError::js_execution("Runtime not initialized".to_string()))
            }
        })
    }

    fn add_module_to_loader(
        &self,
        specifier: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        let inner = self.inner.clone();
        let specifier = specifier.to_string();
        let env_vars = self.env_vars.clone();

        Box::pin(async move {
            let self_copy = LazyRuntime::new(inner, env_vars);
            self_copy.with_runtime(|runtime| runtime.add_module_to_loader(&specifier)).await?.await
        })
    }

    fn load_es_module(
        &self,
        specifier: &str,
    ) -> Pin<Box<dyn Future<Output = Result<deno_core::ModuleId, RariError>> + Send>> {
        let inner = self.inner.clone();
        let specifier = specifier.to_string();
        let env_vars = self.env_vars.clone();

        Box::pin(async move {
            let initialized_result = {
                let this = LazyRuntime::new(inner.clone(), env_vars);
                this.ensure_initialized().await
            };

            initialized_result?;

            let runtime = inner.lock().await;

            if let Some(runtime) = &*runtime {
                runtime.load_es_module(&specifier).await
            } else {
                Err(RariError::js_execution("Runtime not initialized".to_string()))
            }
        })
    }

    fn evaluate_module(
        &self,
        module_id: deno_core::ModuleId,
    ) -> Pin<Box<dyn Future<Output = Result<JsonValue, RariError>> + Send>> {
        let inner = self.inner.clone();
        let env_vars = self.env_vars.clone();

        Box::pin(async move {
            let initialized_result = {
                let this = LazyRuntime::new(inner.clone(), env_vars);
                this.ensure_initialized().await
            };

            initialized_result?;

            let runtime = inner.lock().await;

            if let Some(runtime) = &*runtime {
                runtime.evaluate_module(module_id).await
            } else {
                Err(RariError::js_execution("Runtime not initialized".to_string()))
            }
        })
    }

    fn get_module_namespace(
        &self,
        module_id: deno_core::ModuleId,
    ) -> Pin<Box<dyn Future<Output = Result<JsonValue, RariError>> + Send>> {
        let inner = self.inner.clone();
        let env_vars = self.env_vars.clone();

        Box::pin(async move {
            let initialized_result = {
                let this = LazyRuntime::new(inner.clone(), env_vars);
                this.ensure_initialized().await
            };

            initialized_result?;

            let runtime = inner.lock().await;

            if let Some(runtime) = &*runtime {
                runtime.get_module_namespace(module_id).await
            } else {
                Err(RariError::js_execution("Runtime not initialized".to_string()))
            }
        })
    }

    fn add_module_to_loader_only(
        &self,
        specifier: &str,
        code: String,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        let inner = self.inner.clone();
        let specifier = specifier.to_string();
        let env_vars = self.env_vars.clone();

        Box::pin(async move {
            let initialized_result = {
                let this = LazyRuntime::new(inner.clone(), env_vars);
                this.ensure_initialized().await
            };

            initialized_result?;

            let runtime = inner.lock().await;

            if let Some(runtime) = &*runtime {
                runtime.add_module_to_loader_only(&specifier, code).await
            } else {
                Err(RariError::js_execution("Runtime not initialized".to_string()))
            }
        })
    }

    fn clear_module_loader_caches(
        &self,
        component_id: &str,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        let inner = self.inner.clone();
        let component_id = component_id.to_string();
        let env_vars = self.env_vars.clone();

        Box::pin(async move {
            let initialized_result = {
                let this = LazyRuntime::new(inner.clone(), env_vars);
                this.ensure_initialized().await
            };

            initialized_result?;

            let runtime = inner.lock().await;

            if let Some(runtime) = &*runtime {
                runtime.clear_module_loader_caches(&component_id).await
            } else {
                Err(RariError::js_execution("Runtime not initialized".to_string()))
            }
        })
    }

    fn set_request_context(
        &self,
        request_context: std::sync::Arc<crate::server::middleware::request_context::RequestContext>,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        let inner = self.inner.clone();
        let env_vars = self.env_vars.clone();

        Box::pin(async move {
            let initialized_result = {
                let this = LazyRuntime::new(inner.clone(), env_vars);
                this.ensure_initialized().await
            };

            initialized_result?;

            let runtime = inner.lock().await;

            if let Some(runtime) = &*runtime {
                runtime.set_request_context(request_context).await
            } else {
                Err(RariError::js_execution("Runtime not initialized".to_string()))
            }
        })
    }

    fn clear_request_context(&self) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + Send>> {
        let inner = self.inner.clone();
        let env_vars = self.env_vars.clone();

        Box::pin(async move {
            let initialized_result = {
                let this = LazyRuntime::new(inner.clone(), env_vars);
                this.ensure_initialized().await
            };

            initialized_result?;

            let runtime = inner.lock().await;

            if let Some(runtime) = &*runtime {
                runtime.clear_request_context().await
            } else {
                Err(RariError::js_execution("Runtime not initialized".to_string()))
            }
        })
    }
}

pub fn create_lazy_runtime() -> Box<dyn JsRuntimeInterface> {
    let runtime_instance: LazyRuntimeInner = Arc::new(Mutex::new(None));
    let lazy_runtime = LazyRuntime::new(runtime_instance, None);
    Box::new(lazy_runtime)
}

pub fn create_lazy_runtime_with_env(
    env_vars: FxHashMap<String, String>,
) -> Box<dyn JsRuntimeInterface> {
    let runtime_instance: LazyRuntimeInner = Arc::new(Mutex::new(None));
    let lazy_runtime = LazyRuntime::new(runtime_instance, Some(env_vars));
    Box::new(lazy_runtime)
}
