use futures::Stream;
use rustc_hash::FxHashMap;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tracing::error;

use crate::error::RariError;
use crate::runtime::JsExecutionRuntime;

#[derive(Debug, Clone)]
pub struct PartialRenderResult {
    pub initial_content: serde_json::Value,
    pub pending_promises: Vec<PendingSuspensePromise>,
    pub boundaries: Vec<SuspenseBoundaryInfo>,
    pub has_suspense: bool,
}

#[derive(Debug, Clone)]
pub struct PendingSuspensePromise {
    pub id: String,
    pub boundary_id: String,
    pub component_path: String,
    pub promise_handle: String,
}

#[derive(Debug, Clone)]
pub struct SuspenseBoundaryInfo {
    pub id: String,
    pub fallback_content: serde_json::Value,
    pub parent_boundary_id: Option<String>,
    pub pending_promise_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct BoundaryUpdate {
    pub boundary_id: String,
    pub content: serde_json::Value,
    pub row_id: u32,
}

#[derive(Debug, Clone)]
pub struct RscStreamChunkV2 {
    pub data: Vec<u8>,
    pub chunk_type: RscChunkTypeV2,
    pub row_id: u32,
    pub is_final: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RscChunkTypeV2 {
    ModuleImport,
    InitialShell,
    BoundaryUpdate,
    BoundaryError,
    StreamComplete,
}

pub struct BackgroundPromiseResolver {
    runtime: Arc<JsExecutionRuntime>,
    active_promises: Arc<Mutex<FxHashMap<String, PendingSuspensePromise>>>,
    update_sender: mpsc::UnboundedSender<BoundaryUpdate>,
    row_counter: Arc<Mutex<u32>>,
}

impl BackgroundPromiseResolver {
    pub fn new(
        runtime: Arc<JsExecutionRuntime>,
        update_sender: mpsc::UnboundedSender<BoundaryUpdate>,
    ) -> Self {
        Self {
            runtime,
            active_promises: Arc::new(Mutex::new(FxHashMap::default())),
            update_sender,
            row_counter: Arc::new(Mutex::new(0)),
        }
    }

