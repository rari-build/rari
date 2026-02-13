use crate::error::RariError;
use serde_json::Value as JsonValue;

pub const CHANNEL_CAPACITY: usize = 32;
pub const RUNTIME_RESTART_DELAY_MS: u64 = 1000;
pub const RUNTIME_QUICK_RESTART_DELAY_MS: u64 = 100;
pub const COMPONENT_PREFIX: &str = "component_";
pub const VERIFY_REGISTRATION_PREFIX: &str = "verify_registration_";
pub const RARI_REGISTER_FUNCTION: &str = "~rari_register";

pub const MODULE_ALREADY_EVALUATED_ERROR: &str = "Module already evaluated";
pub const JS_EXECUTOR_FAILED_ERROR: &str = "JS executor failed to respond";
pub const JS_EXECUTOR_CHANNEL_CLOSED_ERROR: &str = "JS executor channel closed";
pub const RUNTIME_RESTART_MESSAGE: &str =
    "Runtime is being restarted for stability. Please retry your request.";

pub const ENV_INJECTION_SCRIPT: &str = r#"
(() => {
    if (!globalThis.process.env) {
        globalThis.process.env = {};
    }

    const envVars = {};
    Object.assign(globalThis.process.env, envVars);

    return Object.keys(envVars).length;
})();
"#;

pub const MODULE_CHECK_SCRIPT: &str = r#"
(function() {
    if (!globalThis.RscModuleManager) {
        return { available: false, extension: 'rsc_modules' };
    }
    return { available: true, extension: 'rsc_modules' };
})()
"#;

pub const PROMISE_SETUP_SCRIPT: &str = r#"
(function() {
    try {
        if (!globalThis['~promises']) globalThis['~promises'] = {};
        const promise = globalThis['~promises'].currentObject;
        if (!promise || typeof promise.then !== 'function') {
            globalThis['~promises'].resolvedValue = {
                '~error': "Not a valid promise",
                received: typeof promise,
                promiseToString: String(promise)
            };
            globalThis['~promises'].resolutionComplete = true;
            return;
        }

        globalThis['~promises'].resolvedValue = null;
        globalThis['~promises'].resolutionComplete = false;

        promise.then(function(resolvedValue) {
            globalThis['~promises'].resolvedValue = resolvedValue;
            globalThis['~promises'].resolutionComplete = true;
        }).catch(function(error) {
            globalThis['~promises'].resolvedValue = {
                '~promiseError': true,
                error: String(error),
                stack: error.stack || "No stack trace"
            };
            globalThis['~promises'].resolutionComplete = true;
        });
    } catch (error) {
        globalThis['~promises'].resolvedValue = {
            '~promiseError': true,
            error: String(error),
            stack: error.stack || "No stack trace"
        };
        globalThis['~promises'].resolutionComplete = true;
    }
})()
"#;

pub const PROMISE_EXTRACT_SCRIPT: &str = r#"
(function() {
    if (!globalThis['~promises']) globalThis['~promises'] = {};
    if (globalThis['~promises'].resolutionComplete === true) {
        return globalThis['~promises'].resolvedValue;
    } else {
        return {
            '~timeoutError': "Promise did not resolve in time",
            '~debugInfo': {
                completion_flag: globalThis['~promises'].resolutionComplete,
                resolved_value: globalThis['~promises'].resolvedValue
            }
        };
    }
})()
"#;

