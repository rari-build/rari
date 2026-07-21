//! React vendor entrypoints for app `file://` modules.
//!
//! Full vendors live as `ext:rari/react/vendor/*` (`lazy_loaded_esm`).
//! deno_core 0.408+ rejects `file://` → `ext:` imports after resolution, so
//! bare `react` / `react-server-dom-webpack/*` resolve to
//! `node:rari/react-vendor/*` shims that re-export the real ext modules
//! (`node:` may import `ext:`).

const VENDOR_MODULES: &[&str] = &[
    "react.js",
    "react-server.js",
    "react-jsx-runtime.js",
    "react-dom.js",
    "react-dom-server.js",
    "react-server-dom-webpack-client.js",
    "react-server-dom-webpack-server.js",
];

pub const NODE_VENDOR_PREFIX: &str = "node:rari/react-vendor/";

pub fn normalize_vendor_module_name(raw: &str) -> Option<String> {
    let name = raw.trim_end_matches(".mjs");
    let name = if name.ends_with(".js") { name.to_string() } else { format!("{name}.js") };
    VENDOR_MODULES.contains(&name.as_str()).then_some(name)
}

pub fn node_vendor_specifier(module_name: &str) -> String {
    format!("{NODE_VENDOR_PREFIX}{module_name}")
}

pub fn reexport_shim_source(module_name: &str) -> Option<String> {
    let module_name = normalize_vendor_module_name(module_name)?;
    let ext = format!("ext:rari/react/vendor/{module_name}");
    Some(format!("export * from \"{ext}\";\nexport {{ default }} from \"{ext}\";\n"))
}
