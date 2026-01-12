use super::OgImageError;
use super::cache::OgImageCache;
use super::layout::LayoutEngine;
use super::rendering::ImageRenderer;
use super::types::{JsxChild, JsxElement, OgImageEntry};
use crate::runtime::JsExecutionRuntime;
use cow_utils::CowUtils;
use rustc_hash::FxHashMap;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

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

    pub async fn load_manifest(&self, manifest_path: &str) -> Result<(), OgImageError> {
        let content = tokio::fs::read_to_string(manifest_path)
            .await
            .map_err(|e| OgImageError::InternalError(format!("Failed to read manifest: {}", e)))?;

        let manifest_data: Value = serde_json::from_str(&content)
            .map_err(|e| OgImageError::InternalError(format!("Failed to parse manifest: {}", e)))?;

        let mut manifest = self.manifest.write().await;
        manifest.clear();

        if let Some(og_images) = manifest_data.get("ogImages").and_then(|v| v.as_array()) {
            for entry in og_images {
                if let Ok(og_entry) = serde_json::from_value::<OgImageEntry>(entry.clone()) {
                    manifest.insert(og_entry.path.clone(), og_entry);
                }
            }
        }

        let server_manifest_path =
            manifest_path.cow_replace("routes.json", "manifest.json").into_owned();
        if let Ok(server_content) = tokio::fs::read_to_string(&server_manifest_path).await
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

        if let Some(entry) = manifest.get(route_path) {
            return Some(entry.clone());
        }

        let path_segments: Vec<&str> = route_path.split('/').filter(|s| !s.is_empty()).collect();

        for (pattern, entry) in manifest.iter() {
            let pattern_segments: Vec<&str> =
                pattern.split('/').filter(|s| !s.is_empty()).collect();

            if pattern_segments.len() != path_segments.len() {
                continue;
            }

            let mut matches = true;

            for (pattern_seg, path_seg) in pattern_segments.iter().zip(path_segments.iter()) {
                if pattern_seg.starts_with('[') && pattern_seg.ends_with(']') {
                    continue;
                } else if pattern_seg != path_seg {
                    matches = false;
                    break;
                }
            }

            if matches {
                return Some(entry.clone());
            }
        }

        None
    }

    pub async fn generate(&self, route_path: &str) -> Result<(Vec<u8>, bool), OgImageError> {
        const MAX_OG_WIDTH: u32 = 2400;
        const MAX_OG_HEIGHT: u32 = 1260;

        if let Some(cached) = self.cache.get(route_path) {
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
                    .map_err(|e| OgImageError::GenerationError(format!("Layout failed: {}", e)))?;
            (computed_layout, font_context)
        };

        let mut renderer = ImageRenderer::new(width, height, font_context);
        let image = renderer.render(&computed_layout).map_err(|e| {
            OgImageError::GenerationError(format!("Image generation failed: {}", e))
        })?;

        let webp_data = Self::encode_webp(&image)
            .map_err(|e| OgImageError::GenerationError(format!("Failed to encode WebP: {}", e)))?;

        self.cache.insert(route_path.to_string(), webp_data.clone());

        Ok((webp_data, false))
    }

    fn find_matching_entry<'a>(
        &self,
        manifest: &'a FxHashMap<String, OgImageEntry>,
        route_path: &str,
    ) -> Option<(&'a OgImageEntry, FxHashMap<String, String>)> {
        if let Some(entry) = manifest.get(route_path) {
            return Some((entry, FxHashMap::default()));
        }

        let path_segments: Vec<&str> = route_path.split('/').filter(|s| !s.is_empty()).collect();

        for (pattern, entry) in manifest {
            let pattern_segments: Vec<&str> =
                pattern.split('/').filter(|s| !s.is_empty()).collect();

            if pattern_segments.len() != path_segments.len() {
                continue;
            }

            let mut params = FxHashMap::default();
            let mut matches = true;

            for (pattern_seg, path_seg) in pattern_segments.iter().zip(path_segments.iter()) {
                if pattern_seg.starts_with('[') && pattern_seg.ends_with(']') {
                    let param_name = &pattern_seg[1..pattern_seg.len() - 1];
                    params.insert(param_name.to_string(), path_seg.to_string());
                } else if pattern_seg != path_seg {
                    matches = false;
                    break;
                }
            }

            if matches {
                return Some((entry, params));
            }
        }

        None
    }

    async fn execute_og_component(
        &self,
        entry: &OgImageEntry,
        route_path: &str,
        params: &FxHashMap<String, String>,
    ) -> Result<JsxElement, OgImageError> {
        let component_id = {
            let path = entry.file_path.as_str();
            let path = path.cow_replace(".tsx", "");
            let path = path.cow_replace(".ts", "");
            let path = path.cow_replace(".jsx", "");
            let path = path.cow_replace(".js", "");
            let path = path.cow_replace("[", "_");
            let path = path.cow_replace("]", "_");
            format!("app/{}", path)
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

        let module_url = format!("file://{}/dist/{}", self.project_path.display(), bundle_path);

        let params_json = serde_json::to_string(params).map_err(|e| {
            OgImageError::InternalError(format!("Failed to serialize params: {}", e))
        })?;

        let wrapper_script = format!(
            r#"
(async function() {{
    const module = await import("{}");

    const ImageComponent = module.default;

    if (!ImageComponent) {{
        throw new Error('No default export found in OG image component');
    }}

    const params = {};

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
"#,
            module_url, params_json
        );

        let result = self
            .runtime
            .execute_script(
                format!("og_image_{}", route_path.cow_replace("/", "_")),
                wrapper_script,
            )
            .await
            .map_err(|e| {
                OgImageError::ExecutionError(format!("Failed to execute component: {}", e))
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

        let props = obj.get("props").cloned().unwrap_or(Value::Object(Default::default()));

        let children_array =
            obj.get("children").and_then(|v| v.as_array()).map(|v| v.as_slice()).unwrap_or(&[]);

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

    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    pub fn invalidate(&self, route_path: &str) {
        self.cache.remove(route_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_find_og_image_for_static_route() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let generator = OgImageGenerator::new(runtime, std::path::PathBuf::from("."));

        let entry = OgImageEntry {
            path: "/blog".to_string(),
            file_path: "blog/opengraph-image.tsx".to_string(),
            width: Some(1200),
            height: Some(630),
            content_type: Some("image/png".to_string()),
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
        let generator = OgImageGenerator::new(runtime, std::path::PathBuf::from("."));

        let entry = OgImageEntry {
            path: "/blog/[slug]".to_string(),
            file_path: "blog/[slug]/opengraph-image.tsx".to_string(),
            width: Some(1200),
            height: Some(630),
            content_type: Some("image/png".to_string()),
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
        let generator = OgImageGenerator::new(runtime, std::path::PathBuf::from("."));

        let found = generator.find_og_image_for_route("/nonexistent").await;
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_find_og_image_prefers_exact_match() {
        let runtime = Arc::new(JsExecutionRuntime::new(None));
        let generator = OgImageGenerator::new(runtime, std::path::PathBuf::from("."));

        let dynamic_entry = OgImageEntry {
            path: "/blog/[slug]".to_string(),
            file_path: "blog/[slug]/opengraph-image.tsx".to_string(),
            width: Some(1200),
            height: Some(630),
            content_type: Some("image/png".to_string()),
        };

        let static_entry = OgImageEntry {
            path: "/blog/featured".to_string(),
            file_path: "blog/featured/opengraph-image.tsx".to_string(),
            width: Some(1600),
            height: Some(900),
            content_type: Some("image/png".to_string()),
        };

        generator.register_component("/blog/[slug]".to_string(), dynamic_entry).await;
        generator.register_component("/blog/featured".to_string(), static_entry).await;

        let found = generator.find_og_image_for_route("/blog/featured").await;
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.path, "/blog/featured");
        assert_eq!(found.width, Some(1600));
    }
}
