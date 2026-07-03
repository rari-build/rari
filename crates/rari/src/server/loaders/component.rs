#![expect(clippy::missing_errors_doc, clippy::too_many_lines)]
use std::{future::Future, path::Path, pin::Pin, sync::Arc};

use cow_utils::CowUtils;
use rari_error::RariError;
use rari_rsc::utils::extract_dependencies;
use rari_utils::path_to_file_url;
use serde_json::Value;
use tokio::fs;

use crate::{
    rendering::base::RscRenderer,
    runtime::JsExecutionRuntime,
    server::core::utils::component::{
        has_use_client_directive, has_use_server_directive, wrap_server_action_module,
    },
};

const DIST_DIR: &str = "dist";

#[non_exhaustive]
pub struct ComponentLoader;

impl ComponentLoader {
    pub async fn load_production_components(renderer: &mut RscRenderer) -> Result<(), RariError> {
        let manifest_path = Path::new(DIST_DIR).join("server").join("manifest.json");
        if !fs::try_exists(&manifest_path).await.unwrap_or(false) {
            return Ok(());
        }

        let manifest = Self::read_manifest(&manifest_path).await?;
        let components = Self::parse_manifest_components(&manifest)?;

        let mut sorted_components: Vec<_> = components.iter().collect();
        sorted_components.sort_by_key(|(id, _)| i32::from(!id.starts_with("components/")));

        for (component_id, component_info) in sorted_components {
            if component_id == "proxy" || component_id.ends_with("/proxy") {
                continue;
            }

            let module_specifier = component_info.get("moduleSpecifier").and_then(|s| s.as_str());

            let bundle_path =
                component_info.get("bundlePath").and_then(|p| p.as_str()).ok_or_else(|| {
                    RariError::configuration(format!("Component {component_id} missing bundlePath"))
                })?;

            let component_file = Path::new(DIST_DIR).join(bundle_path);
            if !fs::try_exists(&component_file).await.unwrap_or(false) {
                tracing::error!("Component file not found: {}", component_file.display());
                continue;
            }

            let component_code = fs::read_to_string(&component_file)
                .await
                .map_err(|_e| RariError::io("Failed to read component file".to_string()))?;

            let is_server_action = has_use_server_directive(&component_code);

            if let Some(specifier) = module_specifier {
                if let Err(e) =
                    renderer.runtime.add_module_to_loader(specifier, component_code.clone()).await
                {
                    tracing::error!(
                        "Failed to add component {} to module loader: {}",
                        component_id,
                        e
                    );
                    continue;
                }

                match renderer.runtime.load_es_module(component_id).await {
                    Ok(module_id) => {
                        if let Err(e) = renderer.runtime.evaluate_module(module_id).await {
                            tracing::error!(
                                "Failed to evaluate module {} (id: {}): {}",
                                component_id,
                                module_id,
                                e
                            );
                            continue;
                        }

                        match renderer.runtime.get_module_namespace(module_id).await {
                            Ok(_namespace) => {
                                let specifier_json = serde_json::to_string(specifier)
                                    .unwrap_or_else(|e| {
                                        tracing::error!(
                                            "Failed to serialize module specifier for {}: {}",
                                            component_id,
                                            e
                                        );
                                        "\"\"".to_string()
                                    });
                                let component_id_json = serde_json::to_string(component_id)
                                    .unwrap_or_else(|e| {
                                        tracing::error!(
                                            "Failed to serialize component_id {}: {}",
                                            component_id,
                                            e
                                        );
                                        "\"\"".to_string()
                                    });

                                if is_server_action {
                                    let action_registration_script = format!(
                                        r#"(async function() {{
                                            try {{
                                                const moduleNamespace = await import({specifier_json});
                                                if (!globalThis['~serverFunctions']) {{
                                                    globalThis['~serverFunctions'] = {{}};
                                                }}
                                                if (!globalThis['~serverFunctions'].all) {{
                                                    globalThis['~serverFunctions'].all = {{}};
                                                }}
                                                if (!globalThis['~serverFunctions'].exported) {{
                                                    globalThis['~serverFunctions'].exported = {{}};
                                                }}
                                                for (const [key, value] of Object.entries(moduleNamespace)) {{
                                                    if (typeof value === 'function') {{
                                                        const moduleKey = {component_id_json} + ':' + key;
                                                        globalThis['~serverFunctions'].all[moduleKey] = value;
                                                        globalThis['~serverFunctions'].exported[moduleKey] = value;
                                                    }}
                                                }}
                                                return {{ success: true }};
                                            }} catch (error) {{
                                                console.error("Failed to register server action " + {component_id_json}, error);
                                                return {{ success: false, error: error.message }};
                                            }}
                                        }})()"#
                                    );

                                    match renderer
                                        .runtime
                                        .execute_script(
                                            format!(
                                                "register_action_{}.js",
                                                component_id.cow_replace('/', "_")
                                            ),
                                            action_registration_script,
                                        )
                                        .await
                                    {
                                        Ok(result) => {
                                            if let Some(success) = result
                                                .get("success")
                                                .and_then(serde_json::Value::as_bool)
                                                && !success
                                            {
                                                tracing::error!(
                                                    "Failed to register server action {}: {:?}",
                                                    component_id,
                                                    result.get("error")
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                "Failed to register server action {}: {}",
                                                component_id,
                                                e
                                            );
                                        }
                                    }
                                }

                                if !is_server_action {
                                    let skip_global_binding = component_id.starts_with("lib/");
                                    let registration_script = format!(
                                        r"globalThis['~rari'].componentLoader.registerComponent({specifier_json}, {component_id_json}, {skip_global_binding})"
                                    );

                                    match renderer
                                        .runtime
                                        .execute_script(
                                            format!(
                                                "register_{}.js",
                                                component_id.cow_replace('/', "_")
                                            ),
                                            registration_script,
                                        )
                                        .await
                                    {
                                        Ok(result) => {
                                            if let Some(success) = result
                                                .get("success")
                                                .and_then(serde_json::Value::as_bool)
                                                && !success
                                            {
                                                tracing::error!(
                                                    "Failed to register component {} to globalThis: {:?}",
                                                    component_id,
                                                    result.get("error")
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                "Failed to register component {} to globalThis: {}",
                                                component_id,
                                                e
                                            );
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to get module namespace for {}: {}",
                                    component_id,
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to load component {} as ESM module: {}",
                            component_id,
                            e
                        );
                    }
                }
            } else {
                tracing::error!("Component {} missing moduleSpecifier in manifest.", component_id);
            }
        }

        Ok(())
    }

    pub async fn load_server_actions_from_source(
        renderer: &mut RscRenderer,
    ) -> Result<(), RariError> {
        let src_dir = Path::new("src");
        if !fs::try_exists(src_dir).await.unwrap_or(false) {
            return Ok(());
        }

        Self::scan_for_server_actions(src_dir, renderer).await?;

        Ok(())
    }

    fn scan_for_server_actions<'a>(
        dir: &'a Path,
        renderer: &'a mut RscRenderer,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + 'a>> {
        Box::pin(async move {
            let mut entries = fs::read_dir(dir).await.map_err(|e| {
                RariError::io(format!("Failed to read directory {}: {}", dir.display(), e))
            })?;

            while let Some(entry) = entries
                .next_entry()
                .await
                .map_err(|e| RariError::io(format!("Failed to read directory entry: {e}")))?
            {
                let path = entry.path();
                let file_type = entry
                    .file_type()
                    .await
                    .map_err(|e| RariError::io(format!("Failed to read file type: {e}")))?;

                if file_type.is_dir() {
                    Self::scan_for_server_actions(&path, renderer).await?;
                } else if path
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s == "ts" || s == "tsx" || s == "js" || s == "jsx")
                    .unwrap_or(false)
                {
                    let Ok(code) = fs::read_to_string(&path).await else {
                        continue;
                    };

                    if has_use_server_directive(&code) {
                        let src_dir = Path::new("src");
                        let relative_path = path.strip_prefix(src_dir).unwrap_or(&path);
                        let action_id = relative_path
                            .to_str()
                            .unwrap_or("unknown")
                            .cow_replace(".ts", "")
                            .cow_replace(".tsx", "")
                            .cow_replace(".js", "")
                            .cow_replace(".jsx", "")
                            .cow_replace('\\', "/")
                            .into_owned();

                        let dist_path =
                            Path::new(DIST_DIR).join("server").join(format!("{action_id}.js"));

                        if fs::try_exists(&dist_path).await.unwrap_or(false) {
                            match fs::read_to_string(&dist_path).await {
                                Ok(dist_code) => {
                                    let canonical_path =
                                        fs::canonicalize(&dist_path).await.unwrap_or(dist_path);
                                    let module_specifier = path_to_file_url(&canonical_path);

                                    let esm_load_result = renderer
                                        .runtime
                                        .add_module_to_loader(&module_specifier, dist_code.clone())
                                        .await;

                                    if esm_load_result.is_ok() {
                                        match renderer.runtime.load_es_module(&action_id).await {
                                            Ok(module_id) => {
                                                if let Err(e) = renderer
                                                    .runtime
                                                    .evaluate_module(module_id)
                                                    .await
                                                {
                                                    tracing::error!(
                                                        "Failed to evaluate server action module {}: {}",
                                                        action_id,
                                                        e
                                                    );
                                                } else {
                                                    let module_specifier_json = serde_json::to_string(&module_specifier)
                                                        .map_err(|e| {
                                                            tracing::error!("Failed to serialize module_specifier for {}: {}", action_id, e);
                                                            RariError::internal(format!("Failed to serialize module_specifier: {e}"))
                                                        })?;
                                                    let action_id_json = serde_json::to_string(
                                                        &action_id,
                                                    )
                                                    .map_err(|e| {
                                                        tracing::error!(
                                                            "Failed to serialize action_id {}: {}",
                                                            action_id,
                                                            e
                                                        );
                                                        RariError::internal(format!(
                                                            "Failed to serialize action_id: {e}"
                                                        ))
                                                    })?;

                                                    let registration_script = format!(
                                                        r#"(async function() {{
                                                            try {{
                                                                const moduleNamespace = await import({module_specifier_json});
                                                                if (!globalThis['~serverFunctions']) {{
                                                                    globalThis['~serverFunctions'] = {{}};
                                                                }}
                                                                if (!globalThis['~serverFunctions'].all) {{
                                                                    globalThis['~serverFunctions'].all = {{}};
                                                                }}
                                                                if (!globalThis['~serverFunctions'].exported) {{
                                                                    globalThis['~serverFunctions'].exported = {{}};
                                                                }}
                                                                for (const [key, value] of Object.entries(moduleNamespace)) {{
                                                                    if (typeof value === 'function') {{
                                                                        const moduleKey = {action_id_json} + ':' + key;
                                                                        globalThis['~serverFunctions'].all[moduleKey] = value;
                                                                        globalThis['~serverFunctions'].exported[moduleKey] = value;
                                                                    }}
                                                                }}
                                                                return {{ success: true }};
                                                            }} catch (error) {{
                                                                console.error("Failed to register server action " + {action_id_json}, error);
                                                                return {{ success: false, error: error.message }};
                                                            }}
                                                        }})()"#
                                                    );

                                                    match renderer
                                                        .runtime
                                                        .execute_script(
                                                            format!(
                                                                "register_action_{}.js",
                                                                action_id.cow_replace('/', "_")
                                                            ),
                                                            registration_script,
                                                        )
                                                        .await
                                                    {
                                                        Ok(result) => {
                                                            if let Some(success) =
                                                                result.get("success").and_then(
                                                                    serde_json::Value::as_bool,
                                                                )
                                                                && !success
                                                            {
                                                                if let Some(error_msg) = result
                                                                    .get("error")
                                                                    .and_then(|v| v.as_str())
                                                                {
                                                                    tracing::error!(
                                                                        "Failed to register server action {}: {}",
                                                                        action_id,
                                                                        error_msg
                                                                    );
                                                                } else {
                                                                    tracing::error!(
                                                                        "Failed to register server action {}: unknown error",
                                                                        action_id
                                                                    );
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            tracing::error!(
                                                                "Failed to register server action {}: {}",
                                                                action_id,
                                                                e
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                tracing::error!(
                                                    "Failed to load server action {} as ESM module: {}",
                                                    action_id,
                                                    e
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
                                                    action_id.cow_replace('/', "_")
                                                ),
                                                wrapped_code,
                                            )
                                            .await
                                        {
                                            Ok(_) => {}
                                            Err(e) => {
                                                tracing::error!(
                                                    "Failed to load server action {}: {}",
                                                    action_id,
                                                    e
                                                );
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "Failed to read built server action {:?}: {}",
                                        dist_path,
                                        e
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
        let server_dir = Path::new(DIST_DIR).join("server");
        if !fs::try_exists(&server_dir).await.unwrap_or(false) {
            return Ok(());
        }

        Self::load_server_components_recursive(&server_dir, &server_dir, renderer).await?;

        Ok(())
    }

    fn load_server_components_recursive<'a>(
        dir: &'a Path,
        base_dir: &'a Path,
        renderer: &'a mut RscRenderer,
    ) -> Pin<Box<dyn Future<Output = Result<(), RariError>> + 'a>> {
        Box::pin(async move {
            let mut entries = fs::read_dir(dir).await.map_err(|e| {
                RariError::io(format!("Failed to read directory {}: {}", dir.display(), e))
            })?;

            while let Some(entry) = entries
                .next_entry()
                .await
                .map_err(|e| RariError::io(format!("Failed to read directory entry: {e}")))?
            {
                let path = entry.path();
                let file_type = entry
                    .file_type()
                    .await
                    .map_err(|e| RariError::io(format!("Failed to read file type: {e}")))?;

                if file_type.is_dir() {
                    Self::load_server_components_recursive(&path, base_dir, renderer).await?;
                } else if path.extension().and_then(|s| s.to_str()) == Some("js") {
                    let file_name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                    if file_name == "proxy" {
                        continue;
                    }

                    let component_code = fs::read_to_string(&path).await.map_err(|e| {
                        RariError::io(format!("Failed to read component file: {e}"))
                    })?;

                    if has_use_server_directive(&component_code) {
                        let relative_path = path.strip_prefix(base_dir).unwrap_or(&path);
                        let relative_str = relative_path
                            .to_str()
                            .unwrap_or("unknown")
                            .cow_replace(".js", "")
                            .cow_replace('\\', "/")
                            .into_owned();

                        let canonical_path =
                            fs::canonicalize(&path).await.unwrap_or_else(|_| path.clone());
                        let module_specifier = path_to_file_url(&canonical_path);

                        let esm_load_result = renderer
                            .runtime
                            .add_module_to_loader(&module_specifier, component_code.clone())
                            .await;

                        if esm_load_result.is_ok() {
                            match renderer.runtime.load_es_module(&relative_str).await {
                                Ok(module_id) => {
                                    if let Err(e) =
                                        renderer.runtime.evaluate_module(module_id).await
                                    {
                                        tracing::error!(
                                            "Failed to evaluate server action module {}: {}",
                                            relative_str,
                                            e
                                        );
                                    } else {
                                        let module_specifier_json = serde_json::to_string(
                                            &module_specifier,
                                        )
                                        .map_err(|e| {
                                            tracing::error!(
                                                "Failed to serialize module_specifier for {}: {}",
                                                relative_str,
                                                e
                                            );
                                            RariError::internal(format!(
                                                "Failed to serialize module_specifier: {e}"
                                            ))
                                        })?;
                                        let relative_str_json =
                                            serde_json::to_string(&relative_str).map_err(|e| {
                                                tracing::error!(
                                                    "Failed to serialize relative_str {}: {}",
                                                    relative_str,
                                                    e
                                                );
                                                RariError::internal(format!(
                                                    "Failed to serialize relative_str: {e}"
                                                ))
                                            })?;

                                        let registration_script = format!(
                                            r#"(async function() {{
                                                try {{
                                                    const moduleNamespace = await import({module_specifier_json});
                                                    if (!globalThis['~serverFunctions']) {{
                                                        globalThis['~serverFunctions'] = {{}};
                                                    }}
                                                    if (!globalThis['~serverFunctions'].all) {{
                                                        globalThis['~serverFunctions'].all = {{}};
                                                    }}
                                                    if (!globalThis['~serverFunctions'].exported) {{
                                                        globalThis['~serverFunctions'].exported = {{}};
                                                    }}
                                                    for (const [key, value] of Object.entries(moduleNamespace)) {{
                                                        if (typeof value === 'function') {{
                                                            const moduleKey = {relative_str_json} + ':' + key;
                                                            globalThis['~serverFunctions'].all[moduleKey] = value;
                                                            globalThis['~serverFunctions'].exported[moduleKey] = value;
                                                        }}
                                                    }}
                                                    return {{ success: true }};
                                                }} catch (error) {{
                                                    console.error("Failed to register server action " + {relative_str_json}, error);
                                                    return {{ success: false, error: error.message }};
                                                }}
                                            }})()"#
                                        );

                                        match renderer
                                            .runtime
                                            .execute_script(
                                                format!(
                                                    "register_{}.js",
                                                    relative_str.cow_replace('/', "_")
                                                ),
                                                registration_script,
                                            )
                                            .await
                                        {
                                            Ok(result) => {
                                                if let Some(success) = result
                                                    .get("success")
                                                    .and_then(serde_json::Value::as_bool)
                                                    && !success
                                                {
                                                    if let Some(error_msg) =
                                                        result.get("error").and_then(|v| v.as_str())
                                                    {
                                                        tracing::error!(
                                                            "Failed to register server functions from {}: {}",
                                                            relative_str,
                                                            error_msg
                                                        );
                                                    } else {
                                                        tracing::error!(
                                                            "Failed to register server functions from {}: unknown error",
                                                            relative_str
                                                        );
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                tracing::error!(
                                                    "Failed to register server functions from {}: {}",
                                                    relative_str,
                                                    e
                                                );
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "Failed to load server action {} as ESM module: {}",
                                        relative_str,
                                        e
                                    );
                                }
                            }
                        } else {
                            let wrapped_code =
                                wrap_server_action_module(&component_code, &relative_str);
                            match renderer
                                .runtime
                                .execute_script(
                                    format!("load_{}.js", relative_str.cow_replace('/', "_")),
                                    wrapped_code,
                                )
                                .await
                            {
                                Ok(_) => {}
                                Err(e) => {
                                    tracing::error!(
                                        "Failed to load server actions from {}: {}",
                                        relative_str,
                                        e
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
                        .cow_replace(".js", "")
                        .cow_replace('\\', "/")
                        .into_owned();

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

                    let canonical_path =
                        fs::canonicalize(&path).await.unwrap_or_else(|_| path.clone());
                    let module_specifier = path_to_file_url(&canonical_path);

                    let esm_load_result = renderer
                        .runtime
                        .add_module_to_loader(&module_specifier, component_code.clone())
                        .await;

                    if esm_load_result.is_ok() {
                        match renderer.runtime.load_es_module(&component_id).await {
                            Ok(module_id) => {
                                if let Err(e) = renderer.runtime.evaluate_module(module_id).await {
                                    tracing::error!(
                                        "Failed to evaluate ESM module {} (id: {}): {}",
                                        component_id,
                                        module_id,
                                        e
                                    );
                                } else {
                                    let skip_global_binding = component_id.starts_with("lib/");
                                    let module_specifier_json =
                                        serde_json::to_string(&module_specifier).map_err(|e| {
                                            tracing::error!(
                                                "Failed to serialize module_specifier for {}: {}",
                                                component_id,
                                                e
                                            );
                                            RariError::internal(format!(
                                                "Failed to serialize module_specifier: {e}"
                                            ))
                                        })?;
                                    let component_id_json = serde_json::to_string(&component_id)
                                        .map_err(|e| {
                                            tracing::error!(
                                                "Failed to serialize component_id {}: {}",
                                                component_id,
                                                e
                                            );
                                            RariError::internal(format!(
                                                "Failed to serialize component_id: {e}"
                                            ))
                                        })?;
                                    let registration_script = format!(
                                        r"globalThis['~rari'].componentLoader.registerComponent({module_specifier_json}, {component_id_json}, {skip_global_binding})"
                                    );

                                    match renderer
                                        .runtime
                                        .execute_script(
                                            format!(
                                                "register_{}.js",
                                                component_id.cow_replace('/', "_")
                                            ),
                                            registration_script,
                                        )
                                        .await
                                    {
                                        Ok(result) => {
                                            let registration_succeeded = result
                                                .get("success")
                                                .and_then(serde_json::Value::as_bool)
                                                .unwrap_or(false);

                                            if !registration_succeeded {
                                                tracing::error!(
                                                    "Failed to register component {} to globalThis: {:?}",
                                                    component_id,
                                                    result.get("error")
                                                );
                                            }

                                            if registration_succeeded && is_client_component {
                                                let component_id_json = serde_json::to_string(
                                                    &component_id,
                                                )
                                                .unwrap_or_else(|e| {
                                                    tracing::error!(
                                                        "Failed to serialize component_id {}: {}",
                                                        component_id,
                                                        e
                                                    );
                                                    "\"\"".to_string()
                                                });
                                                let mark_client_script = if skip_global_binding {
                                                    format!(
                                                        r"(function() {{
                                                            const module = globalThis['~rsc']?.modules?.[{component_id_json}];
                                                            if (module) {{
                                                                const comp = module.default || Object.values(module).find(v => typeof v === 'function');
                                                                if (comp && typeof comp === 'function') {{
                                                                    comp['~isClientComponent'] = true;
                                                                    comp['~clientComponentId'] = {component_id_json};
                                                                }}
                                                            }}
                                                            return {{ componentId: {component_id_json}, isClient: true }};
                                                        }})()"
                                                    )
                                                } else {
                                                    format!(
                                                        r"(function() {{
                                                            const comp = globalThis[{component_id_json}];
                                                            if (comp && typeof comp === 'function') {{
                                                                comp['~isClientComponent'] = true;
                                                                comp['~clientComponentId'] = {component_id_json};
                                                            }}
                                                            return {{ componentId: {component_id_json}, isClient: true }};
                                                        }})()"
                                                    )
                                                };

                                                if let Err(e) = renderer
                                                    .runtime
                                                    .execute_script(
                                                        format!(
                                                            "mark_client_{}.js",
                                                            component_id.cow_replace('/', "_")
                                                        ),
                                                        mark_client_script,
                                                    )
                                                    .await
                                                {
                                                    tracing::error!(
                                                        "Failed to mark component {} as client: {}",
                                                        component_id,
                                                        e
                                                    );
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                "Failed to register component {}: {}",
                                                component_id,
                                                e
                                            );
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to load component {} as ESM module: {}",
                                    component_id,
                                    e
                                );
                            }
                        }
                    } else {
                        match renderer
                            .runtime
                            .execute_script(
                                format!("load_{}.js", component_id.cow_replace('/', "_")),
                                transformed_module_code,
                            )
                            .await
                        {
                            Ok(_) => {
                                if is_client_component {
                                    let skip_global_binding = component_id.starts_with("lib/");
                                    let component_id_json = serde_json::to_string(&component_id)
                                        .unwrap_or_else(|e| {
                                            tracing::error!(
                                                "Failed to serialize component_id {}: {}",
                                                component_id,
                                                e
                                            );
                                            "\"\"".to_string()
                                        });

                                    let mark_client_script = if skip_global_binding {
                                        format!(
                                            r"(function() {{
                                                const module = globalThis['~rsc']?.modules?.[{component_id_json}];
                                                if (module) {{
                                                    const comp = module.default || Object.values(module).find(v => typeof v === 'function');
                                                    if (comp && typeof comp === 'function') {{
                                                        comp['~isClientComponent'] = true;
                                                        comp['~clientComponentId'] = {component_id_json};
                                                    }}
                                                }}
                                                return {{ componentId: {component_id_json}, isClient: true }};
                                            }})()"
                                        )
                                    } else {
                                        format!(
                                            r"(function() {{
                                                const comp = globalThis[{component_id_json}];
                                                if (comp && typeof comp === 'function') {{
                                                    comp['~isClientComponent'] = true;
                                                    comp['~clientComponentId'] = {component_id_json};
                                                }}
                                                return {{ componentId: {component_id_json}, isClient: true }};
                                            }})()"
                                        )
                                    };

                                    if let Err(e) = renderer
                                        .runtime
                                        .execute_script(
                                            format!(
                                                "mark_client_{}.js",
                                                component_id.cow_replace('/', "_")
                                            ),
                                            mark_client_script,
                                        )
                                        .await
                                    {
                                        tracing::error!(
                                            "Failed to mark component {} as client: {}",
                                            component_id,
                                            e
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to execute component {}: {}",
                                    component_id,
                                    e
                                );
                            }
                        }
                    }
                }
            }

            Ok(())
        })
    }

    async fn read_manifest(manifest_path: &Path) -> Result<serde_json::Value, RariError> {
        let manifest_content = fs::read_to_string(manifest_path)
            .await
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

        let component_file = Path::new(DIST_DIR).join(bundle_path);

        if !fs::try_exists(&component_file).await.unwrap_or(false) {
            return Err(RariError::not_found(format!(
                "Component file not found: {}",
                component_file.display()
            )));
        }

        let component_code = fs::read_to_string(&component_file)
            .await
            .map_err(|e| RariError::io(format!("Failed to read component file: {e}")))?;

        renderer
            .register_component(component_id, &component_code)
            .await
            .map_err(|e| RariError::internal(format!("Failed to register component: {e}")))
    }

    pub async fn load_ssr_client_components(
        runtime: &Arc<JsExecutionRuntime>,
    ) -> Result<(), RariError> {
        let manifest_path = Path::new(DIST_DIR).join("ssr").join("manifest.json");
        if !fs::try_exists(&manifest_path).await.unwrap_or(false) {
            return Ok(());
        }

        let init_script = r"
            if (!globalThis['~rari']) {
                globalThis['~rari'] = {};
            }
            if (!globalThis['~rari'].ssrModules) {
                globalThis['~rari'].ssrModules = {};
            }
        ";
        runtime
            .execute_script("init_ssr_modules".to_string(), init_script.to_string())
            .await
            .map_err(|e| RariError::internal(format!("Failed to initialize ssrModules: {e}")))?;

        let manifest_content = fs::read_to_string(&manifest_path)
            .await
            .map_err(|e| RariError::io(format!("Failed to read SSR manifest: {e}")))?;

        let manifest: serde_json::Value = serde_json::from_str(&manifest_content)
            .map_err(|e| RariError::internal(format!("Failed to parse SSR manifest: {e}")))?;

        let Some(entries) = manifest.as_object() else {
            return Ok(());
        };

        let mut to_import: Vec<(String, String)> = Vec::new();
        for (module_path, info) in entries {
            let bundle_path = info.get("bundlePath").and_then(|v| v.as_str()).unwrap_or_default();

            let component_file = Path::new(DIST_DIR).join(bundle_path);
            if !fs::try_exists(&component_file).await.unwrap_or(false) {
                continue;
            }

            let Ok(code) = fs::read_to_string(&component_file).await else {
                continue;
            };

            let module_specifier = format!("file:///{}", bundle_path.cow_replace('\\', "/"));
            if let Err(e) = runtime.add_module_to_loader(&module_specifier, code).await {
                tracing::error!("Failed to add SSR module {}: {}", module_path, e);
                continue;
            }

            to_import.push((module_path.clone(), module_specifier));
        }

        for (module_path, module_specifier) in &to_import {
            let module_path = module_path.as_str();
            let module_path_json = serde_json::to_string(module_path).unwrap_or_default();

            let exports = entries
                .get(module_path)
                .and_then(|v| v.get("exports"))
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter().filter_map(|v| v.as_str()).map(String::from).collect::<Vec<_>>()
                })
                .unwrap_or_default();

            let register_script = format!(
                r"(async function() {{
                    try {{
                        const mod = await import({specifier});
                        if (!globalThis['~rari'] || !globalThis['~rari'].ssrModules) {{
                            console.error('[rari] SSR: globalThis[~rari].ssrModules not initialized');
                            return false;
                        }}
                        globalThis['~rari'].ssrModules[{path}] = mod;

                        const exports = {exports_json};
                        for (const exportName of exports) {{
                            const fullId = {path} + '#' + exportName;
                            globalThis['~rari'].ssrModules[fullId] = mod;
                        }}

                        return true;
                    }} catch (e) {{
                        console.error('[rari] SSR: Failed to import module ' + {path} + ':', e?.message || e);
                        return false;
                    }}
                }})()",
                specifier = serde_json::to_string(&module_specifier).unwrap_or_default(),
                path = module_path_json,
                exports_json = serde_json::to_string(&exports).unwrap_or_else(|_| "[]".to_string()),
            );

            if let Err(e) = runtime
                .execute_script(
                    format!("ssr_load_{}.js", module_path.cow_replace('/', "_")),
                    register_script,
                )
                .await
            {
                tracing::error!("Failed to load SSR module {}: {}", module_path, e);
            }
        }

        Ok(())
    }

    pub async fn load_client_reference_manifest(
        runtime: &Arc<JsExecutionRuntime>,
    ) -> Result<(), RariError> {
        let manifest_path =
            Path::new(DIST_DIR).join("server").join("client-reference-manifest.json");
        if !fs::try_exists(&manifest_path).await.unwrap_or(false) {
            return Ok(());
        }

        let manifest_content = fs::read_to_string(&manifest_path)
            .await
            .map_err(|e| RariError::io(format!("Failed to read client reference manifest: {e}")))?;

        let manifest: Value = serde_json::from_str(&manifest_content).map_err(|e| {
            RariError::internal(format!("Failed to parse client reference manifest: {e}"))
        })?;

        let manifest_json = serde_json::to_string(&manifest).unwrap_or_else(|_| "{}".to_string());

        let init_script = format!(
            r"(function() {{
                if (!globalThis['~rari']) {{
                    globalThis['~rari'] = {{}};
                }}
                globalThis['~rari'].clientReferenceManifest = {manifest_json};
            }})()"
        );

        runtime
            .execute_script("init_client_reference_manifest".to_string(), init_script)
            .await
            .map_err(|e| {
                RariError::internal(format!("Failed to initialize client reference manifest: {e}"))
            })?;

        Ok(())
    }
}