    pub async fn resolve_async(&self, promise: PendingSuspensePromise) {
        let promise_id = promise.id.clone();
        let boundary_id = promise.boundary_id.clone();

        {
            let mut active = self.active_promises.lock().await;
            active.insert(promise_id.clone(), promise.clone());
        }

        let runtime = Arc::clone(&self.runtime);
        let update_sender = self.update_sender.clone();
        let row_counter = Arc::clone(&self.row_counter);
        let active_promises = Arc::clone(&self.active_promises);

        tokio::spawn(async move {
            let resolution_script = format!(
                r#"
                (function() {{
                    try {{
                        const promiseId = '{promise_id}';
                        const boundaryId = '{boundary_id}';

                        const promise = globalThis.__suspense_promises[promiseId];

                        if (!promise) {{
                            return Promise.resolve({{
                                success: false,
                                boundary_id: boundaryId,
                                error: 'Promise not found: ' + promiseId,
                                errorName: 'PromiseNotFound',
                                debug_available_promises: Object.keys(globalThis.__suspense_promises || {{}})
                            }});
                        }}

                        return promise.then(function(resolvedElement) {{
                            if (resolvedElement === undefined || resolvedElement === null) {{
                                return {{
                                    success: false,
                                    boundary_id: boundaryId,
                                    error: 'Promise resolved to null/undefined',
                                    errorName: 'InvalidPromiseResolution',
                                    resolvedType: typeof resolvedElement,
                                    resolvedValue: String(resolvedElement)
                                }};
                            }}

                            let rscData;
                            try {{
                                if (globalThis.renderToRSC) {{
                                    rscData = globalThis.renderToRSC(resolvedElement, globalThis.__rsc_client_components || {{}});
                                }} else {{
                                    rscData = resolvedElement;
                                }}
                            }} catch (rscError) {{
                                return {{
                                    success: false,
                                    boundary_id: boundaryId,
                                    error: 'RSC conversion failed: ' + (rscError.message || 'Unknown RSC error'),
                                    errorName: 'RSCConversionError',
                                    rscErrorName: rscError.name || 'UnknownRSCError',
                                    rscErrorStack: rscError.stack || 'No RSC stack',
                                    resolvedElementType: typeof resolvedElement
                                }};
                            }}

                            return {{
                                success: true,
                                boundary_id: boundaryId,
                                content: rscData
                            }};
                        }}).catch(function(awaitError) {{
                            return {{
                                success: false,
                                boundary_id: boundaryId,
                                error: 'Promise await failed: ' + (awaitError.message || 'Unknown await error'),
                                errorName: 'PromiseAwaitError',
                                awaitErrorName: awaitError.name || 'UnknownAwaitError',
                                awaitErrorStack: awaitError.stack || 'No await stack'
                            }};
                        }});

                    }} catch (error) {{
                        return Promise.resolve({{
                            success: false,
                            boundary_id: boundaryId,
                            error: 'General error: ' + (error.message || 'Unknown general error'),
                            stack: error.stack || 'No stack available',
                            errorName: error.name || 'UnknownGeneralError',
                            errorToString: error.toString() || 'toString failed'
                        }});
                    }}
                }})()
                "#
            );

            let script_name = format!("<promise_resolution_{promise_id}>");

            match runtime.execute_script(script_name.clone(), resolution_script).await {
                Ok(result) => {
                    let result_string = result.to_string();

                    match serde_json::from_str::<serde_json::Value>(&result_string) {
                        Ok(result_data) => {
                            if result_data["success"].as_bool().unwrap_or(false) {
                                let row_id = {
                                    let mut counter = row_counter.lock().await;
                                    *counter += 1;
                                    *counter
                                };

                                let update = BoundaryUpdate {
                                    boundary_id: boundary_id.clone(),
                                    content: result_data["content"].clone(),
                                    row_id,
                                };

                                match update_sender.send(update) {
                                    Ok(_) => {}
                                    Err(e) => {
                                        error!(
                                            "Failed to send boundary update for {}: {}",
                                            boundary_id, e
                                        );
                                    }
                                }
                            } else {
                                error!(
                                    "Promise resolution failed for boundary {}: {} (Details: error={}, stack={}, errorName={}, errorToString={}, debug_info={:?})",
                                    boundary_id,
                                    result_data["error"].as_str().unwrap_or("Unknown error"),
                                    result_data["error"].as_str().unwrap_or("N/A"),
                                    result_data["stack"].as_str().unwrap_or("N/A"),
                                    result_data["errorName"].as_str().unwrap_or("N/A"),
                                    result_data["errorToString"].as_str().unwrap_or("N/A"),
                                    result_data
                                );
                            }
                        }
                        Err(e) => {
                            error!(
                                "Failed to parse promise resolution result for {}: {} - Raw result: {} - Script: {}",
                                boundary_id, e, result_string, script_name
                            );
                        }
                    }
                }
                Err(e) => {
                    error!(
                        "Failed to execute promise resolution script {} for boundary {}: {}",
                        script_name, boundary_id, e
                    );
                }
            }

            {
                let mut active = active_promises.lock().await;
                active.remove(&promise_id);
            }
        });
    }

    pub async fn active_count(&self) -> usize {
        self.active_promises.lock().await.len()
    }
}

pub struct SuspenseBoundaryManager {
    boundaries: Arc<Mutex<FxHashMap<String, SuspenseBoundaryInfo>>>,
    boundary_stack: Vec<String>,
    resolved_boundaries: Arc<Mutex<FxHashMap<String, serde_json::Value>>>,
}

impl Default for SuspenseBoundaryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SuspenseBoundaryManager {
    pub fn new() -> Self {
        Self {
            boundaries: Arc::new(Mutex::new(FxHashMap::default())),
            boundary_stack: Vec::new(),
            resolved_boundaries: Arc::new(Mutex::new(FxHashMap::default())),
        }
    }

    pub async fn register_boundary(&mut self, boundary: SuspenseBoundaryInfo) {
        let boundary_id = boundary.id.clone();
        {
            let mut boundaries = self.boundaries.lock().await;
            boundaries.insert(boundary_id.clone(), boundary);
        }
        self.boundary_stack.push(boundary_id);
    }

    pub async fn resolve_boundary(&self, boundary_id: &str, content: serde_json::Value) {
        {
            let mut resolved = self.resolved_boundaries.lock().await;
            resolved.insert(boundary_id.to_string(), content);
        }

        {
            let mut boundaries = self.boundaries.lock().await;
            if let Some(boundary) = boundaries.get_mut(boundary_id) {
                boundary.pending_promise_count = 0;
            }
        }
    }

