use std::time::{SystemTime, UNIX_EPOCH};

use cow_utils::CowUtils;
use rari_error::RariError;
use serde_json::Value;

use super::interface::JsRuntimeInterface;

fn escape_js_string(s: &str) -> String {
    s.cow_replace('\\', "\\\\")
        .cow_replace('"', r#"\""#)
        .cow_replace('\n', "\\n")
        .cow_replace('\r', "\\r")
        .into_owned()
}

fn is_ident_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_' || c == b'$'
}

fn is_ident_continue(c: u8) -> bool {
    is_ident_start(c) || c.is_ascii_digit()
}

#[derive(Clone, Copy)]
enum ScanPrev {
    Start,
    Punct(u8),
    IdentRegexPrefix,
    IdentOther,
    Primary,
}

fn is_regex_prefix_keyword(word: &[u8]) -> bool {
    matches!(
        word,
        b"return"
            | b"typeof"
            | b"case"
            | b"throw"
            | b"delete"
            | b"void"
            | b"new"
            | b"await"
            | b"yield"
            | b"in"
            | b"of"
            | b"instanceof"
            | b"extends"
    )
}

fn is_module_keyword_boundary(bytes: &[u8], start: usize) -> bool {
    if start == 0 {
        return true;
    }
    let prev = bytes[start - 1];
    !is_ident_continue(prev) && prev != b'.'
}

fn is_module_keyword_suffix(bytes: &[u8], end: usize) -> bool {
    let mut j = end;
    while j < bytes.len() && bytes[j].is_ascii_whitespace() {
        j += 1;
    }
    if j < bytes.len() && matches!(bytes[j], b'*' | b'{' | b'"' | b'\'') {
        return true;
    }
    end < bytes.len()
        && bytes[end].is_ascii_whitespace()
        && j < bytes.len()
        && is_ident_start(bytes[j])
}

fn skip_line_comment(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && bytes[i] != b'\n' {
        i += 1;
    }
    i
}

fn skip_block_comment(bytes: &[u8], mut i: usize) -> usize {
    while i + 1 < bytes.len() {
        if bytes[i] == b'*' && bytes[i + 1] == b'/' {
            return i + 2;
        }
        i += 1;
    }
    bytes.len()
}

fn skip_string(bytes: &[u8], start: usize, quote: u8) -> usize {
    let mut i = start + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i = (i + 2).min(bytes.len()),
            c if c == quote => return i + 1,
            b'\n' if quote != b'`' => return i,
            _ => i += 1,
        }
    }
    bytes.len()
}

fn is_regex_start_context(prev: ScanPrev) -> bool {
    match prev {
        ScanPrev::IdentOther | ScanPrev::Primary => false,
        ScanPrev::Punct(c) => !matches!(c, b')' | b']' | b'/') && !c.is_ascii_digit(),
        ScanPrev::Start | ScanPrev::IdentRegexPrefix => true,
    }
}

fn skip_regex_literal(bytes: &[u8], start: usize) -> usize {
    let mut i = start + 1;
    let mut in_class = false;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i = (i + 2).min(bytes.len()),
            b'[' if !in_class => {
                in_class = true;
                i += 1;
            }
            b']' if in_class => {
                in_class = false;
                i += 1;
            }
            b'/' if !in_class => {
                i += 1;
                while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                    i += 1;
                }
                return i;
            }
            b'\n' => return i,
            _ => i += 1,
        }
    }
    bytes.len()
}

fn try_skip_slash(bytes: &[u8], i: usize, prev: ScanPrev) -> Option<(usize, bool)> {
    if i >= bytes.len() || bytes[i] != b'/' {
        return None;
    }
    if i + 1 < bytes.len() && bytes[i + 1] == b'/' {
        return Some((skip_line_comment(bytes, i + 2), false));
    }
    if i + 1 < bytes.len() && bytes[i + 1] == b'*' {
        return Some((skip_block_comment(bytes, i + 2), false));
    }
    if is_regex_start_context(prev) {
        return Some((skip_regex_literal(bytes, i), true));
    }
    None
}

fn skip_template(bytes: &[u8], start: usize) -> usize {
    let mut i = start + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i = (i + 2).min(bytes.len()),
            b'`' => return i + 1,
            b'$' if i + 1 < bytes.len() && bytes[i + 1] == b'{' => {
                i += 2;
                i = skip_template_expression(bytes, i);
            }
            _ => i += 1,
        }
    }
    bytes.len()
}

