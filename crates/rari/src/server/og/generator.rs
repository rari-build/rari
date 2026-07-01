#![expect(
    clippy::unnecessary_wraps,
    reason = "Generator methods return Result for API consistency with error-handling variants"
)]
#![allow(clippy::explicit_counter_loop)]

use std::{path::PathBuf, string::ToString, sync::Arc, vec::Vec};

use cow_utils::CowUtils;
use rari_utils::path_to_file_url;
use rustc_hash::FxHashMap;
use serde_json::{Map, Value};
use tokio::{fs, sync::RwLock};

use super::{
    OgImageError,
    cache::OgImageCache,
    layout::LayoutEngine,
    rendering::ImageRenderer,
    types::{JsxChild, JsxElement, OgImageEntry},
};
use crate::{
    runtime::JsExecutionRuntime,
    server::{cache::handler::CacheError, routing::types::ParamValue},
};

pub struct OgImageGenerator {
    runtime: Arc<JsExecutionRuntime>,
    cache: OgImageCache,
    manifest: Arc<RwLock<FxHashMap<String, OgImageEntry>>>,
    project_path: PathBuf,
    server_manifest: Arc<RwLock<FxHashMap<String, String>>>,
}

impl OgImageGenerator {
    pub fn new(runtime: Arc<JsExecutionRuntime>, project_path: PathBuf) -> Self {
        Self {
            runtime,
            cache: OgImageCache::new(20, &project_path),
            manifest: Arc::new(RwLock::new(FxHashMap::default())),
            project_path,
            server_manifest: Arc::new(RwLock::new(FxHashMap::default())),
        }
    }

    pub fn with_capacity(
        runtime: Arc<JsExecutionRuntime>,
        project_path: PathBuf,
        cache_capacity: usize,
    ) -> Self {
        Self {
            runtime,
            cache: OgImageCache::new(cache_capacity, &project_path),
            manifest: Arc::new(RwLock::new(FxHashMap::default())),
            project_path,
            server_manifest: Arc::new(RwLock::new(FxHashMap::default())),
        }
    }

    pub fn with_capacity_and_cache(
        runtime: Arc<JsExecutionRuntime>,
        project_path: PathBuf,
        cache: OgImageCache,
    ) -> Self {
        Self {
            runtime,
            cache,
            manifest: Arc::new(RwLock::new(FxHashMap::default())),
            project_path,
            server_manifest: Arc::new(RwLock::new(FxHashMap::default())),
        }
    }

    pub async fn load_manifest(&self, manifest_path: &str) -> Result<(), OgImageError> {
        let content = fs::read_to_string(manifest_path)
            .await
            .map_err(|e| OgImageError::InternalError(format!("Failed to read manifest: {e}")))?;

        let manifest_data: Value = serde_json::from_str(&content)
            .map_err(|e| OgImageError::InternalError(format!("Failed to parse manifest: {e}")))?;

        let mut manifest = self.manifest.write().await;
        manifest.clear();

        if let Some(og_images) = manifest_data.get("ogImages").and_then(|v| v.as_array()) {
            for entry in og_images {
                if let Ok(og_entry) = serde_json::from_value::<OgImageEntry>(entry.clone()) {
                    if let Some(existing) = manifest.get(&og_entry.path) {
                        tracing::warn!(
                            "OG image path collision: '{}' is already used by '{}', overwriting with '{}'",
                            og_entry.path,
                            existing.file_path,
                            og_entry.file_path
                        );
                    }
                    manifest.insert(og_entry.path.clone(), og_entry);
                }
            }
        }

        let server_manifest_path =
            manifest_path.cow_replace("routes.json", "manifest.json").into_owned();
        if let Ok(server_content) = fs::read_to_string(&server_manifest_path).await
            && let Ok(server_data) = serde_json::from_str::<Value>(&server_content)
            && let Some(components) = server_data.get("components").and_then(|v| v.as_object())
        {
            let mut server_manifest = self.server_manifest.write().await;
            for (id, component) in components {
                if let Some(bundle_path) = component.get("bundlePath").and_then(|v| v.as_str()) {
                    server_manifest.insert(id.clone(), bundle_path.to_string());
                }
            }
        }

        Ok(())
    }

