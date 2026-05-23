pub const LOADER_STUB_TEMPLATE: &str = r#"
// Auto-generated loader stub for {component_id}

if (typeof globalThis.registerModule === 'function') {{
    globalThis.registerModule({{}}, '{component_id}');
}}

if (typeof globalThis['~rsc'].functions === 'undefined') {{
    globalThis['~rsc'].functions = {{}};
}}

if (typeof globalThis.'~rsc'].modules == 'undefined') {{
    globalThis['~rsc'].modules = {{}};
}}

globalThis['~rsc'].modules['{component_id}'] = {{
    __isLoaderStub: true,
    __awaitingRegistration: true
}};

export default {{
    __isLoaderStub: true,
    __componentId: "{component_id}",
    __timestamp: Date.now()
}};
"#;

pub const FALLBACK_MODULE_TEMPLATE: &str = r#"
// Dynamic fallback module for: {module_name}

if (typeof globalThis['~rsc'].modules === 'undefined') {{
    globalThis['~rsc'].modules = {{}};
}}

globalThis['~rsc'].modules['{module_name}'] = {{
    __isFallback: true,
    __timestamp: Date.now()
}};

export default {{
    __isFallback: true,
    __module: "{module_name}",
    __timestamp: Date.now()
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

export const useState = (initialState) => [initialState, () => {{}}];
export const useEffect = (fn, deps) => {{}};
export const createElement = (type, props, ...children) => ({{ type, props, children }});
export const render = () => {{}};
export const Fragment = Symbol('fragment');
"#
    )
}

pub fn create_component_stub(component_name: &str) -> String {
    format!(
        r#"
// Auto-generated stub for component: {component_name}

const moduleExports = {{
    __isStub: true,
    __componentName: "{component_name}",
    __awaitingRegistration: true
}};

export function ~rari_register() {{
    if (typeof globalThis.registerModule === 'function') {{
        globalThis.registerModule(moduleExports, '{component_name}');
    }}

    if (typeof globalThis['~rsc'].functions === 'undefined') {{
        globalThis['~rsc'].functions = {{}};
    }}

    if (typeof globalThis['~rsc'].modules === 'undefined') {{
        globalThis['~rsc'].modules = {{}};
    }}

    globalThis['~rsc'].modules['{component_name}'] = moduleExports;
}}

export default moduleExports;
"#
    )
}