fn skip_template_expression(bytes: &[u8], mut i: usize) -> usize {
    let mut depth = 1usize;
    let mut prev = ScanPrev::Punct(b'{');
    while i < bytes.len() && depth > 0 {
        let c = bytes[i];
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if let Some((next, ends_like_primary)) = try_skip_slash(bytes, i, prev) {
            if ends_like_primary {
                prev = ScanPrev::Primary;
            }
            i = next;
            continue;
        }
        match c {
            b'\'' | b'"' => {
                i = skip_string(bytes, i, c);
                prev = ScanPrev::Primary;
            }
            b'`' => {
                i = skip_template(bytes, i);
                prev = ScanPrev::Primary;
            }
            b'{' => {
                depth += 1;
                prev = ScanPrev::Punct(b'{');
                i += 1;
            }
            b'}' => {
                depth -= 1;
                prev = ScanPrev::Punct(b'}');
                i += 1;
            }
            _ if is_ident_start(c) => {
                let start = i;
                i += 1;
                while i < bytes.len() && is_ident_continue(bytes[i]) {
                    i += 1;
                }
                prev = if is_regex_prefix_keyword(&bytes[start..i]) {
                    ScanPrev::IdentRegexPrefix
                } else {
                    ScanPrev::IdentOther
                };
            }
            _ => {
                prev = ScanPrev::Punct(c);
                i += 1;
            }
        }
    }
    i
}

pub fn is_esm_code(code: &str) -> bool {
    let bytes = code.as_bytes();
    let mut i = 0;
    let mut prev = ScanPrev::Start;
    while i < bytes.len() {
        let c = bytes[i];

        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        if let Some((next, ends_like_primary)) = try_skip_slash(bytes, i, prev) {
            if ends_like_primary {
                prev = ScanPrev::Primary;
            }
            i = next;
            continue;
        }

        if c == b'\'' || c == b'"' {
            i = skip_string(bytes, i, c);
            prev = ScanPrev::Primary;
            continue;
        }
        if c == b'`' {
            i = skip_template(bytes, i);
            prev = ScanPrev::Primary;
            continue;
        }

        if is_ident_start(c) {
            let start = i;
            i += 1;
            while i < bytes.len() && is_ident_continue(bytes[i]) {
                i += 1;
            }
            let word = &code[start..i];
            if (word == "import" || word == "export")
                && is_module_keyword_boundary(bytes, start)
                && is_module_keyword_suffix(bytes, i)
            {
                return true;
            }
            prev = if is_regex_prefix_keyword(&bytes[start..i]) {
                ScanPrev::IdentRegexPrefix
            } else {
                ScanPrev::IdentOther
            };
            continue;
        }

        prev = ScanPrev::Punct(c);
        i += 1;
    }
    false
}

pub fn invalidate_script_name(component_id: &str) -> String {
    format!("invalidate_{}", component_id.cow_replace('/', "_"))
}

pub fn pending_component_id(component_id: &str) -> String {
    format!("__rari_hmr_pending__:{component_id}")
}

pub fn build_invalidate_script(component_id: &str) -> String {
    let escaped_component_id = escape_js_string(component_id);
    format!(
        r#"(function() {{
                const clear = globalThis['~rari']?.clearHmrComponent;
                if (typeof clear !== 'function') {{
                    throw new Error('clearHmrComponent unavailable');
                }}
                return clear("{escaped_component_id}");
            }})()"#
    )
}

fn build_pending_registration_script(component_id: &str, hmr_specifier: &str) -> String {
    let escaped_component_id = escape_js_string(component_id);
    let escaped_hmr_specifier = escape_js_string(hmr_specifier);
    format!(
        r#"(async function() {{
                    const componentId = "{escaped_component_id}";
                    try {{
                        const moduleNamespace = await import("{escaped_hmr_specifier}");

                        if (!globalThis['~rsc']) globalThis['~rsc'] = {{}};
                        if (!globalThis['~rsc'].hmrPending) globalThis['~rsc'].hmrPending = {{}};

                        let defaultExport = null;
                        if (moduleNamespace.default) {{
                            defaultExport = moduleNamespace.default;
                        }} else {{
                            const exports = Object.values(moduleNamespace).filter(v => typeof v === 'function');
                            if (exports.length > 0) {{
                                defaultExport = exports[0];
                            }}
                        }}

                        const namedExports = {{}};
                        for (const [key, value] of Object.entries(moduleNamespace)) {{
                            if (key !== 'default' && typeof value === 'function') {{
                                namedExports[key] = value;
                            }}
                        }}

                        globalThis['~rsc'].hmrPending[componentId] = {{
                            moduleNamespace,
                            defaultExport,
                            namedExports,
                        }};

                        return {{ success: true, hasDefault: defaultExport != null }};
                    }} catch (error) {{
                        console.error('[rari] Failed to stage pending component ' + componentId + ':', error);
                        return {{ success: false, error: error.message }};
                    }}
                }})()"#
    )
}