    pub async fn register_component(&self, path: String, entry: OgImageEntry) {
        let mut manifest = self.manifest.write().await;
        manifest.insert(path, entry);
    }

    pub async fn find_og_image_for_route(&self, route_path: &str) -> Option<OgImageEntry> {
        let manifest = self.manifest.read().await;
        self.find_matching_entry(&manifest, route_path).map(|(entry, _)| entry.clone())
    }

    pub async fn generate(&self, route_path: &str) -> Result<(Vec<u8>, bool), OgImageError> {
        const MAX_OG_WIDTH: u32 = 2400;
        const MAX_OG_HEIGHT: u32 = 1260;

        if let Some(cached) = self.cache.get(route_path).await {
            return Ok((cached, true));
        }

        let manifest = self.manifest.read().await;

        let (entry, params) = self
            .find_matching_entry(&manifest, route_path)
            .ok_or_else(|| OgImageError::ComponentNotFound(route_path.to_string()))?;

        let entry = entry.clone();
        drop(manifest);

        let jsx_element = self.execute_og_component(&entry, route_path, &params).await?;

        let width = entry.width.unwrap_or(1200).min(MAX_OG_WIDTH);
        let height = entry.height.unwrap_or(630).min(MAX_OG_HEIGHT);

        let (computed_layout, font_context) = {
            let mut layout_engine = LayoutEngine::new();
            let font_context = layout_engine.get_font_context();
            let computed_layout =
                layout_engine
                    .layout(&jsx_element, width as f32, height as f32)
                    .map_err(|e| OgImageError::GenerationError(format!("Layout failed: {e}")))?;
            (computed_layout, font_context)
        };

        let mut renderer = ImageRenderer::new(width, height, font_context);
        let image = renderer
            .render(&computed_layout)
            .map_err(|e| OgImageError::GenerationError(format!("Image generation failed: {e}")))?;

        let webp_data = Self::encode_webp(&image)
            .map_err(|e| OgImageError::GenerationError(format!("Failed to encode WebP: {e}")))?;

        if let Err(e) = self.cache.insert(route_path.to_string(), webp_data.clone()).await {
            tracing::warn!(error = %e, route_path, "OG cache insert failed");
        }

        Ok((webp_data, false))
    }

