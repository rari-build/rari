use crate::error::RariError;
use crate::rsc::rendering::core::RscRenderer;
use crate::rsc::utils::dependency_utils::extract_dependencies;
use crate::server::utils::component_utils::{
    has_use_client_directive, has_use_server_directive, wrap_server_action_module,
};
use tracing::{error, warn};

const DIST_DIR: &str = "dist";

pub struct ComponentLoader;

impl ComponentLoader {
    pub async fn load_production_components(renderer: &mut RscRenderer) -> Result<(), RariError> {
        let manifest_path = std::path::Path::new("dist/server-manifest.json");
        if !manifest_path.exists() {
            warn!(
                "No server manifest found at {}, production components will not be available",
                manifest_path.display()
            );
            return Ok(());
        }

        let manifest = Self::read_manifest(manifest_path)?;
        let components = Self::parse_manifest_components(&manifest)?;

        let mut sorted_components: Vec<_> = components.iter().collect();
        sorted_components.sort_by_key(|(id, _)| if id.starts_with("components/") { 0 } else { 1 });

        for (component_id, component_info) in sorted_components {
            let module_specifier = component_info.get("moduleSpecifier").and_then(|s| s.as_str());

            let bundle_path =
                component_info.get("bundlePath").and_then(|p| p.as_str()).ok_or_else(|| {
                    RariError::configuration(format!("Component {component_id} missing bundlePath"))
                })?;

            let component_file = std::path::Path::new(DIST_DIR).join(bundle_path);
            if !component_file.exists() {
                error!("Component file not found: {}", component_file.display());
                continue;
            }

            let component_code = std::fs::read_to_string(&component_file)
                .map_err(|_e| RariError::io("Failed to read component file".to_string()))?;

            if let Some(specifier) = module_specifier {
                if let Err(e) = renderer
                    .runtime
                    .add_module_to_loader_only(specifier, component_code.clone())
                    .await
                {
                    error!("Failed to add component {} to module loader: {}", component_id, e);
                    continue;
                }

                match renderer.runtime.load_es_module(component_id).await {
                    Ok(module_id) => {
                        if let Err(e) = renderer.runtime.evaluate_module(module_id).await {
                            error!(
                                "Failed to evaluate module {} (id: {}): {}",
                                component_id, module_id, e
                            );
                            continue;
                        }

                        match renderer.runtime.get_module_namespace(module_id).await {
                            Ok(namespace) => {
                                let export_names: Vec<String> =
                                    if let Some(obj) = namespace.as_object() {
                                        obj.keys()
                                            .filter(|k| *k != "Symbol(Symbol.toStringTag)")
                                            .map(|k| k.to_string())
                                            .collect()
                                    } else {
                                        vec![]
                                    };

                                let export_names_json = serde_json::to_string(&export_names)
                                    .unwrap_or_else(|_| "[]".to_string());
                                let registration_script = format!(
                                    r#"(async function() {{
                                        try {{
                                            const moduleNamespace = await import("{}");
                                            const exportNames = {};

                                            if (moduleNamespace.default) {{
                                                globalThis["{}"] = moduleNamespace.default;
                                            }} else {{
                                                const exports = Object.values(moduleNamespace).filter(v => typeof v === 'function');
                                                if (exports.length > 0) {{
                                                    globalThis["{}"] = exports[0];
                                                }}
                                            }}

                                            for (const [key, value] of Object.entries(moduleNamespace)) {{
                                                if (key !== 'default' && typeof value === 'function') {{
                                                    globalThis[key] = value;
                                                }}
                                            }}

                                            if (!globalThis['~rsc'].modules) {{
                                                globalThis['~rsc'].modules = {{}};
                                            }}
                                            globalThis['~rsc'].modules["{}"] = moduleNamespace;

                                            return {{ success: true, hasDefault: !!moduleNamespace.default, exportCount: exportNames.length }};
                                        }} catch (error) {{
                                            console.error("Failed to register component {}: ", error);
                                            return {{ success: false, error: error.message }};
                                        }}
                                    }})()"#,
                                    specifier,
                                    export_names_json,
                                    component_id,
                                    component_id,
                                    component_id,
                                    component_id
                                );

                                match renderer
                                    .runtime
                                    .execute_script(
                                        format!("register_{}.js", component_id.replace('/', "_")),
                                        registration_script,
                                    )
                                    .await
                                {
                                    Ok(result) => {
                                        if let Some(success) =
                                            result.get("success").and_then(|v| v.as_bool())
                                            && !success
                                        {
                                            error!(
                                                "Failed to register component {} to globalThis: {:?}",
                                                component_id,
                                                result.get("error")
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        error!(
                                            "Failed to register component {} to globalThis: {}",
                                            component_id, e
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                error!(
                                    "Failed to get module namespace for {}: {}",
                                    component_id, e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to load component {} as ESM module: {}", component_id, e);
                    }
                }
            } else {
                error!("Component {} missing moduleSpecifier in manifest.", component_id);
            }
        }

        Ok(())
    }

    pub async fn load_server_actions_from_source(
        renderer: &mut RscRenderer,
    ) -> Result<(), RariError> {
        let src_dir = std::path::Path::new("src");
        if !src_dir.exists() {
            return Ok(());
        }

        Self::scan_for_server_actions(src_dir, renderer).await?;

        Ok(())
    }

    fn scan_for_server_actions<'a>(
        dir: &'a std::path::Path,
        renderer: &'a mut RscRenderer,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), RariError>> + 'a>> {
        Box::pin(async move {
            let entries = std::fs::read_dir(dir).map_err(|e| {
                RariError::io(format!("Failed to read directory {}: {}", dir.display(), e))
            })?;

            for entry in entries {
                let entry = entry
                    .map_err(|e| RariError::io(format!("Failed to read directory entry: {e}")))?;
                let path = entry.path();

                if path.is_dir() {
                    Self::scan_for_server_actions(&path, renderer).await?;
                } else if path
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s == "ts" || s == "tsx" || s == "js" || s == "jsx")
                    .unwrap_or(false)
                {
                    let code = match std::fs::read_to_string(&path) {
                        Ok(c) => c,
                        Err(_) => continue,
                    };

                    if has_use_server_directive(&code) {
                        let src_dir = std::path::Path::new("src");
                        let relative_path = path.strip_prefix(src_dir).unwrap_or(&path);
                        let action_id = relative_path
                            .to_str()
                            .unwrap_or("unknown")
                            .replace(".ts", "")
                            .replace(".tsx", "")
                            .replace(".js", "")
                            .replace(".jsx", "")
                            .replace('\\', "/");

                        let dist_path = std::path::Path::new("dist")
                            .join("server")
                            .join(format!("{}.js", action_id));

                        if dist_path.exists() {
                            match std::fs::read_to_string(&dist_path) {
                                Ok(dist_code) => {
                                    let module_specifier = format!(
                                        "file://{}",
                                        dist_path
                                            .canonicalize()
                                            .unwrap_or(dist_path.clone())
                                            .display()
                                    );

                                    let esm_load_result = renderer
                                        .runtime
                                        .add_module_to_loader_only(
                                            &module_specifier,
                                            dist_code.clone(),
                                        )
                                        .await;

                                    if esm_load_result.is_ok() {
                                        match renderer.runtime.load_es_module(&action_id).await {
                                            Ok(module_id) => {
                                                if let Err(e) = renderer
                                                    .runtime
                                                    .evaluate_module(module_id)
                                                    .await
                                                {
                                                    error!(
                                                        "Failed to evaluate server action module {}: {}",
                                                        action_id, e
                                                    );
                                                } else {
                                                    let registration_script = format!(
                                                        r#"(async function() {{
                                                            try {{
                                                                const moduleNamespace = await import("{}");
                                                                if (!globalThis['~serverFunctions']) {{
                                                                    globalThis['~serverFunctions'] = {{}};
                                                                }}
                                                                if (!globalThis['~serverFunctions'].all) {{
                                                                    globalThis['~serverFunctions'].all = {{}};
                                                                }}
                                                                for (const [key, value] of Object.entries(moduleNamespace)) {{
                                                                    if (typeof value === 'function') {{
                                                                        globalThis['~serverFunctions'].all[key] = value;
                                                                        globalThis[key] = value;
                                                                    }}
                                                                }}
                                                                return {{ success: true }};
                                                            }} catch (error) {{
                                                                console.error("Failed to register server action {}: ", error);
                                                                return {{ success: false, error: error.message }};
                                                            }}
                                                        }})()"#,
                                                        module_specifier, action_id
                                                    );

                                                    if let Err(e) = renderer
                                                        .runtime
                                                        .execute_script(
                                                            format!(
                                                                "register_action_{}.js",
                                                                action_id.replace('/', "_")
                                                            ),
                                                            registration_script,
                                                        )
                                                        .await
                                                    {
                                                        warn!(
                                                            "Failed to register server action {} to globalThis: {}",
                                                            action_id, e
                                                        );
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                error!(
                                                    "Failed to load server action {} as ESM module: {}",
                                                    action_id, e
                                                );
                                            }
                                        }
                                    } else {
                                        let wrapped_code =
                                            wrap_server_action_module(&dist_code, &action_id);
                                        match renderer
                                            .runtime
                                            .execute_script(
                                                format!(
                                                    "load_action_{}.js",
                                                    action_id.replace('/', "_")
                                                ),
                                                wrapped_code,
                                            )
                                            .await
                                        {
                                            Ok(_) => {}
                                            Err(e) => {
                                                error!(
                                                    "Failed to load server action {}: {}",
                                                    action_id, e
                                                );
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!(
                                        "Failed to read built server action {:?}: {}",
                                        dist_path, e
                                    );
                                }
                            }
                        }
                    }
                }
            }

            Ok(())
        })
    }

    pub async fn load_app_router_components(renderer: &mut RscRenderer) -> Result<(), RariError> {
        let server_dir = std::path::Path::new(DIST_DIR).join("server");
        if !server_dir.exists() {
            return Ok(());
        }

        Self::load_server_components_recursive(&server_dir, &server_dir, renderer).await?;

        Ok(())
    }

    fn load_server_components_recursive<'a>(
        dir: &'a std::path::Path,
        base_dir: &'a std::path::Path,
        renderer: &'a mut RscRenderer,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), RariError>> + 'a>> {
        Box::pin(async move {
            let entries = std::fs::read_dir(dir).map_err(|e| {
                RariError::io(format!("Failed to read directory {}: {}", dir.display(), e))
            })?;

            for entry in entries {
                let entry = entry
                    .map_err(|e| RariError::io(format!("Failed to read directory entry: {e}")))?;
                let path = entry.path();

                if path.is_dir() {
                    Self::load_server_components_recursive(&path, base_dir, renderer).await?;
                } else if path.extension().and_then(|s| s.to_str()) == Some("js") {
                    let component_code = std::fs::read_to_string(&path).map_err(|e| {
                        RariError::io(format!("Failed to read component file: {e}"))
                    })?;

                    if has_use_server_directive(&component_code) {
                        let relative_path = path.strip_prefix(base_dir).unwrap_or(&path);
                        let relative_str = relative_path
                            .to_str()
                            .unwrap_or("unknown")
                            .replace(".js", "")
                            .replace('\\', "/");

                        let module_specifier = format!(
                            "file://{}",
                            path.canonicalize().unwrap_or(path.to_path_buf()).display()
                        );

                        let esm_load_result = renderer
                            .runtime
                            .add_module_to_loader_only(&module_specifier, component_code.clone())
                            .await;

                        if esm_load_result.is_ok() {
                            match renderer.runtime.load_es_module(&relative_str).await {
                                Ok(module_id) => {
                                    if let Err(e) =
                                        renderer.runtime.evaluate_module(module_id).await
                                    {
                                        error!(
                                            "Failed to evaluate server action module {}: {}",
                                            relative_str, e
                                        );
                                    } else {
                                        let registration_script = format!(
                                            r#"(async function() {{
                                                try {{
                                                    const moduleNamespace = await import("{}");
                                                    if (!globalThis['~serverFunctions']) {{
                                                        globalThis['~serverFunctions'] = {{}};
                                                    }}
                                                    if (!globalThis['~serverFunctions'].all) {{
                                                        globalThis['~serverFunctions'].all = {{}};
                                                    }}
                                                    for (const [key, value] of Object.entries(moduleNamespace)) {{
                                                        if (typeof value === 'function') {{
                                                            globalThis['~serverFunctions'].all[key] = value;
                                                            globalThis[key] = value;
                                                        }}
                                                    }}
                                                    return {{ success: true }};
                                                }} catch (error) {{
                                                    console.error("Failed to register server action {}: ", error);
                                                    return {{ success: false, error: error.message }};
                                                }}
                                            }})()"#,
                                            module_specifier, relative_str
                                        );

                                        if let Err(e) = renderer
                                            .runtime
                                            .execute_script(
                                                format!(
                                                    "register_{}.js",
                                                    relative_str.replace('/', "_")
                                                ),
                                                registration_script,
                                            )
                                            .await
                                        {
                                            warn!(
                                                "Failed to register server action {} to globalThis: {}",
                                                relative_str, e
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!(
                                        "Failed to load server action {} as ESM module: {}",
                                        relative_str, e
                                    );
                                }
                            }
                        } else {
                            let wrapped_code =
                                wrap_server_action_module(&component_code, &relative_str);
                            match renderer
                                .runtime
                                .execute_script(
                                    format!("load_{}.js", relative_str.replace('/', "_")),
                                    wrapped_code,
                                )
                                .await
                            {
                                Ok(_) => {}
                                Err(e) => {
                                    error!(
                                        "Failed to load server actions from {}: {}",
                                        relative_str, e
                                    );
                                }
                            }
                        }
                        continue;
                    }

                    let relative_path = path.strip_prefix(base_dir).unwrap_or(&path);
                    let relative_str = relative_path
                        .to_str()
                        .unwrap_or("unknown")
                        .replace(".js", "")
                        .replace('\\', "/");

                    let component_id = relative_str.clone();

                    let is_client_component = has_use_client_directive(&component_code);

                    let transformed_module_code = component_code.clone();

                    let dependencies = extract_dependencies(&component_code);

                    {
                        let mut registry = renderer.component_registry.lock();
                        let _ = registry.register_component(
                            &component_id,
                            &component_code,
                            transformed_module_code.clone(),
                            dependencies.clone().into_iter().collect(),
                        );
                    }

                    let module_specifier = format!(
                        "file://{}",
                        path.canonicalize().unwrap_or(path.to_path_buf()).display()
                    );

                    let esm_load_result = renderer
                        .runtime
                        .add_module_to_loader_only(&module_specifier, component_code.clone())
                        .await;

                    if esm_load_result.is_ok() {
                        match renderer.runtime.load_es_module(&component_id).await {
                            Ok(module_id) => {
                                if let Err(e) = renderer.runtime.evaluate_module(module_id).await {
                                    error!(
                                        "Failed to evaluate ESM module {} (id: {}): {}",
                                        component_id, module_id, e
                                    );
                                } else {
                                    let registration_script = format!(
                                        r#"(async function() {{
                                            try {{
                                                const moduleNamespace = await import("{}");

                                                if (moduleNamespace.default) {{
                                                    globalThis["{}"] = moduleNamespace.default;
                                                }} else {{
                                                    const exports = Object.values(moduleNamespace).filter(v => typeof v === 'function');
                                                    if (exports.length > 0) {{
                                                        globalThis["{}"] = exports[0];
                                                    }}
                                                }}

                                                for (const [key, value] of Object.entries(moduleNamespace)) {{
                                                    if (key !== 'default' && typeof value === 'function') {{
                                                        globalThis[key] = value;
                                                    }}
                                                }}

                                                if (!globalThis['~rsc'].modules) {{
                                                    globalThis['~rsc'].modules = {{}};
                                                }}
                                                globalThis['~rsc'].modules["{}"] = moduleNamespace;

                                                return {{ success: true }};
                                            }} catch (error) {{
                                                console.error("Failed to register component {}: ", error);
                                                return {{ success: false, error: error.message }};
                                            }}
                                        }})()"#,
                                        module_specifier,
                                        component_id,
                                        component_id,
                                        component_id,
                                        component_id
                                    );

                                    if let Err(e) = renderer
                                        .runtime
                                        .execute_script(
                                            format!(
                                                "register_{}.js",
                                                component_id.replace('/', "_")
                                            ),
                                            registration_script,
                                        )
                                        .await
                                    {
                                        warn!(
                                            "Failed to register ESM component {} to globalThis: {}",
                                            component_id, e
                                        );
                                    }

                                    if is_client_component {
                                        let mark_client_script = format!(
                                            r#"(function() {{
                                                const comp = globalThis["{}"];
                                                if (comp && typeof comp === 'function') {{
                                                    comp['~isClientComponent'] = true;
                                                    comp['~clientComponentId'] = "{}";
                                                }}
                                                return {{ componentId: "{}", isClient: true }};
                                            }})()"#,
                                            component_id, component_id, component_id
                                        );

                                        if let Err(e) = renderer
                                            .runtime
                                            .execute_script(
                                                format!(
                                                    "mark_client_{}.js",
                                                    component_id.replace('/', "_")
                                                ),
                                                mark_client_script,
                                            )
                                            .await
                                        {
                                            warn!(
                                                "Failed to mark component {} as client: {}",
                                                component_id, e
                                            );
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!(
                                    "Failed to load component {} as ESM module: {}",
                                    component_id, e
                                );
                            }
                        }
                    } else {
                        match renderer
                            .runtime
                            .execute_script(
                                format!("load_{}.js", component_id.replace('/', "_")),
                                transformed_module_code,
                            )
                            .await
                        {
                            Ok(_) => {
                                if is_client_component {
                                    let mark_client_script = format!(
                                        r#"(function() {{
                                            const comp = globalThis["{}"];
                                            if (comp && typeof comp === 'function') {{
                                                comp['~isClientComponent'] = true;
                                                comp['~clientComponentId'] = "{}";
                                            }}
                                            return {{ componentId: "{}", isClient: true }};
                                        }})()"#,
                                        component_id, component_id, component_id
                                    );

                                    if let Err(e) = renderer
                                        .runtime
                                        .execute_script(
                                            format!(
                                                "mark_client_{}.js",
                                                component_id.replace('/', "_")
                                            ),
                                            mark_client_script,
                                        )
                                        .await
                                    {
                                        warn!(
                                            "Failed to mark component {} as client: {}",
                                            component_id, e
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to execute component {}: {}", component_id, e);
                            }
                        }
                    }
                }
            }

            Ok(())
        })
    }

    fn read_manifest(manifest_path: &std::path::Path) -> Result<serde_json::Value, RariError> {
        let manifest_content = std::fs::read_to_string(manifest_path)
            .map_err(|e| RariError::io(format!("Failed to read server manifest: {e}")))?;

        serde_json::from_str(&manifest_content)
            .map_err(|e| RariError::configuration(format!("Failed to parse server manifest: {e}")))
    }

    fn parse_manifest_components(
        manifest: &serde_json::Value,
    ) -> Result<&serde_json::Map<String, serde_json::Value>, RariError> {
        manifest.get("components").and_then(|c| c.as_object()).ok_or_else(|| {
            RariError::configuration("Invalid manifest: missing components".to_string())
        })
    }

    pub async fn load_component_from_manifest(
        component_id: &str,
        component_info: &serde_json::Value,
        renderer: &mut RscRenderer,
    ) -> Result<(), RariError> {
        let bundle_path =
            component_info.get("bundlePath").and_then(|p| p.as_str()).ok_or_else(|| {
                RariError::configuration(format!("Component {component_id} missing bundlePath"))
            })?;

        let component_file = std::path::Path::new(DIST_DIR).join(bundle_path);

        if !component_file.exists() {
            return Err(RariError::not_found(format!(
                "Component file not found: {}",
                component_file.display()
            )));
        }

        let component_code = std::fs::read_to_string(&component_file)
            .map_err(|e| RariError::io(format!("Failed to read component file: {e}")))?;

        renderer
            .register_component(component_id, &component_code)
            .await
            .map_err(|e| RariError::internal(format!("Failed to register component: {e}")))
    }
}