pub const FETCH_CACHE_INIT_SCRIPT: &str = r#"
(function() {
    if (typeof globalThis === 'undefined' || typeof globalThis.fetch !== 'function') {
        return { installed: false, reason: 'fetch not available' };
    }

    const originalFetch = globalThis.fetch;
    const requestDedupeMap = new Map();

    function generateCacheKey(input, init) {
        const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;
        const method = init?.method || 'GET';

        let headersStr = '{}';
        if (init?.headers) {
            const headerEntries = [];
            const headers = init.headers;

            if (typeof headers.entries === 'function') {
                for (const [name, value] of headers.entries()) {
                    headerEntries.push([name.toLowerCase(), value]);
                }
            } else if (typeof headers.forEach === 'function') {
                headers.forEach((value, name) => {
                    headerEntries.push([name.toLowerCase(), value]);
                });
            } else if (typeof headers === 'object') {
                for (const [name, value] of Object.entries(headers)) {
                    headerEntries.push([name.toLowerCase(), String(value)]);
                }
            }

            headerEntries.sort((a, b) => a[0].localeCompare(b[0]));
            headersStr = JSON.stringify(headerEntries);
        }

        const body = init?.body ? String(init.body) : '';
        return `${method}:${url}:${headersStr}:${body}`;
    }

    function shouldCache(init) {
        if (init?.cache === 'no-store' || init?.cache === 'no-cache') {
            return false;
        }
        const revalidate = init?.rari?.revalidate ?? init?.next?.revalidate;
        if (revalidate === false || revalidate === 0) {
            return false;
        }
        const method = init?.method?.toUpperCase() || 'GET';
        if (method !== 'GET' && method !== 'HEAD') {
            return false;
        }
        return true;
    }

    async function fetchWithRustCache(input, init) {
        const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;
        const options = {};

        if (init?.headers) {
            const headers = new Headers(init.headers);
            const headerPairs = [];
            headers.forEach((value, key) => {
                headerPairs.push([key, value]);
            });
            if (headerPairs.length > 0) {
                options.headers = JSON.stringify(headerPairs);
            }
        }

        const revalidate = init?.rari?.revalidate ?? init?.next?.revalidate;
        if (typeof revalidate === 'number') {
            options.cacheTTLMs = String(revalidate * 1000);
        }

        options.timeout = '5000';

        try {
            const result = await Deno.core.ops.op_fetch_with_cache(url, JSON.stringify(options));
            if (!result.ok) {
                throw new Error(result.error || 'Fetch failed');
            }

            const responseHeaders = new Headers();
            if (result.headers && typeof result.headers === 'object') {
                for (const [name, value] of Object.entries(result.headers)) {
                    responseHeaders.set(name, value);
                }
            }

            if (!responseHeaders.has('content-type')) {
                let detectedType = 'text/plain';
                let detectionMethod = 'default';

                const urlPath = url.split('?')[0].split('#')[0];
                const extensionMatch = urlPath.match(/\.([^./]+)$/);
                const extension = extensionMatch ? extensionMatch[1].toLowerCase() : null;

                if (extension === 'json') {
                    detectedType = 'application/json';
                    detectionMethod = 'extension';
                } else if (extension === 'html' || extension === 'htm') {
                    detectedType = 'text/html';
                    detectionMethod = 'extension';
                } else if (extension === 'xml') {
                    detectedType = 'application/xml';
                    detectionMethod = 'extension';
                } else if (extension === 'txt') {
                    detectedType = 'text/plain';
                    detectionMethod = 'extension';
                } else if (result.body && result.body.length > 0 && result.body.length < 10000) {
                    const trimmed = result.body.trim();
                    if ((trimmed.startsWith('{') && trimmed.endsWith('}')) ||
                        (trimmed.startsWith('[') && trimmed.endsWith(']'))) {
                        detectedType = 'application/json';
                        detectionMethod = 'body-sniff';
                    }
                }

                responseHeaders.set('content-type', detectedType);

                if (typeof console !== 'undefined' && console.debug) {
                    console.debug(`[Fetch Cache] Content-Type fallback: ${detectedType} (${detectionMethod}) for ${url}`);
                }
            }

            return new Response(result.body, {
                status: result.status,
                statusText: result.statusText || 'OK',
                headers: responseHeaders,
            });
        } catch (error) {
            return originalFetch(input, init);
        }
    }

    const cachedFetch = async function(input, init) {
        if (!shouldCache(init)) {
            return originalFetch(input, init);
        }

        const cacheKey = generateCacheKey(input, init);
        const inFlight = requestDedupeMap.get(cacheKey);

        if (inFlight) {
            const response = await inFlight;
            return response.clone();
        }

        const hasRustOp = typeof Deno?.core?.ops?.op_fetch_with_cache === 'function';

        if (hasRustOp) {
            const promise = fetchWithRustCache(input, init);
            requestDedupeMap.set(cacheKey, promise);
            try {
                const response = await promise;
                return response.clone();
            } finally {
                requestDedupeMap.delete(cacheKey);
            }
        } else {
            const promise = originalFetch(input, init);
            requestDedupeMap.set(cacheKey, promise);
            try {
                const response = await promise;
                return response.clone();
            } finally {
                requestDedupeMap.delete(cacheKey);
            }
        }
    };

    globalThis.fetch = cachedFetch;
    globalThis.__rariFetchCacheInstalled = true;

    return { installed: true, hasRustOp: typeof Deno?.core?.ops?.op_fetch_with_cache === 'function' };
})()
"#;

pub fn is_critical_error(error: &RariError) -> bool {
    let error_str = error.to_string();
    error_str.contains("assertion") || error_str.contains("panicked")
}

pub fn is_runtime_restart_needed(error: &RariError) -> bool {
    let error_str = error.to_string();
    error_str.contains(MODULE_ALREADY_EVALUATED_ERROR)
        || error_str.contains(JS_EXECUTOR_FAILED_ERROR)
        || error_str.contains(JS_EXECUTOR_CHANNEL_CLOSED_ERROR)
}

pub fn create_graceful_error() -> RariError {
    RariError::js_runtime(RUNTIME_RESTART_MESSAGE.to_string())
}

#[allow(clippy::disallowed_methods)]
pub fn create_already_evaluated_response(component_name: &str) -> JsonValue {
    serde_json::json!({
        "status": "already_evaluated",
        "component": component_name
    })
}

#[allow(clippy::disallowed_methods)]
pub fn create_already_loaded_response(component_name: &str) -> JsonValue {
    serde_json::json!({
        "status": "already_loaded",
        "component": component_name
    })
}

pub fn create_registration_script(specifier_str: &str, script_name: &str) -> String {
    format!(
        r#"
        (async function() {{
            try {{
                const module = await import("{specifier_str}");
                if (typeof module.{RARI_REGISTER_FUNCTION} === 'function') {{
                    const result = module.{RARI_REGISTER_FUNCTION}.call(module);
                    return {{ success: true, result }};
                }} else {{
                    return {{ success: false, error: 'No {RARI_REGISTER_FUNCTION} function found' }};
                }}
            }} catch (e) {{
                console.error("[rari] Failed to call {RARI_REGISTER_FUNCTION} for '{script_name}': " + e.message);
                return {{ success: false, error: e.message }};
            }}
        }})()
        "#
    )
}