    fn find_matching_entry<'a>(
        &self,
        manifest: &'a FxHashMap<String, OgImageEntry>,
        route_path: &str,
    ) -> Option<(&'a OgImageEntry, FxHashMap<String, ParamValue>)> {
        if let Some(entry) = manifest.get(route_path) {
            return Some((entry, FxHashMap::default()));
        }

        if let Some(entry) = manifest.values().find(|entry| {
            entry
                .additional_paths
                .as_deref()
                .is_some_and(|paths| paths.iter().any(|path| path.as_str() == route_path))
        }) {
            return Some((entry, FxHashMap::default()));
        }

        let path_segments: Vec<&str> = route_path.split('/').filter(|s| !s.is_empty()).collect();

        for (pattern, entry) in manifest {
            let pattern_segments: Vec<&str> =
                pattern.split('/').filter(|s| !s.is_empty()).collect();

            let has_catch_all =
                pattern_segments.iter().any(|seg| seg.starts_with("[...") && seg.ends_with(']'));

            if has_catch_all {
                let mut params = FxHashMap::default();
                let mut matches = true;
                let mut path_idx = 0;

                for pattern_seg in &pattern_segments {
                    if pattern_seg.starts_with("[...") && pattern_seg.ends_with(']') {
                        let param_name = &pattern_seg[4..pattern_seg.len() - 1];
                        let remaining: Vec<String> =
                            path_segments[path_idx..].iter().map(ToString::to_string).collect();
                        params.insert(param_name.to_string(), ParamValue::Multiple(remaining));
                        break;
                    } else if path_idx >= path_segments.len()
                        || pattern_seg != &path_segments[path_idx]
                    {
                        matches = false;
                        break;
                    }
                    path_idx += 1;
                }

                if matches {
                    return Some((entry, params));
                }
            } else {
                if pattern_segments.len() != path_segments.len() {
                    continue;
                }

                let mut params = FxHashMap::default();
                let mut matches = true;

                for (pattern_seg, path_seg) in pattern_segments.iter().zip(path_segments.iter()) {
                    if pattern_seg.starts_with('[') && pattern_seg.ends_with(']') {
                        let param_name = &pattern_seg[1..pattern_seg.len() - 1];
                        params.insert(
                            param_name.to_string(),
                            ParamValue::Single(path_seg.to_string()),
                        );
                    } else if pattern_seg != path_seg {
                        matches = false;
                        break;
                    }
                }

                if matches {
                    return Some((entry, params));
                }
            }
        }

        None
    }

    async fn execute_og_component(
        &self,
        entry: &OgImageEntry,
        route_path: &str,
        params: &FxHashMap<String, ParamValue>,
    ) -> Result<JsxElement, OgImageError> {
        let component_id = {
            let path = entry.file_path.as_str();
            let path = path.cow_replace(".tsx", "");
            let path = path.cow_replace(".ts", "");
            let path = path.cow_replace(".jsx", "");
            let path = path.cow_replace(".js", "");
            let path =
                path.chars()
                    .map(|c| {
                        if c.is_alphanumeric() || c == '/' || c == '-' || c == '_' {
                            c
                        } else {
                            '_'
                        }
                    })
                    .collect::<String>();
            format!("app/{path}")
        };

        let server_manifest = self.server_manifest.read().await;
        let bundle_path = server_manifest
            .get(&component_id)
            .ok_or_else(|| {
                OgImageError::ExecutionError(format!(
                    "Component '{}' not found in server manifest. Available: {:?}",
                    component_id,
                    server_manifest.keys().filter(|k| k.contains("opengraph")).collect::<Vec<_>>()
                ))
            })?
            .clone();
        drop(server_manifest);

        let module_path = self.project_path.join("dist").join(&bundle_path);
        let module_url = path_to_file_url(&module_path);

        let params_json = serde_json::to_string(params)
            .map_err(|e| OgImageError::InternalError(format!("Failed to serialize params: {e}")))?;

        let wrapper_script = format!(
            r#"
(async function() {{
    const module = await import("{module_url}");

    const ImageComponent = module.default;

    if (!ImageComponent) {{
        throw new Error('No default export found in OG image component');
    }}

    const params = {params_json};

    let result;
    try {{
        result = await ImageComponent({{ params }});
    }} catch (e) {{
        result = ImageComponent({{ params }});
    }}

    if (result && result.toJSON) {{
        const json = result.toJSON();
        if (json.element) {{
            return json.element;
        }}
    }}

    throw new Error('Component did not return an ImageResponse');
}})();
"#
        );

        let result = self
            .runtime
            .execute_script(
                format!("og_image_{}", route_path.cow_replace("/", "_")),
                wrapper_script,
            )
            .await
            .map_err(|e| {
                OgImageError::ExecutionError(format!("Failed to execute component: {e}"))
            })?;

        let jsx_element = Self::parse_serialized_jsx(&result)?;

        Ok(jsx_element)
    }

    fn parse_serialized_jsx(value: &Value) -> Result<JsxElement, OgImageError> {
        if value.is_null() {
            return Err(OgImageError::ExecutionError("Component returned null".to_string()));
        }

        let obj = value.as_object().ok_or_else(|| {
            OgImageError::ExecutionError("Expected object from component".to_string())
        })?;

        let element_type =
            obj.get("elementType").and_then(|v| v.as_str()).unwrap_or("div").to_string();

        let props = obj.get("props").cloned().unwrap_or(Value::Object(Map::default()));

        let children_array =
            obj.get("children").and_then(|v| v.as_array()).map(Vec::as_slice).unwrap_or(&[]);

        let mut children = Vec::new();
        for child_value in children_array {
            if let Some(child_type) = child_value.get("type").and_then(|v| v.as_str()) {
                match child_type {
                    "text" => {
                        if let Some(text) = child_value.get("value").and_then(|v| v.as_str()) {
                            children.push(JsxChild::Text(text.to_string()));
                        }
                    }
                    "element" => {
                        let child_element = Self::parse_serialized_jsx(child_value)?;
                        children.push(JsxChild::Element(Box::new(child_element)));
                    }
                    _ => {}
                }
            }
        }

        Ok(JsxElement { element_type, props, children })
    }

    fn encode_webp(image: &image::RgbaImage) -> Result<Vec<u8>, String> {
        use webp::Encoder;

        let encoder = Encoder::from_rgba(image.as_raw(), image.width(), image.height());

        let webp = encoder.encode(80.0);

        Ok(webp.to_vec())
    }

    #[cfg(test)]
    #[expect(clippy::expect_used)]
    pub async fn clear_cache(&self) {
        self.cache.clear().await.expect("clear");
    }

    pub async fn invalidate(&self, route_path: &str) -> Result<(), CacheError> {
        self.cache.remove(route_path).await.map(|_| ())
    }
}

