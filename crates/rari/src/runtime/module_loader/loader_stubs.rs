pub const LOADER_STUB_TEMPLATE: &str = r#"
// Auto-generated loader stub for {component_id}

if (typeof globalThis.registerModule === 'function') {{
    globalThis.registerModule({{}}, '{component_id}');
}}

if (typeof globalThis['~rsc'] === 'undefined') {{
    globalThis['~rsc'] = {{}};
}}

if (typeof globalThis['~rsc'].functions === 'undefined') {{
    globalThis['~rsc'].functions = {{}};
}}

if (typeof globalThis['~rsc'].modules === 'undefined') {{
    globalThis['~rsc'].modules = {{}};
}}

globalThis['~rsc'].modules['{component_id}'] = {{
    '~isLoaderStub': true,
    '~awaitingRegistration': true
}};

export default {{
    '~isLoaderStub': true,
    '~componentId': "{component_id}",
    '~timestamp': Date.now()
}};
"#;

pub const FALLBACK_MODULE_TEMPLATE: &str = r#"
// Dynamic fallback module for: {module_name}

if (typeof globalThis['~rsc'] === 'undefined') {{
    globalThis['~rsc'] = {{}};
}}

if (typeof globalThis['~rsc'].modules === 'undefined') {{
    globalThis['~rsc'].modules = {{}};
}}

globalThis['~rsc'].modules['{module_name}'] = {{
    '~isFallback': true,
    '~timestamp': Date.now()
}};

export default {{
    '~isFallback': true,
    '~module': "{module_name}",
    '~timestamp': Date.now()
}};
"#;

pub fn create_generic_module_stub(module_path: &str) -> String {
    format!(
        r#"
// Generic fallback stub for node module: {module_path}

export default {{
  name: '{module_path}',
  isStub: true
}};
"#
    )
}

pub fn create_component_stub(component_name: &str) -> String {
    format!(
        r#"
// Auto-generated stub for component: {component_name}

const moduleExports = {{
    '~isStub': true,
    '~componentName': "{component_name}",
    '~awaitingRegistration': true
}};

if (typeof globalThis.registerModule === 'function') {{
    globalThis.registerModule(moduleExports, '{component_name}');
}}

if (typeof globalThis['~rsc'] === 'undefined') {{
    globalThis['~rsc'] = {{}};
}}

if (typeof globalThis['~rsc'].functions === 'undefined') {{
    globalThis['~rsc'].functions = {{}};
}}

if (typeof globalThis['~rsc'].modules === 'undefined') {{
    globalThis['~rsc'].modules = {{}};
}}

globalThis['~rsc'].modules['{component_name}'] = moduleExports;

export default moduleExports;
"#
    )
}