fn build_atomic_swap_script(component_id: &str) -> String {
    let escaped_component_id = escape_js_string(component_id);
    format!(
        r#"(function() {{
                    const componentId = "{escaped_component_id}";
                    const pending = globalThis['~rsc']?.hmrPending?.[componentId];
                    if (!pending) {{
                        return {{ success: false, error: 'No pending HMR payload for ' + componentId }};
                    }}

                    if (!globalThis['~rsc']) globalThis['~rsc'] = {{}};
                    if (!globalThis['~rsc'].modules) globalThis['~rsc'].modules = {{}};
                    if (!globalThis['~rsc'].functions) globalThis['~rsc'].functions = {{}};

                    globalThis['~rsc'].modules[componentId] = pending.moduleNamespace;
                    if (pending.defaultExport) {{
                        globalThis[componentId] = pending.defaultExport;
                    }}
                    if (pending.namedExports && Object.keys(pending.namedExports).length > 0) {{
                        globalThis['~rsc'].functions[componentId] = pending.namedExports;
                    }} else {{
                        delete globalThis['~rsc'].functions[componentId];
                    }}

                    delete globalThis['~rsc'].hmrPending[componentId];
                    return {{
                        success: typeof globalThis[componentId] !== 'undefined',
                        componentId,
                    }};
                }})()"#
    )
}

fn build_discard_pending_script(component_id: &str) -> String {
    let escaped_component_id = escape_js_string(component_id);
    format!(
        r#"(function() {{
                    const componentId = "{escaped_component_id}";
                    if (globalThis['~rsc']?.hmrPending?.[componentId]) {{
                        delete globalThis['~rsc'].hmrPending[componentId];
                    }}
                    return {{ success: true }};
                }})()"#
    )
}

async fn discard_pending_component(
    runtime: &dyn JsRuntimeInterface,
    component_id: &str,
) -> Result<(), RariError> {
    let _ = runtime
        .execute_script(
            format!("discard_pending_{}.js", component_id.cow_replace('/', "_")),
            build_discard_pending_script(component_id),
        )
        .await;
    Ok(())
}

