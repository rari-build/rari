pub struct RscJsLoader;

fn create_js_wrapper(_component_id: &str, js_code: &str) -> String {
    format!(
        r#"
        (function() {{
            try {{
                {js_code}
            }} catch (error) {{
                throw error;
            }}
        }})()
        "#
    )
}

impl RscJsLoader {
    pub fn load_component_isolation_with_id(component_id: &str) -> Result<String, &'static str> {
        let template = include_str!("component_isolation.js");
        Ok(template.replace("{component_id}", component_id))
    }

    pub fn load_component_render_with_data(
        component_id: &str,
        component_hash: &str,
        props_json: &str,
    ) -> Result<String, &'static str> {
        let template = include_str!("component_render.js");
        let script = template
            .replace("{component_id}", component_id)
            .replace("{component_hash}", component_hash)
            .replace("{props_json}", props_json);
        Ok(script)
    }

    pub fn create_global_init() -> String {
        r#"
        (function() {
            if (!globalThis.__rari_initialized) {
                globalThis.__rari_initialized = true;
                globalThis.__rari_init_timestamp = Date.now();

                globalThis.__rsc_functions = globalThis.__rsc_functions || {};
                globalThis.__rsc_modules = globalThis.__rsc_modules || {};
                globalThis.__rsc_module_dependencies = globalThis.__rsc_module_dependencies || {};
                globalThis.__rsc_exported_functions = globalThis.__rsc_exported_functions || {};
                globalThis.__rsc_components = globalThis.__rsc_components || {};

                globalThis.__rsc_component_functions = globalThis.__rsc_component_functions || new Map();
                globalThis.__rsc_component_server_functions = globalThis.__rsc_component_server_functions || new Map();
                globalThis.__rsc_component_data = globalThis.__rsc_component_data || new Map();
                globalThis.__rsc_component_namespaces = globalThis.__rsc_component_namespaces || new Map();

                globalThis.__RSC_PROMISE_MAP = globalThis.__RSC_PROMISE_MAP || new Map();
                globalThis.__component_promise_map = globalThis.__component_promise_map || new Map();
                globalThis.__component_specific_promises = globalThis.__component_specific_promises || new Map();
                globalThis.__component_resolved_promises = globalThis.__component_resolved_promises || new Map();
                globalThis.__module_promises = globalThis.__module_promises || new Map();
                globalThis.__resolved_values = globalThis.__resolved_values || new Map();

                globalThis.__component_permissions = globalThis.__component_permissions || new Map();
                globalThis.__rari_manual_register = globalThis.__rari_manual_register || {};

                if (!globalThis.__sanitizeDataForComponent) {
                    globalThis.__sanitizeDataForComponent = function(data, componentId) {
                        return data;
                    };
                }

                if (!globalThis.__checkComponentPermission) {
                    globalThis.__checkComponentPermission = function(componentId, permissionType) {
                        const permissions = globalThis.__component_permissions.get(componentId);
                        if (!permissions) return false;
                        return permissions[permissionType] === true;
                    };
                }

                globalThis.__component_render_log = globalThis.__component_render_log || [];

                return {
                    initialized: true,
                    timestamp: globalThis.__rari_init_timestamp,
                    source: 'init'
                };
            }

            return {
                initialized: false,
                reason: 'already_initialized',
                timestamp: globalThis.__rari_init_timestamp
            };
        })();
        "#.to_string()
    }

    pub fn create_component_environment_setup(component_id: &str) -> String {
        let setup_code = format!(
            r#"
            globalThis.__current_rendering_component = "{component_id}";

            if (globalThis.__rsc_component_data && !globalThis.__rsc_component_data.has("{component_id}")) {{
                globalThis.__rsc_component_data.set("{component_id}", {{
                    promises: new Map(),
                    values: new Map(),
                    renderTime: Date.now(),
                    isolated: true
                }});
            }}

            if (globalThis.__component_permissions) {{
                const componentType = "{component_id}".includes("TestComponent") ? "test" : "generic";
                globalThis.__component_permissions.set("{component_id}", {{
                    canAccessCalculations: true,
                    componentType: componentType
                }});
            }}

            return {{
                componentId: "{component_id}",
                environmentSetup: true,
                timestamp: Date.now(),
                source: 'component_specific_setup'
            }};
            "#
        );

        create_js_wrapper(component_id, &setup_code)
    }

    pub fn create_stub_via_js_function(component_id: &str, stub_type: StubType) -> String {
        let function_name = match stub_type {
            StubType::Component => "createComponentStub",
            StubType::Loader => "createLoaderStub",
        };

        format!(
            r#"
            (function() {{
                if (typeof globalThis.RscModuleManager?.stubs?.{} === 'function') {{
                    return globalThis.RscModuleManager.stubs.{}('{}');
                }} else if (typeof globalThis.{} === 'function') {{
                    return globalThis.{}('{}');
                }} else {{
                    throw new Error('Stub creation function {} not available - RSC modules extension may not be loaded');
                }}
            }})()
            "#,
            match stub_type {
                StubType::Component => "component",
                StubType::Loader => "loader",
            },
            match stub_type {
                StubType::Component => "component",
                StubType::Loader => "loader",
            },
            component_id,
            function_name,
            function_name,
            component_id,
            function_name
        )
    }

    pub fn create_module_operation_script(
        component_id: &str,
        operation: RscModuleOperation,
    ) -> String {
        match operation {
            RscModuleOperation::Load { module_specifier } => {
                format!(
                    r#"
                    (function() {{
                        try {{

                            import("{module_specifier}").then(module => {{
                                try {{
                                    if (typeof globalThis.RscModuleManager?.register === 'function') {{
                                        const result = globalThis.RscModuleManager.register(module, "{component_id}");

                                        let isAsync = false;
                                        if (module.default && typeof module.default === 'function') {{
                                            isAsync = module.default.constructor.name === 'AsyncFunction' ||
                                                     module.default.toString().includes('async function');
                                        }}

                                        return {{ success: true, module: "{component_id}", exports: result.exportCount, async: isAsync }};
                                    }} else if (typeof globalThis.registerModule === 'function') {{
                                        const result = globalThis.registerModule(module, "{component_id}");
                                        return {{ success: true, module: "{component_id}", exports: result.exportCount }};
                                    }}
                                }} catch (registrationError) {{
                                    // Silently handle registration errors
                                }}
                            }}).catch(importError => {{
                                // Silently handle import errors
                            }});

                            return {{ success: true, module: "{component_id}", loading: true }};
                        }} catch (error) {{
                            return {{ success: false, error: error.message }};
                        }}
                    }})()
                    "#
                )
            }
            RscModuleOperation::Register { dependencies_json } => {
                format!(
                    r#"
                    (async function() {{
                        try {{
                            globalThis.__rsc_module_dependencies = globalThis.__rsc_module_dependencies || {{}};
                            globalThis.__rsc_module_dependencies["{component_id}"] = {dependencies_json};

                            globalThis.__rsc_modules = globalThis.__rsc_modules || {{}};
                            if (!globalThis.__rsc_modules["{component_id}"]) {{
                                let componentFunction = null;

                                if (typeof globalThis["{component_id}"] === 'function') {{
                                    componentFunction = globalThis["{component_id}"];
                                }} else if (globalThis.__rsc_functions && typeof globalThis.__rsc_functions["{component_id}"] === 'function') {{
                                    componentFunction = globalThis.__rsc_functions["{component_id}"];
                                }} else if (globalThis.__rsc_functions && typeof globalThis.__rsc_functions["fn_{component_id}"] === 'function') {{
                                    componentFunction = globalThis.__rsc_functions["fn_{component_id}"];
                                }}

                                if (componentFunction) {{
                                    globalThis.__rsc_modules["{component_id}"] = {{ default: componentFunction }};
                                }} else {{
                                    globalThis.__rsc_modules["{component_id}"] = {{}};
                                }}
                            }}

                            for (const dep of {dependencies_json}) {{
                                const depName = dep.split('/').pop().replace(/\.\w+$/, '');

                                let resolvedModule = null;

                                if (globalThis.__rsc_modules && globalThis.__rsc_modules[depName]) {{
                                    resolvedModule = globalThis.__rsc_modules[depName];
                                }}

                                if (!resolvedModule) {{
                                    const pathVariants = [
                                        `lib_${{depName}}`,
                                        `utils_${{depName}}`,
                                        `helpers_${{depName}}`,
                                        depName
                                    ];

                                    for (const variant of pathVariants) {{
                                        if (globalThis.__rsc_modules && globalThis.__rsc_modules[variant]) {{
                                            resolvedModule = globalThis.__rsc_modules[variant];
                                            break;
                                        }}
                                    }}
                                }}

                                if (!globalThis.__rsc_modules[depName]) {{
                                    globalThis.__rsc_modules[depName] = resolvedModule || new Proxy({{}}, {{
                                        get: function(target, prop) {{
                                            if (resolvedModule && resolvedModule[prop]) {{
                                                return resolvedModule[prop];
                                            }}

                                            if (typeof globalThis.RscModuleManager?.getFunction === 'function') {{
                                                const fn = globalThis.RscModuleManager.getFunction(prop);
                                                if (fn) {{
                                                    return fn;
                                                }}
                                            }}

                                            if (typeof globalThis.getServerFunction === 'function') {{
                                                const fn = globalThis.getServerFunction(prop);
                                                if (fn) {{
                                                    return fn;
                                                }}
                                            }}

                                            if (globalThis.__rsc_functions && globalThis.__rsc_functions[prop]) {{
                                                return globalThis.__rsc_functions[prop];
                                            }}

                                            const moduleSpecificKey = `${{depName}}_${{prop}}`;
                                            if (globalThis.__rsc_functions && globalThis.__rsc_functions[moduleSpecificKey]) {{
                                                return globalThis.__rsc_functions[moduleSpecificKey];
                                            }}

                                            return function(...args) {{
                                                throw new Error(`Function ${{prop}} not available in module ${{depName}}`);
                                            }};
                                        }}
                                    }});
                                }}
                            }}

                            if (!globalThis.__rsc_modules["{component_id}"] || Object.keys(globalThis.__rsc_modules["{component_id}"]).length === 0) {{
                                let componentFunction = null;

                                if (typeof globalThis["{component_id}"] === 'function') {{
                                    componentFunction = globalThis["{component_id}"];
                                }} else if (globalThis.__rsc_functions && typeof globalThis.__rsc_functions["{component_id}"] === 'function') {{
                                    componentFunction = globalThis.__rsc_functions["{component_id}"];
                                }} else if (globalThis.__rsc_functions && typeof globalThis.__rsc_functions["fn_{component_id}"] === 'function') {{
                                    componentFunction = globalThis.__rsc_functions["fn_{component_id}"];
                                }}

                                if (componentFunction) {{
                                    globalThis.__rsc_modules["{component_id}"] = {{ default: componentFunction }};
                                }}
                            }}

                            return {{ success: true, component: "{component_id}", dependencies: {dependencies_json}.length }};
                        }} catch (error) {{
                            return {{ success: false, error: error.message }};
                        }}
                    }})()
                    "#
                )
            }
            RscModuleOperation::PostRegister => {
                format!(
                    r#"
                    (function() {{
                        try {{
                            globalThis.__rsc_modules = globalThis.__rsc_modules || {{}};

                            let componentFunction = null;
                            let attempts = 0;
                            const maxAttempts = 5;

                            const findComponent = () => {{
                                if (typeof globalThis["{component_id}"] === 'function') {{
                                    return globalThis["{component_id}"];
                                }} else if (globalThis.__rsc_functions && typeof globalThis.__rsc_functions["{component_id}"] === 'function') {{
                                    return globalThis.__rsc_functions["{component_id}"];
                                }} else if (globalThis.__rsc_functions && typeof globalThis.__rsc_functions["fn_{component_id}"] === 'function') {{
                                    return globalThis.__rsc_functions["fn_{component_id}"];
                                }}
                                return null;
                            }};

                            componentFunction = findComponent();

                            while (!componentFunction && attempts < maxAttempts) {{
                                attempts++;

                                const start = Date.now();
                                while (Date.now() - start < 20) {{
                                }}

                                componentFunction = findComponent();
                                if (componentFunction) {{
                                    break;
                                }}
                            }}

                            if (componentFunction) {{
                                const isAsync = componentFunction.constructor.name === 'AsyncFunction' ||
                                               componentFunction.toString().includes('async function');

                                globalThis.__rsc_modules["{component_id}"] = {{ default: componentFunction }};

                                return {{ success: true, component: "{component_id}", function_found: true, async: isAsync }};
                            }} else {{
                                return {{
                                    success: false,
                                    component: "{component_id}",
                                    function_found: false
                                }};
                            }}
                        }} catch (error) {{
                            return {{ success: false, error: error.message }};
                        }}
                    }})()
                    "#
                )
            }
        }
    }

    pub fn create_html_extraction_script(component_id: &str) -> String {
        let extraction_code = format!(
            r#"
            if (typeof globalThis.__lastRenderResult === 'undefined') {{
                return {{
                    error: true,
                    message: "No rendered HTML available. The component may have suspended.",
                    html: "<div><h2>Component: {component_id}</h2><p>Component suspended or failed to render</p></div>"
                }};
            }}

            const extractedHtml = globalThis.__lastRenderResult && globalThis.__lastRenderResult.html
                ? globalThis.__lastRenderResult.html
                : "<div><h2>{component_id}</h2><p>Failed to extract HTML content</p></div>";

            return {{
                success: true,
                html: extractedHtml,
                debug: {{ component_id: "{component_id}", extractedLength: extractedHtml.length }}
            }};
            "#
        );

        create_js_wrapper(component_id, &extraction_code)
    }

    pub fn create_rsc_extraction_script(component_id: &str) -> String {
        let extraction_code = format!(
            r#"
            if (typeof globalThis.__lastRenderResult === 'undefined') {{
                return {{
                    error: true,
                    message: "No rendered result available. The component may have suspended.",
                    rsc: ["$", "div", null, {{
                        children: [
                            "Component: {component_id}",
                            "Component suspended or failed to render"
                        ]
                    }}]
                }};
            }}

            const renderResult = globalThis.__lastRenderResult;
            let extractedRsc = null;

            if (renderResult && renderResult.rsc) {{
                extractedRsc = renderResult.rsc;
            }} else if (renderResult && renderResult.html) {{
                extractedRsc = ["$", "div", null, {{
                    "data-rsc-component": "{component_id}",
                    children: "Component rendered but RSC data unavailable"
                }}];
            }} else {{
                extractedRsc = ["$", "div", null, {{
                    children: [
                        "Component: {component_id}",
                        "Failed to extract RSC content"
                    ]
                }}];
            }}

            return {{
                success: true,
                rsc: extractedRsc,
                debug: {{
                    component_id: "{component_id}",
                    rscType: typeof extractedRsc,
                    hasRscData: !!renderResult?.rsc,
                    hasHtmlData: !!renderResult?.html,
                    hasRenderResult: !!renderResult,
                    renderResultKeys: renderResult ? Object.keys(renderResult) : []
                }}
            }};
            "#
        );

        create_js_wrapper(component_id, &extraction_code)
    }

    pub fn create_component_verification_script(
        component_id: &str,
        hashed_component_id: &str,
    ) -> String {
        let verification_code = format!(
            r#"
            let isRegistered = false;
            const details = {{
                availableGlobals: [],
                componentName: "{component_id}",
                hashedName: "{hashed_component_id}",
                foundInRegistry: false,
                foundAsGlobal: false,
                foundAsHashedGlobal: false,
                registryContents: []
            }};

            if (globalThis.__rsc_modules) {{
                details.registryContents = Object.keys(globalThis.__rsc_modules);
                if (globalThis.__rsc_modules["{component_id}"]) {{
                    isRegistered = true;
                    details.foundInRegistry = true;
                }}
            }}

            if (typeof globalThis["{component_id}"] === 'function') {{
                isRegistered = true;
                details.foundAsGlobal = true;
            }}

            if (typeof globalThis["{hashed_component_id}"] === 'function') {{
                isRegistered = true;
                details.foundAsHashedGlobal = true;
            }}

            details.availableGlobals = Object.keys(globalThis)
                .filter(k => typeof globalThis[k] === 'function' && k.match(/^[A-Z]/))
                .slice(0, 10);

            return {{
                success: isRegistered,
                details: details
            }};
            "#
        );

        create_js_wrapper(component_id, &verification_code)
    }

    pub fn create_isolation_namespacing_script(component_id: &str) -> String {
        format!(
            r#"
            (function() {{
                if (!globalThis.__rsc_component_namespaces.has("{component_id}")) {{
                    globalThis.__rsc_component_namespaces.set("{component_id}", new Map());
                }}

                return {{
                    componentId: "{component_id}",
                    hasNamespace: globalThis.__rsc_component_namespaces.has("{component_id}")
                }};
            }})();
            "#
        )
    }

    pub fn create_module_loader_check_script() -> String {
        r#"
        (function() {
            return globalThis.RariModuleLoader ? true : false;
        })()
        "#
        .to_string()
    }

    pub fn create_isolation_init_script(component_id: &str) -> String {
        format!(
            r#"
            (function() {{
                globalThis.__current_rendering_component = "{component_id}";

                if (globalThis.__component_promise_map && globalThis.__component_promise_map.has("{component_id}")) {{
                    globalThis.__component_promise_map.set("{component_id}", new Map());
                }}

                return true;
            }})();
            "#
        )
    }
}

#[derive(Debug, Clone)]
pub enum StubType {
    Component,
    Loader,
}

#[derive(Debug, Clone)]
pub enum RscModuleOperation {
    Load { module_specifier: String },
    PostRegister,
    Register { dependencies_json: String },
}