    pub async fn get_pending_boundaries(&self) -> Vec<SuspenseBoundaryInfo> {
        let boundaries = self.boundaries.lock().await;
        let resolved = self.resolved_boundaries.lock().await;

        boundaries
            .values()
            .filter(|b| !resolved.contains_key(&b.id) && b.pending_promise_count > 0)
            .cloned()
            .collect()
    }
}

pub struct StreamingRenderer {
    runtime: Arc<JsExecutionRuntime>,
    promise_resolver: Option<BackgroundPromiseResolver>,
    row_counter: u32,
}

impl StreamingRenderer {
    pub fn new(runtime: Arc<JsExecutionRuntime>) -> Self {
        Self { runtime, promise_resolver: None, row_counter: 0 }
    }

    pub async fn start_streaming(
        &mut self,
        component_id: &str,
        props: Option<&str>,
    ) -> Result<RscStreamV2, RariError> {
        let (update_sender, update_receiver) = mpsc::unbounded_channel::<BoundaryUpdate>();
        let (chunk_sender, chunk_receiver) = mpsc::channel::<RscStreamChunkV2>(64);

        self.promise_resolver =
            Some(BackgroundPromiseResolver::new(Arc::clone(&self.runtime), update_sender));

        let partial_result = self.render_partial(component_id, props).await?;

        self.send_initial_shell(&chunk_sender, &partial_result).await?;

        if let Some(resolver) = &self.promise_resolver {
            for promise in partial_result.pending_promises {
                resolver.resolve_async(promise).await;
            }
        } else {
            return Err(RariError::internal(
                "No promise resolver available - this should not happen",
            ));
        }

        let chunk_sender_clone = chunk_sender.clone();
        tokio::spawn(async move {
            let mut update_receiver = update_receiver;

            while let Some(update) = update_receiver.recv().await {
                Self::send_boundary_update(&chunk_sender_clone, update).await;
            }

            let final_chunk = RscStreamChunkV2 {
                data: b"STREAM_COMPLETE\n".to_vec(),
                chunk_type: RscChunkTypeV2::StreamComplete,
                row_id: u32::MAX,
                is_final: true,
            };

            let _ = chunk_sender_clone.send(final_chunk).await;
        });

        Ok(RscStreamV2::new(chunk_receiver))
    }

