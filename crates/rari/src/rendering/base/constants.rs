use std::time::Duration;

pub const MEMORY_PRESSURE_THRESHOLD: f64 = 0.8;
pub const MEMORY_PRESSURE_RENDER_THRESHOLD_NUM: usize = 8;
pub const MEMORY_PRESSURE_RENDER_THRESHOLD_DEN: usize = 10;
pub const CACHE_CLEANUP_INTERVAL: Duration = Duration::from_millis(10);

pub const MAX_RETRIES: u64 = 3;
pub const RETRY_BASE_DELAY_MS: u64 = 150;
pub const COMPONENT_AVAILABILITY_CHECK_DELAY_MS: u64 = 20;

pub const DEFAULT_MAX_CONCURRENT_RENDERS: usize = 50;
pub const DEFAULT_MAX_RENDER_TIME_MS: u64 = 8000;
pub const DEFAULT_MAX_SCRIPT_EXECUTION_TIME_MS: u64 = 3000;
pub const DEFAULT_MAX_MEMORY_PER_COMPONENT_MB: usize = 50;
pub const DEFAULT_MAX_CACHE_SIZE: usize = 1000;

pub const V8_CACHE_CLEAR_SCRIPT: &str = include_str!("js/v8_cache_clear.ts");
pub const SERVER_ACTION_INVOCATION_SCRIPT: &str = include_str!("js/server_action_invocation.ts");

pub const FIZZ_RENDER_SCRIPT: &str = include_str!("../layout/js/fizz_render.ts");
pub const STREAMING_FIZZ_SCRIPT: &str = include_str!("../layout/js/streaming_fizz.ts");
pub const RSC_RENDERER_SCRIPT: &str = include_str!("js/rsc_renderer.ts");

pub const STREAMING_PIPELINE_READY_CHECK: &str = "typeof globalThis['~rari']?.renderStreamingDocument === 'function' \
        && typeof globalThis['~rari']?.renderStaticDocument === 'function'";

pub const LOAD_FULL_REACT_VENDORS_SCRIPT: &str = r"
(async function() {
    try {
        const [react, reactDomServer, flightClient, flightServer] = await Promise.all([
            import('file:///react_vendor/react.js'),
            import('file:///react_vendor/react-dom-server.js'),
            import('file:///react_vendor/react-server-dom-webpack-client.js'),
            import('file:///react_vendor/react-server-dom-webpack-server.js'),
        ]);
        if (!globalThis.React?.createElement) {
            globalThis.React = react.default && react.default.createElement ? react.default : react;
        }
        globalThis['~reactServer'] = reactDomServer;
        globalThis['~flightClient'] = flightClient;
        globalThis['~reactServerRenderer'] = flightServer;
        return !!(globalThis.React.createElement
            && globalThis['~reactServer'].renderToReadableStream
            && globalThis['~flightClient'].createFromReadableStream
            && globalThis['~reactServerRenderer'].renderToReadableStream);
    } catch (e) {
        console.warn('[rari] Could not load React server modules:', e?.message || e);
        return false;
    }
})()
";

pub const LOAD_RSC_VENDORS_SCRIPT: &str = r"
(async function() {
    const [react, flightServer] = await Promise.all([
        import('file:///react_vendor/react.js'),
        import('file:///react_vendor/react-server-dom-webpack-server.js'),
    ]);
    if (!globalThis.React?.createElement) {
        globalThis.React = react.default && react.default.createElement ? react.default : react;
    }
    globalThis['~reactServerRenderer'] = flightServer;
    return !!(globalThis.React.createElement && globalThis['~reactServerRenderer'].renderToReadableStream);
})()
";

pub const EXTENSION_CHECKS: &str = r"(function () {
  const checks = {};
  checks.rsc_renderer = true;
  if (!globalThis.registerModule)
    throw new Error('RSC Modules extension not loaded');
  checks.rsc_modules = true;
  return {
    initialized: true,
    extensions: checks,
    timestamp: Date.now(),
  };
})();";

pub const BATCH_ERROR_COLLECTION: &str = r"(function () {
  if (!globalThis['~errors'])
    globalThis['~errors'] = {};
  const errors = globalThis['~errors'].batch || [];
  globalThis['~errors'].batch = [];
  return {
    success: errors.length === 0,
    errors,
    timestamp: Date.now(),
  };
})();";

pub const SERVER_FUNCTION_RESOLVER: &str = r"(function () {
  if (!globalThis.ServerFunctions)
    throw new Error('ServerFunctions extension not loaded');
  return globalThis.ServerFunctions.resolve();
})();";

pub fn resolve_server_functions_for_component(component_id: &str) -> String {
    format!(
        r"(async function () {{
  try {{
    if (typeof globalThis.resolveServerFunctionsForComponent === 'function')
      await globalThis.resolveServerFunctionsForComponent('{component_id}');
    return {{ success: true, resolved: true }};
  }} catch (error) {{
    return {{ success: false, error: error.message }};
  }}
}})()"
    )
}

pub fn module_registration_script(module_namespace_json: &str, component_id: &str) -> String {
    format!(
        r"(function () {{
  try {{
    const moduleNamespace = {module_namespace_json};
    if (typeof globalThis.RscModuleManager?.register === 'function') {{
      const result = globalThis.RscModuleManager.register(moduleNamespace, '{component_id}');
      return {{ success: true, module: '{component_id}', exports: result.exportCount }};
    }} else if (typeof globalThis.registerModule === 'function') {{
      const result = globalThis.registerModule(moduleNamespace, '{component_id}');
      return {{ success: true, module: '{component_id}', exports: result.exportCount }};
    }} else {{
      return {{ success: false, error: 'No module registration function available' }};
    }}
  }} catch (error) {{
    return {{ success: false, error: error.message }};
  }}
}})()"
    )
}