#[cfg(test)]
#[allow(
    clippy::allow_attributes,
    clippy::unreadable_literal,
    clippy::needless_raw_string_hashes,
    clippy::panic,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::float_cmp,
    clippy::bool_assert_comparison,
    clippy::redundant_clone,
    clippy::redundant_closure_for_method_calls,
    clippy::single_char_pattern,
    clippy::approx_constant,
    clippy::uninlined_format_args,
    clippy::module_inception,
    clippy::return_self_not_must_use,
    clippy::disallowed_methods,
    clippy::clone_on_ref_ptr,
    clippy::get_unwrap
)]
mod tests {
    use std::env;

    use super::*;

    #[tokio::test]
    async fn test_find_og_image_for_static_route() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let test_dir = env::temp_dir().join("rari-test-og-static");
        let generator = OgImageGenerator::new(runtime, test_dir);

        let entry = OgImageEntry {
            path: "/blog".to_string(),
            file_path: "blog/opengraph-image.tsx".to_string(),
            width: Some(1200),
            height: Some(630),
            content_type: Some("image/png".to_string()),
            additional_paths: None,
        };

        generator.register_component("/blog".to_string(), entry.clone()).await;

        let found = generator.find_og_image_for_route("/blog").await;
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.path, "/blog");
        assert_eq!(found.width, Some(1200));
        assert_eq!(found.height, Some(630));
    }

    #[tokio::test]
    async fn test_find_og_image_for_dynamic_route() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let test_dir = env::temp_dir().join("rari-test-og-dynamic");
        let generator = OgImageGenerator::new(runtime, test_dir);

        let entry = OgImageEntry {
            path: "/blog/[slug]".to_string(),
            file_path: "blog/[slug]/opengraph-image.tsx".to_string(),
            width: Some(1200),
            height: Some(630),
            content_type: Some("image/png".to_string()),
            additional_paths: None,
        };

        generator.register_component("/blog/[slug]".to_string(), entry.clone()).await;

        let found = generator.find_og_image_for_route("/blog/hello-world").await;
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.path, "/blog/[slug]");
        assert_eq!(found.width, Some(1200));
        assert_eq!(found.height, Some(630));
    }

    #[tokio::test]
    async fn test_find_og_image_not_found() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let test_dir = env::temp_dir().join("rari-test-og-not-found");
        let generator = OgImageGenerator::new(runtime, test_dir);

        let found = generator.find_og_image_for_route("/nonexistent").await;
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_find_og_image_prefers_exact_match() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let test_dir = env::temp_dir().join("rari-test-og-exact-match");
        let generator = OgImageGenerator::new(runtime, test_dir);

        let dynamic_entry = OgImageEntry {
            path: "/blog/[slug]".to_string(),
            file_path: "blog/[slug]/opengraph-image.tsx".to_string(),
            width: Some(1200),
            height: Some(630),
            content_type: Some("image/png".to_string()),
            additional_paths: None,
        };

        let static_entry = OgImageEntry {
            path: "/blog/featured".to_string(),
            file_path: "blog/featured/opengraph-image.tsx".to_string(),
            width: Some(1600),
            height: Some(900),
            content_type: Some("image/png".to_string()),
            additional_paths: None,
        };

        generator.register_component("/blog/[slug]".to_string(), dynamic_entry).await;
        generator.register_component("/blog/featured".to_string(), static_entry).await;

        let found = generator.find_og_image_for_route("/blog/featured").await;
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.path, "/blog/featured");
        assert_eq!(found.width, Some(1600));
    }

    #[tokio::test]
    async fn test_find_og_image_with_additional_paths() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let test_dir = env::temp_dir().join("rari-test-og-additional-paths");
        let generator = OgImageGenerator::new(runtime, test_dir);

        let entry = OgImageEntry {
            path: "/about".to_string(),
            file_path: "(marketing)/opengraph-image.tsx".to_string(),
            width: Some(1200),
            height: Some(630),
            content_type: Some("image/png".to_string()),
            additional_paths: Some(vec!["/pricing".to_string()]),
        };

        generator.register_component("/about".to_string(), entry).await;

        let found = generator.find_og_image_for_route("/pricing").await;
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.path, "/about");
        assert_eq!(found.file_path, "(marketing)/opengraph-image.tsx");
    }

    #[test]
    fn test_og_image_entry_deserializes_additional_paths() {
        let entry: OgImageEntry = serde_json::from_value(serde_json::json!({
            "path": "/about",
            "filePath": "(marketing)/opengraph-image.tsx",
            "width": 1200,
            "height": 630,
            "contentType": "image/png",
            "additionalPaths": ["/pricing"]
        }))
        .unwrap();

        assert_eq!(entry.additional_paths, Some(vec!["/pricing".to_string()]));
    }

    #[tokio::test]
    async fn test_load_manifest_warns_on_path_collision_and_overwrites() {
        let manifest = serde_json::json!({
            "ogImages": [
                {
                    "path": "/",
                    "filePath": "(marketing)/opengraph-image.tsx",
                    "width": 1200,
                    "height": 630,
                    "contentType": "image/png"
                },
                {
                    "path": "/",
                    "filePath": "(auth)/opengraph-image.tsx",
                    "width": 1600,
                    "height": 900,
                    "contentType": "image/png"
                }
            ]
        });

        let dir = env::temp_dir().join("rari-test-manifest-collision");
        fs::create_dir_all(&dir).await.unwrap();
        let manifest_path = dir.join("routes.json");
        fs::write(&manifest_path, serde_json::to_string(&manifest).unwrap()).await.unwrap();

        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let test_dir = env::temp_dir().join("rari-test-og-load-manifest");
        let generator = OgImageGenerator::new(runtime, test_dir);

        let result = generator.load_manifest(manifest_path.to_str().unwrap()).await;
        assert!(result.is_ok(), "load_manifest should succeed: {:?}", result);

        let found = generator.find_og_image_for_route("/").await;
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.file_path, "(auth)/opengraph-image.tsx");
        assert_eq!(found.width, Some(1600));
    }
}