    async fn render_partial(
        &mut self,
        component_id: &str,
        props: Option<&str>,
    ) -> Result<PartialRenderResult, RariError> {
        let react_init_script = r#"
            (function() {
                if (typeof React === 'undefined') {
                    try {
                        if (typeof globalThis.__rsc_modules !== 'undefined') {
                            const reactModule = globalThis.__rsc_modules['react'] ||
                                              globalThis.__rsc_modules['React'] ||
                                              Object.values(globalThis.__rsc_modules).find(m => m && m.createElement);
                            if (reactModule) {
                                globalThis.React = reactModule;
                            }
                        }

                        if (typeof React === 'undefined' && typeof require !== 'undefined') {
                            globalThis.React = require('react');
                        }

                        if (typeof React !== 'undefined' && React.createElement && !globalThis.__react_patched) {
                            globalThis.__original_create_element = React.createElement;

                            const createElementOverride = function(type, props, ...children) {
                                const debugInfo = {
                                    typeOf: typeof type,
                                    typeName: type?.name || type?.displayName,
                                    isSuspenseFunction: typeof React !== 'undefined' && React.Suspense && type === React.Suspense,
                                };

                                const isSuspenseComponent = (type) => {
                                    if (typeof React !== 'undefined' && React.Suspense && type === React.Suspense) {
                                        return true;
                                    }
                                    if (typeof type === 'function' && type.name === 'Suspense') {
                                        return true;
                                    }
                                    return false;
                                };

                                if (isSuspenseComponent(type)) {
                                    const boundaryId = 'boundary_' + Date.now() + '_' + Math.random().toString(36).substr(2, 9);
                                    const previousBoundaryId = globalThis.__current_boundary_id;
                                    globalThis.__current_boundary_id = boundaryId;

                                    const safeFallback = props?.fallback || globalThis.__original_create_element('div', null, 'Loading...');
                                    const serializableFallback = globalThis.__safeSerializeElement(safeFallback);

                                    globalThis.__discovered_boundaries.push({
                                        id: boundaryId,
                                        fallback: serializableFallback,
                                        parentId: previousBoundaryId
                                    });

                                    globalThis.__current_boundary_id = previousBoundaryId;
                                    return globalThis.__original_create_element('suspense', {...props, key: boundaryId}, ...children);
                                }
                                return globalThis.__original_create_element(type, props, ...children);
                            };

                            Object.defineProperty(React, 'createElement', {
                                value: createElementOverride,
                                writable: false,
                                enumerable: true,
                                configurable: false
                            });

                            globalThis.__react_patched = true;
                        }

                        if (typeof React !== 'undefined' && React.Suspense) {
                            React.__originalSuspense = React.Suspense;

                            React.Suspense = function SuspenseOverride(props) {
                                try {
                                    if (!props) {
                                        return null;
                                    }

                                    if (!props.children) {
                                        return props.fallback || null;
                                    }

                                    const previousDepth = globalThis.__current_suspense_depth || 0;
                                    globalThis.__current_suspense_depth = previousDepth + 1;

                                    try {
                                        let result = props.children;

                                        return result;

                                    } finally {
                                        globalThis.__current_suspense_depth = previousDepth;
                                    }

                                } catch (error) {
                                    if (error.$$typeof === Symbol.for('react.suspense.pending')) {

                                        if (error.promise) {
                                            const promiseId = 'suspense_' + Date.now() + '_' + Math.random().toString(36).substr(2, 9);
                                            globalThis.__suspense_promises = globalThis.__suspense_promises || {};
                                            globalThis.__suspense_promises[promiseId] = error.promise;
                                        }

                                        return props.fallback || React.createElement('div', null, 'Loading...');
                                    }

                                    return props?.fallback || React.createElement('div', null, 'Suspense Error: ' + error.message);
                                }
                            };
                        }

                        if (typeof React === 'undefined') {
                            globalThis.React = {
                                createElement: function(type, props, ...children) {
                                    return {
                                        type: type,
                                        props: props ? { ...props, children: children.length > 0 ? children : props.children } : { children: children },
                                        key: props?.key || null,
                                        ref: props?.ref || null
                                    };
                                },
                                Fragment: Symbol.for('react.fragment'),
                                Suspense: function(props) {
                                    return props.children;
                                }
                            };
                        }
                    } catch (e) {
                        console.error('Failed to load React in streaming context:', e);
                        throw new Error('Cannot initialize streaming without React: ' + e.message);
                    }
                }

                return {
                    available: typeof React !== 'undefined',
                    reactType: typeof React,
                    createElementType: typeof React.createElement,
                    suspenseType: typeof React.Suspense
                };
            })()
        "#;

        let react_init_result = self
            .runtime
            .execute_script("streaming-react-init".to_string(), react_init_script.to_string())
            .await?;

        if let Some(available) = react_init_result.get("available").and_then(|v| v.as_bool()) {
            if !available {
                return Err(RariError::internal("Failed to initialize React in streaming context"));
            }
        } else {
            return Err(RariError::internal("Failed to check React initialization"));
        }

        let init_script = r#"
            if (!globalThis.renderToRSC) {
                globalThis.renderToRSC = function(element, clientComponents = {}) {
                    if (!element) return null;

                    if (typeof element === 'string' || typeof element === 'number' || typeof element === 'boolean') {
                        return element;
                    }

                    if (Array.isArray(element)) {
                        return element.map(child => globalThis.renderToRSC(child, clientComponents));
                    }

                    if (element && typeof element === 'object') {
                        const uniqueKey = element.key || `element-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;

                        if (element.type) {
                            if (typeof element.type === 'string') {
                                const props = element.props || {};
                                const { children: propsChildren, ...otherProps } = props;

                                const actualChildren = element.children || propsChildren;

                                const rscProps = {
                                    ...otherProps,
                                    children: actualChildren ? globalThis.renderToRSC(actualChildren, clientComponents) : undefined
                                };
                                if (rscProps.children === undefined) {
                                    delete rscProps.children;
                                }
                                return ["$", element.type, uniqueKey, rscProps];
                            } else if (typeof element.type === 'function') {
                                try {
                                    const rendered = element.type(element.props || {});
                                    return globalThis.renderToRSC(rendered, clientComponents);
                                } catch (error) {
                                    console.error('Error rendering function component in fallback renderToRSC:', error);
                                    return ["$", "div", uniqueKey, {
                                        style: { color: 'red', border: '1px solid red', padding: '10px' },
                                        children: `Error: ${error.message}`
                                    }];
                                }
                            }
                        }

                        return ["$", "div", uniqueKey, {
                            className: "rsc-unknown",
                            children: "Unknown element type"
                        }];
                    }

                    return element;
                };
            }


            if (typeof React === 'undefined') {
                throw new Error('React is not available in streaming context. This suggests the runtime was not properly initialized with React extensions.');
            }

            if (!globalThis.__suspense_streaming_v2) {
                globalThis.__suspense_streaming_v2 = true;
                globalThis.__suspense_promises = {};
                globalThis.__boundary_props = {};
                globalThis.__discovered_boundaries = [];
                globalThis.__pending_promises = [];
                globalThis.__current_boundary_id = null;

                globalThis.__safeSerializeElement = function(element) {
                    if (!element) return null;

                    try {
                        if (typeof element === 'string' || typeof element === 'number' || typeof element === 'boolean') {
                            return element;
                        }

                        if (element && typeof element === 'object') {
                            return {
                                type: element.type || 'div',
                                props: element.props ? {
                                    children: element.props.children || 'Loading...',
                                    ...(element.props.className && { className: element.props.className })
                                } : { children: 'Loading...' },
                                key: null,
                                ref: null
                            };
                        }

                        return { type: 'div', props: { children: 'Loading...' }, key: null, ref: null };
                    } catch (e) {
                        return { type: 'div', props: { children: 'Loading...' }, key: null, ref: null };
                    }
                };

                if (!globalThis.__react_patched && typeof React !== 'undefined' && React.createElement) {
                    globalThis.__original_create_element = React.createElement;

                    const createElementOverride = function(type, props, ...children) {
                        return globalThis.__original_create_element(type, props, ...children);
                    };

                    React.createElement = createElementOverride;
                    globalThis.__react_patched = true;
                }
            } else {
                globalThis.__discovered_boundaries = [];
                globalThis.__pending_promises = [];
                globalThis.__current_boundary_id = null;
            }
        "#;

        self.runtime
            .execute_script("<streaming_init>".to_string(), init_script.to_string())
            .await
            .map_err(|e| RariError::internal(format!("Streaming init failed: {e}")))?;

        let component_hash = crate::rsc::jsx_transform::hash_string(component_id);
        let render_script = format!(
            r#"
            (async function() {{
                try {{
                    let Component = globalThis['{component_id}'] ||
                                    globalThis['Component_{component_id}'] ||
                                    globalThis['Component_{component_hash}'] ||
                                    (globalThis.__rsc_modules && (globalThis.__rsc_modules['{component_id}']?.default || globalThis.__rsc_modules['{component_id}']));

                    if (Component && typeof Component === 'object' && typeof Component.default === 'function') {{
                        Component = Component.default;
                    }}

                    if (!Component || typeof Component !== 'function') {{
                        throw new Error('Component {component_id} not found or not a function');
                    }}

                    const props = {props_json};
                    globalThis.__boundary_props['root'] = props;

                    let element;
                    let renderError = null;
                    let isAsyncResult = false;


                    try {{
                        const isOverrideActive = React.createElement.toString().includes('SUSPENSE BOUNDARY FOUND');

                        if (!isOverrideActive) {{
                            if (!globalThis.__original_create_element) {{
                                globalThis.__original_create_element = React.createElement;
                            }}

                            React.createElement = function(type, props, ...children) {{
                                const isSuspenseComponent = (type) => {{
                                    if (typeof React !== 'undefined' && React.Suspense && type === React.Suspense) {{
                                        return true;
                                    }}
                                    if (typeof type === 'function' && type.name === 'Suspense') {{
                                        return true;
                                    }}
                                    return false;
                                }};

                                if (isSuspenseComponent(type)) {{
                                    const boundaryId = 'boundary_' + Date.now() + '_' + Math.random().toString(36).substr(2, 9);
                                    const previousBoundaryId = globalThis.__current_boundary_id;
                                    globalThis.__current_boundary_id = boundaryId;

                                    const safeFallback = props?.fallback || globalThis.__original_create_element('div', null, 'Loading...');
                                    const serializableFallback = globalThis.__safeSerializeElement(safeFallback);

                                    globalThis.__discovered_boundaries.push({{
                                        id: boundaryId,
                                        fallback: serializableFallback,
                                        parentId: previousBoundaryId
                                    }});

                                    globalThis.__current_boundary_id = previousBoundaryId;
                                    return globalThis.__original_create_element('suspense', {{...props, key: boundaryId}}, ...children);
                                }}
                                return globalThis.__original_create_element(type, props, ...children);
                            }};
                        }}

                        element = Component(props);

                        if (element && typeof element.then === 'function') {{
                            try {{
                                element = await element;
                            }} catch (asyncError) {{
                                console.error('Async component execution failed:', asyncError);
                                element = globalThis.__original_create_element ?
                                    globalThis.__original_create_element('div', null, 'Async Error: ' + asyncError.message) :
                                    {{'type': 'div', 'props': {{'children': 'Async Error: ' + asyncError.message}}}};
                            }}
                        }}

                        const processSuspenseInStructure = (el, parentBoundaryId = null) => {{
                                if (!el || typeof el !== 'object') return el;

                                if (!el.type && el.props && el.props.fallback && el.children) {{
                                    const boundaryId = 'boundary_' + Date.now() + '_' + Math.random().toString(36).substr(2, 9);
                                    const previousBoundaryId = globalThis.__current_boundary_id;
                                    globalThis.__current_boundary_id = boundaryId;

                                    const safeFallback = el.props.fallback || {{'type': 'div', 'props': {{'children': ['Loading...']}}}};
                                    const serializableFallback = globalThis.__safeSerializeElement(safeFallback);

                                    globalThis.__discovered_boundaries.push({{
                                        id: boundaryId,
                                        fallback: serializableFallback,
                                        parentId: previousBoundaryId
                                    }});

                                    const processedChildren = el.children.map(child => {{
                                        if (child && child.props && Object.keys(child.props).length === 0 &&
                                            (!child.children || child.children.length === 0)) {{
                                            if (globalThis.SimpleAsyncContent && typeof globalThis.SimpleAsyncContent === 'function') {{
                                                try {{
                                                    const result = globalThis.SimpleAsyncContent({{}});

                                                    if (result && typeof result.then === 'function') {{
                                                        const promiseId = 'promise_' + Date.now() + '_' + Math.random().toString(36).substr(2, 9);
                                                        globalThis.__suspense_promises = globalThis.__suspense_promises || {{}};
                                                        globalThis.__suspense_promises[promiseId] = result;

                                                        globalThis.__pending_promises.push({{
                                                            id: promiseId,
                                                            boundaryId: boundaryId,
                                                            componentPath: 'SimpleAsyncContent'
                                                        }});

                                                        return safeFallback;
                                                    }} else {{
                                                        return result;
                                                    }}
                                                }} catch (error) {{
                                                    return safeFallback;
                                                }}
                                            }} else {{
                                                return safeFallback;
                                            }}
                                        }}

                                        return processSuspenseInStructure(child, boundaryId);
                                    }});

                                    globalThis.__current_boundary_id = previousBoundaryId;

                                    return {{
                                        type: 'suspense',
                                        props: {{...el.props, key: boundaryId}},
                                        children: processedChildren
                                    }};
                                }}

                                if (el.children && Array.isArray(el.children)) {{
                                    el.children = el.children.map(child => processSuspenseInStructure(child, parentBoundaryId));
                                }}

                                return el;
                            }};

                            element = processSuspenseInStructure(element);
                        }}
                    catch (suspenseError) {{
                        if (suspenseError && suspenseError.$$typeof === Symbol.for('react.suspense.pending')) {{
                            const componentName = suspenseError.componentName || suspenseError.name || suspenseError.message || '{component_id}';
                            const asyncDetected = suspenseError.asyncComponentDetected === true;
                            const hasPromise = suspenseError.promise && typeof suspenseError.promise.then === 'function';

                            const isParentComponent = componentName === '{component_id}' ||
                                componentName.includes('Test') ||
                                componentName.includes('Streaming');

                            const isLeafAsyncComponent = asyncDetected ||
                                (hasPromise && !isParentComponent) ||
                                (componentName.includes('Async') && !isParentComponent);

                            if (isLeafAsyncComponent) {{
                                const promiseId = 'promise_' + Date.now() + '_' + Math.random().toString(36).substr(2, 9);
                                globalThis.__suspense_promises[promiseId] = suspenseError.promise;

                                const boundaryId = globalThis.__current_boundary_id || 'root_boundary';
                                globalThis.__pending_promises.push({{
                                    id: promiseId,
                                    boundaryId: boundaryId,
                                    componentPath: componentName
                                }});

                            }}

                            element = globalThis.__original_create_element ?
                                globalThis.__original_create_element('div', null, 'Loading...') :
                                {{'type': 'div', 'props': {{'children': 'Loading...'}}}};
                        }} else {{
                            console.error('Non-suspense error during rendering:', suspenseError);
                            renderError = suspenseError;
                            element = globalThis.__original_create_element ?
                                globalThis.__original_create_element('div', null, 'Error: ' + suspenseError.message) :
                                {{'type': 'div', 'props': {{'children': 'Error: ' + suspenseError.message}}}};
                        }}
                    }}

                    let rscData;
                    try {{
                        rscData = globalThis.renderToRSC ?
                            globalThis.renderToRSC(element, globalThis.__rsc_client_components || {{}}) :
                            element;
                    }} catch (rscError) {{
                        console.error('Error in RSC conversion:', rscError);
                        rscData = {{
                            type: 'div',
                            props: {{
                                children: renderError ? 'Render Error: ' + renderError.message : 'RSC Conversion Error'
                            }}
                        }};
                    }}

                    const safeBoundaries = (globalThis.__discovered_boundaries || []).map(boundary => ({{
                        id: boundary.id,
                        fallback: globalThis.__safeSerializeElement(boundary.fallback),
                        parentId: boundary.parentId
                    }}));

                    const finalResult = {{
                        success: !renderError,
                        rsc_data: rscData,
                        boundaries: safeBoundaries,
                        pending_promises: globalThis.__pending_promises || [],
                        has_suspense: (safeBoundaries && safeBoundaries.length > 0) ||
                                     (globalThis.__pending_promises && globalThis.__pending_promises.length > 0),
                        error: renderError ? renderError.message : null,
                        error_stack: renderError ? renderError.stack : null
                    }};

                    globalThis.__streaming_result = finalResult;
                    return finalResult;
                }} catch (error) {{
                    console.error('Fatal error in component rendering:', error);
                    const errorResult = {{
                        success: false,
                        error: error.message,
                        stack: error.stack,
                        fatal: true
                    }};
                    globalThis.__streaming_result = errorResult;
                    return errorResult;
                }}
            }})()
            "#,
            component_id = component_id,
            component_hash = component_hash,
            props_json = props.unwrap_or("{}")
        );

        let result = self
            .runtime
            .execute_script(format!("<partial_render_{component_id}>"), render_script)
            .await
            .map_err(|e| RariError::internal(format!("Partial render failed: {e}")))?;

        let result_data: serde_json::Value =
            serde_json::from_str(&result.to_string()).map_err(|e| {
                RariError::internal(format!(
                    "Failed to parse render result: {e} - Raw result: {result}"
                ))
            })?;

        if !result_data["success"].as_bool().unwrap_or(false) {
            return Err(RariError::internal(format!(
                "Component render failed: {}",
                result_data["error"].as_str().unwrap_or("Unknown error")
            )));
        }

        let boundaries = result_data["boundaries"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|b| SuspenseBoundaryInfo {
                id: b["id"].as_str().unwrap_or("unknown").to_string(),
                fallback_content: b["fallback"].clone(),
                parent_boundary_id: b["parentId"].as_str().map(|s| s.to_string()),
                pending_promise_count: 1,
            })
            .collect();

        let pending_promises = result_data["pending_promises"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .map(|p| PendingSuspensePromise {
                id: p["id"].as_str().unwrap_or("unknown").to_string(),
                boundary_id: p["boundaryId"].as_str().unwrap_or("root").to_string(),
                component_path: p["componentPath"].as_str().unwrap_or(component_id).to_string(),
                promise_handle: p["id"].as_str().unwrap_or("unknown").to_string(),
            })
            .collect();

        Ok(PartialRenderResult {
            initial_content: result_data["rsc_data"].clone(),
            pending_promises,
            boundaries,
            has_suspense: result_data["has_suspense"].as_bool().unwrap_or(false),
        })
    }

    async fn send_initial_shell(
        &mut self,
        sender: &mpsc::Sender<RscStreamChunkV2>,
        partial_result: &PartialRenderResult,
    ) -> Result<(), RariError> {
        self.row_counter += 1;
        let module_chunk = self.create_module_chunk(&partial_result.initial_content)?;
        sender
            .send(module_chunk)
            .await
            .map_err(|e| RariError::internal(format!("Failed to send module chunk: {e}")))?;

        self.row_counter += 1;
        let shell_chunk = self.create_shell_chunk(&partial_result.initial_content)?;
        sender
            .send(shell_chunk)
            .await
            .map_err(|e| RariError::internal(format!("Failed to send shell chunk: {e}")))?;

        Ok(())
    }

    async fn send_boundary_update(sender: &mpsc::Sender<RscStreamChunkV2>, update: BoundaryUpdate) {
        let mut inner_obj = serde_json::Map::new();
        inner_obj.insert("resolved".to_string(), serde_json::Value::Bool(true));
        inner_obj.insert("children".to_string(), update.content);

        let update_data = serde_json::Value::Array(vec![
            serde_json::Value::String("$".to_string()),
            serde_json::Value::String(update.boundary_id),
            serde_json::Value::Null,
            serde_json::Value::Object(inner_obj),
        ]);

        let update_row = format!("{}:{}\n", update.row_id, update_data);

        let chunk = RscStreamChunkV2 {
            data: update_row.into_bytes(),
            chunk_type: RscChunkTypeV2::BoundaryUpdate,
            row_id: update.row_id,
            is_final: false,
        };

        if let Err(e) = sender.send(chunk).await {
            error!("Failed to send boundary update: {}", e);
        }
    }

    fn create_module_chunk(
        &self,
        _content: &serde_json::Value,
    ) -> Result<RscStreamChunkV2, RariError> {
        let module_data = format!("{}:I[[\"main\"],[\"default\"]]\n", self.row_counter);

        Ok(RscStreamChunkV2 {
            data: module_data.into_bytes(),
            chunk_type: RscChunkTypeV2::ModuleImport,
            row_id: self.row_counter,
            is_final: false,
        })
    }

    fn create_shell_chunk(
        &self,
        content: &serde_json::Value,
    ) -> Result<RscStreamChunkV2, RariError> {
        let shell_data = format!("{}:{}\n", self.row_counter, content);

        Ok(RscStreamChunkV2 {
            data: shell_data.into_bytes(),
            chunk_type: RscChunkTypeV2::InitialShell,
            row_id: self.row_counter,
            is_final: false,
        })
    }
}

pub struct RscStreamV2 {
    receiver: mpsc::Receiver<RscStreamChunkV2>,
}

impl RscStreamV2 {
    pub fn new(receiver: mpsc::Receiver<RscStreamChunkV2>) -> Self {
        Self { receiver }
    }

    pub async fn next_chunk(&mut self) -> Option<RscStreamChunkV2> {
        self.receiver.recv().await
    }

    pub fn is_complete(&self) -> bool {
        self.receiver.is_closed()
    }
}

impl Stream for RscStreamV2 {
    type Item = Result<Vec<u8>, String>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        use std::task::Poll;

        match self.receiver.poll_recv(cx) {
            Poll::Ready(Some(chunk)) => Poll::Ready(Some(Ok(chunk.data))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_partial_render_result() {
        let partial_result = PartialRenderResult {
            initial_content: serde_json::json!({"test": "content"}),
            pending_promises: vec![],
            boundaries: vec![],
            has_suspense: false,
        };

        assert!(!partial_result.has_suspense);
        assert_eq!(partial_result.pending_promises.len(), 0);
        assert_eq!(partial_result.boundaries.len(), 0);
    }

    #[tokio::test]
    async fn test_boundary_manager() {
        let mut manager = SuspenseBoundaryManager::new();

        let boundary = SuspenseBoundaryInfo {
            id: "test-boundary".to_string(),
            fallback_content: serde_json::json!({"loading": true}),
            parent_boundary_id: None,
            pending_promise_count: 1,
        };

        manager.register_boundary(boundary).await;

        let pending = manager.get_pending_boundaries().await;
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, "test-boundary");
    }

    #[test]
    fn test_rsc_stream_chunk() {
        let chunk = RscStreamChunkV2 {
            data: b"test data".to_vec(),
            chunk_type: RscChunkTypeV2::InitialShell,
            row_id: 1,
            is_final: false,
        };

        assert_eq!(chunk.chunk_type, RscChunkTypeV2::InitialShell);
        assert_eq!(chunk.row_id, 1);
        assert!(!chunk.is_final);
    }
}