async fn load_esm_component_code_atomic(
    runtime: &dyn JsRuntimeInterface,
    component_id: &str,
    component_code: &str,
) -> Result<(), RariError> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();

    let pending_specifier = format!("file:///rari_hmr/pending/{component_id}.js?v={timestamp}");
    let live_specifier = format!("file:///rari_hmr/server/{component_id}.js?v={timestamp}");
    let pending_id = pending_component_id(component_id);

    runtime.add_module_to_loader(&pending_specifier, component_code.to_string()).await.map_err(
        |e| {
            let error_msg =
                format!("Failed to add pending component module for {component_id}: {e}");
            tracing::error!("{}", error_msg);
            RariError::js_execution(error_msg)
        },
    )?;

    let module_id = runtime.load_es_module(&pending_id).await.map_err(|e| {
        let error_msg = format!("Failed to load pending ES module for {component_id}: {e}");
        tracing::error!("{}", error_msg);
        RariError::js_execution(error_msg)
    })?;

    if let Err(e) = runtime.evaluate_module(module_id).await {
        let _ = discard_pending_component(runtime, component_id).await;
        let error_msg = format!("Failed to evaluate pending ES module for {component_id}: {e}");
        tracing::error!("{}", error_msg);
        return Err(RariError::js_execution(error_msg));
    }

    let stage_result = match runtime
        .execute_script(
            format!("stage_pending_{}.js", component_id.cow_replace('/', "_")),
            build_pending_registration_script(component_id, &pending_specifier),
        )
        .await
    {
        Ok(json) => json,
        Err(e) => {
            let _ = discard_pending_component(runtime, component_id).await;
            let error_msg =
                format!("Failed to stage pending component {component_id} to globalThis: {e}");
            tracing::error!("{}", error_msg);
            return Err(RariError::js_execution(error_msg));
        }
    };

    if !stage_result.get("success").and_then(Value::as_bool).unwrap_or(false) {
        let _ = discard_pending_component(runtime, component_id).await;
        let error_msg =
            stage_result.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error");
        tracing::error!("Pending component staging failed for {}: {}", component_id, error_msg);
        return Err(RariError::js_execution(format!(
            "Pending component staging failed for {component_id}: {error_msg}"
        )));
    }

    let swap_result = match runtime
        .execute_script(
            format!("swap_component_{}.js", component_id.cow_replace('/', "_")),
            build_atomic_swap_script(component_id),
        )
        .await
    {
        Ok(json) => json,
        Err(e) => {
            let _ = discard_pending_component(runtime, component_id).await;
            let error_msg = format!("Failed to atomically swap component {component_id}: {e}");
            tracing::error!("{}", error_msg);
            return Err(RariError::js_execution(error_msg));
        }
    };

    if !swap_result.get("success").and_then(Value::as_bool).unwrap_or(false) {
        let _ = discard_pending_component(runtime, component_id).await;
        let error_msg =
            swap_result.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown error");
        return Err(RariError::js_execution(format!(
            "Atomic component swap failed for {component_id}: {error_msg}"
        )));
    }

    if let Err(e) = runtime.clear_module_loader_caches(component_id).await {
        tracing::warn!("Failed to clear module loader caches for {}: {}", component_id, e);
    }
    if let Err(e) = runtime.add_module_to_loader(&live_specifier, component_code.to_string()).await
    {
        tracing::warn!(
            "Failed to promote live component module for {} after successful swap: {}",
            component_id,
            e
        );
    }

    Ok(())
}

pub async fn load_component_code(
    runtime: &dyn JsRuntimeInterface,
    component_id: &str,
    component_code: &str,
) -> Result<(), RariError> {
    if is_esm_code(component_code) {
        return load_esm_component_code_atomic(runtime, component_id, component_code).await;
    }

    let script_name = format!("load_component_{}", component_id.cow_replace('/', "_"));
    match runtime.execute_script(script_name, component_code.to_string()).await {
        Ok(_) => Ok(()),
        Err(e) => {
            let error_msg = format!("Failed to execute component code for {component_id}: {e}");
            tracing::error!("{}", error_msg);
            Err(RariError::js_execution(error_msg))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{build_invalidate_script, is_esm_code};

    #[test]
    fn invalidate_script_invokes_snapshotted_helper() {
        let script = build_invalidate_script(r#"foo"bar"#);
        assert!(script.contains("clearHmrComponent"));
        assert!(script.contains(r#"foo\"bar"#));
        assert!(script.contains("throw new Error"));
        assert!(
            script.len() < 500,
            "per-HMR invalidate script should stay tiny (got {} bytes)",
            script.len()
        );
    }

    #[test]
    fn detects_minified_esm_import_export() {
        let bundled = r#"import{jsx as t}from"react/jsx-runtime";function i(){return t("h1",{children:"hi"})}export{i as default};"#;
        assert!(is_esm_code(bundled));
        assert!(is_esm_code(r#"import"./setup.js""#));
        assert!(is_esm_code("export default function Page() {}"));
        assert!(is_esm_code("  export { foo }"));
        assert!(!is_esm_code("const exportName = 1; function importData() {}"));
        assert!(!is_esm_code("// import { foo } from 'bar'\nfunction x() {}"));
        assert!(!is_esm_code("/* export default 1 */\nfunction x() {}"));
        assert!(!is_esm_code(r#"const s = "import { x } from 'y'""#));
        assert!(!is_esm_code("const s = `export default 1`"));
        assert!(is_esm_code("// just a comment\nexport default function Page() {}"));
        assert!(is_esm_code(r"const re = /\//; export default function Page() {}"));
        assert!(is_esm_code(r#"const re = /["']/; export { foo }"#));
        assert!(is_esm_code(r"const t = `${/\//}`; export default 1"));
        assert!(is_esm_code(r"function f(){return /\//;} export default function Page(){}"));
        assert!(is_esm_code(r"const t = `${(()=>{return /\//;})()}`; export default 1"));
    }
}
