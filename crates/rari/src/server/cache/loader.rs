use std::path::Path;

use cow_utils::CowUtils;
use rari_error::RariError;
use regex::Regex;
use rustc_hash::FxHashMap;
use tokio::fs;

use crate::server::ServerState;

#[non_exhaustive]
pub struct CacheLoader;

impl CacheLoader {
    #[expect(clippy::missing_errors_doc)]
    pub async fn load_page_cache_configs(state: &ServerState) -> Result<(), RariError> {
        let pages_dir = Path::new("src/pages");
        if !fs::try_exists(pages_dir).await.unwrap_or(false) {
            return Ok(());
        }

        let mut loaded_count = 0;
        Self::scan_pages_directory(pages_dir, state, &mut loaded_count).await?;

        Ok(())
    }

    async fn scan_pages_directory(
        dir: &Path,
        state: &ServerState,
        loaded_count: &mut usize,
    ) -> Result<(), RariError> {
        let mut dirs_to_scan = vec![dir.to_path_buf()];

        while let Some(current_dir) = dirs_to_scan.pop() {
            let mut entries = fs::read_dir(&current_dir)
                .await
                .map_err(|e| RariError::io(format!("Failed to read pages directory: {e}")))?;

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
                    dirs_to_scan.push(path);
                } else if {
                    #[expect(
                        clippy::filetype_is_file,
                        reason = "Page cache config is only read from regular source files"
                    )]
                    file_type.is_file()
                } && let Some(extension) = path.extension()
                    && (extension == "tsx"
                        || extension == "jsx"
                        || extension == "ts"
                        || extension == "js")
                {
                    if let Err(e) = Self::load_page_cache_config(&path, state).await {
                        tracing::error!("Failed to load page cache config for {:?}: {}", path, e);
                    }
                    *loaded_count += 1;
                }
            }
        }

        Ok(())
    }

    async fn load_page_cache_config(
        page_path: &Path,
        state: &ServerState,
    ) -> Result<(), RariError> {
        let content = fs::read_to_string(page_path)
            .await
            .map_err(|e| RariError::io(format!("Failed to read page file: {e}")))?;

        if let Some(cache_config) = Self::extract_cache_config_from_content(&content) {
            let route_path = Self::page_path_to_route(page_path)?;

            let mut page_configs = state.page_cache_configs.write().await;
            page_configs.insert(route_path.clone(), cache_config);
        }

        Ok(())
    }

    fn page_path_to_route(page_path: &Path) -> Result<String, RariError> {
        let pages_dir = Path::new("src/pages");
        let relative_path = page_path.strip_prefix(pages_dir).map_err(|_| {
            RariError::configuration("Page path is not within pages directory".to_string())
        })?;

        let route =
            relative_path.with_extension("").to_string_lossy().cow_replace('\\', "/").into_owned();

        let route = if route == "index" { "/".to_string() } else { format!("/{route}") };

        Ok(route)
    }

    fn extract_cache_config_from_content(content: &str) -> Option<FxHashMap<String, String>> {
        let cache_config_regex =
            Regex::new(r"export\s+const\s+cacheConfig\s*:\s*\w+\s*=\s*\{([^}]+)\}").ok()?;

        if let Some(captures) = cache_config_regex.captures(content) {
            let config_content = captures.get(1)?.as_str();
            let mut config = FxHashMap::default();

            let cache_control_regex = Regex::new(r"'cache-control'\s*:\s*'([^']+)'").ok()?;
            if let Some(cache_control_match) = cache_control_regex.captures(config_content) {
                config.insert(
                    "cache-control".to_string(),
                    cache_control_match.get(1)?.as_str().to_string(),
                );
            }

            let vary_regex = Regex::new(r"'vary'\s*:\s*'([^']+)'").ok()?;
            if let Some(vary_match) = vary_regex.captures(config_content) {
                config.insert("vary".to_string(), vary_match.get(1)?.as_str().to_string());
            }

            if !config.is_empty() {
                return Some(config);
            }
        }

        None
    }
}
