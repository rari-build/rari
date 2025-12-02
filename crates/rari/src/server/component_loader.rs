use crate::error::RariError;
use crate::rsc::dependency_utils::extract_dependencies;
use crate::rsc::renderer::RscRenderer;
use crate::server::utils::component_utils::{
    has_use_client_directive, has_use_server_directive, wrap_server_action_module,
};
use tracing::{debug, error, info, warn};

const DIST_DIR: &str = "dist";

pub struct ComponentLoader;

impl ComponentLoader {
    pub async fn load_production_components(renderer: &mut RscRenderer) -> Result<(), RariError> {
        info!("Loading production components");

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

        let mut loaded_count = 0;
        for (component_id, component_info) in sorted_components {
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

            match renderer
                .runtime
                .execute_script(
                    format!("load_{}.js", component_id.replace('/', "_")),
                    component_code,
                )
                .await
            {
                Ok(_) => {
                    debug!("Loaded production component: {}", component_id);
                    loaded_count += 1;
                }
                Err(e) => {
                    error!("Failed to load component {}: {}", component_id, e);
                }
            }
        }

        info!("Loaded {} production components", loaded_count);
        Ok(())
    }

    pub async fn load_production_server_actions(
        renderer: &mut RscRenderer,
    ) -> Result<(), RariError> {
        info!("Loading production server actions");

        let actions_dir = std::path::Path::new("dist/server/actions");
        if !actions_dir.exists() {
            debug!("No server actions directory found at dist/server/actions");
            return Ok(());
        }

        let mut loaded_count = 0;
        Self::load_server_actions_from_dir(actions_dir, actions_dir, renderer, &mut loaded_count)
            .await?;

        info!("Loaded {} production server actions", loaded_count);
        Ok(())
    }

    fn load_server_actions_from_dir<'a>(
        dir: &'a std::path::Path,
        base_dir: &'a std::path::Path,
        renderer: &'a mut RscRenderer,
        loaded_count: &'a mut usize,
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
                    Self::load_server_actions_from_dir(&path, base_dir, renderer, loaded_count)
                        .await?;
                } else if path.extension().and_then(|s| s.to_str()) == Some("js") {
                    let action_code = std::fs::read_to_string(&path)
                        .map_err(|e| RariError::io(format!("Failed to read action file: {e}")))?;

                    let relative_path = path.strip_prefix(base_dir).unwrap_or(&path);
                    let action_id = relative_path
                        .to_str()
                        .unwrap_or("unknown")
                        .replace(".js", "")
                        .replace('\\', "/");

                    debug!("Loading production server action: {}", action_id);

                    let wrapped_code = wrap_server_action_module(&action_code, &action_id);

                    match renderer
                        .runtime
                        .execute_script(
                            format!("load_action_{}.js", action_id.replace('/', "_")),
                            wrapped_code,
                        )
                        .await
                    {
                        Ok(_) => {
                            debug!("Successfully loaded production server action: {}", action_id);
                            *loaded_count += 1;
                        }
                        Err(e) => {
                            error!("Failed to load production server action {}: {}", action_id, e);
                        }
                    }
                }
            }

            Ok(())
        })
    }

    pub async fn load_server_actions_from_source(
        renderer: &mut RscRenderer,
    ) -> Result<(), RariError> {
        info!("Loading server actions from source");

        let src_dir = std::path::Path::new("src");
        if !src_dir.exists() {
            debug!("No src directory found, skipping server action loading");
            return Ok(());
        }

        let mut loaded_count = 0;
        Self::scan_for_server_actions(src_dir, renderer, &mut loaded_count).await?;

        info!("Loaded {} server action files", loaded_count);
        Ok(())
    }

    fn scan_for_server_actions<'a>(
        dir: &'a std::path::Path,
        renderer: &'a mut RscRenderer,
        loaded_count: &'a mut usize,
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
                    Self::scan_for_server_actions(&path, renderer, loaded_count).await?;
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

                        debug!("Found server action file: {:?} with ID: {}", path, action_id);

                        let dist_path = std::path::Path::new("dist")
                            .join("server")
                            .join(format!("{}.js", action_id));

                        if dist_path.exists() {
                            match std::fs::read_to_string(&dist_path) {
                                Ok(dist_code) => {
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
                                        Ok(_) => {
                                            debug!(
                                                "Successfully loaded server action: {}",
                                                action_id
                                            );
                                            *loaded_count += 1;
                                        }
                                        Err(e) => {
                                            error!(
                                                "Failed to load server action {}: {}",
                                                action_id, e
                                            );
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
                        } else {
                            debug!("Server action not yet built: {:?}", dist_path);
                        }
                    }
                }
            }

            Ok(())
        })
    }

    pub async fn load_app_router_components(renderer: &mut RscRenderer) -> Result<(), RariError> {
        info!("Loading app router components");

        let server_dir = std::path::Path::new(DIST_DIR).join("server");
        if !server_dir.exists() {
            debug!(
                "No server directory found at {}, skipping app router component loading",
                server_dir.display()
            );
            return Ok(());
        }

        let mut loaded_count = 0;
        Self::load_server_components_recursive(
            &server_dir,
            &server_dir,
            renderer,
            &mut loaded_count,
        )
        .await?;

        info!("Loaded {} app router components", loaded_count);
        Ok(())
    }

    fn load_server_components_recursive<'a>(
        dir: &'a std::path::Path,
        base_dir: &'a std::path::Path,
        renderer: &'a mut RscRenderer,
        loaded_count: &'a mut usize,
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
                    Self::load_server_components_recursive(&path, base_dir, renderer, loaded_count)
                        .await?;
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

                        debug!("Loading server action file: {} from {:?}", relative_str, path);

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
                            Ok(_) => {
                                debug!("Successfully loaded server actions from: {}", relative_str);
                                *loaded_count += 1;
                            }
                            Err(e) => {
                                error!(
                                    "Failed to load server actions from {}: {}",
                                    relative_str, e
                                );
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

                    let component_id = if relative_str.starts_with("app/") {
                        relative_str.clone()
                    } else {
                        relative_path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown")
                            .to_string()
                    };

                    debug!("Loading component: {} from {:?}", component_id, path);

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

                    match renderer
                        .runtime
                        .execute_script(
                            format!("load_{}.js", component_id.replace('/', "_")),
                            transformed_module_code,
                        )
                        .await
                    {
                        Ok(_) => {
                            debug!("Successfully loaded component: {}", component_id);

                            if is_client_component {
                                let mark_client_script = format!(
                                    r#"(function() {{
                                        const comp = globalThis["{}"];
                                        if (comp && typeof comp === 'function') {{
                                            comp.__isClientComponent = true;
                                            comp.__clientComponentId = "{}";
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

                            *loaded_count += 1;
                        }
                        Err(e) => {
                            error!("Failed to execute component {}: {}", component_id, e);
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

    #[allow(dead_code)]
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
