use rari_error::RariError;
use serde::{Deserialize, Serialize};
use tokio::fs;

use super::{
    api_routes::{ApiRouteEntry, ApiRouteManifest},
    app_router::AppRouteManifest,
};
use crate::server::og::OgImageEntry;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RoutesManifest {
    #[serde(flatten)]
    pub app: AppRouteManifest,
    #[serde(rename = "apiRoutes", default)]
    pub api_routes: Vec<ApiRouteEntry>,
    #[serde(rename = "ogImages", default)]
    pub og_images: Vec<OgImageEntry>,
}

impl RoutesManifest {
    #[expect(clippy::missing_errors_doc)]
    pub async fn load_from_file(path: &str) -> Result<Self, RariError> {
        let content = fs::read_to_string(path)
            .await
            .map_err(|e| RariError::io(format!("Failed to read routes manifest: {e}")))?;

        serde_json::from_str(&content)
            .map_err(|e| RariError::configuration(format!("Failed to parse routes manifest: {e}")))
    }

    pub fn api_manifest(&self) -> ApiRouteManifest {
        ApiRouteManifest { api_routes: self.api_routes.clone() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_shared_routes_manifest() {
        let json = r#"{
            "routes": [{"path": "/", "filePath": "page.tsx", "segments": [], "params": [], "isDynamic": false, "componentId": "app/page"}],
            "layouts": [],
            "loading": [],
            "errors": [],
            "notFound": [],
            "templates": [],
            "generated": "2026-01-01",
            "apiRoutes": [{"path": "/api/hello", "filePath": "api/hello/route.ts", "methods": ["GET"], "segments": [], "params": [], "isDynamic": false, "componentId": "app/api/hello/route"}],
            "ogImages": [{"path": "/", "filePath": "opengraph-image.tsx", "width": 1200, "height": 630}]
        }"#;

        #[expect(clippy::expect_used)]
        let manifest: RoutesManifest = serde_json::from_str(json).expect("manifest should parse");
        assert_eq!(manifest.app.routes.len(), 1);
        assert_eq!(manifest.api_routes.len(), 1);
        assert_eq!(manifest.og_images.len(), 1);
        assert_eq!(manifest.api_manifest().api_routes.len(), 1);
    }
}
