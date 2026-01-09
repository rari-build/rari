use crate::error::RariError;
use crate::server::ServerState;
use cow_utils::CowUtils;
use regex::Regex;
use rustc_hash::FxHashMap;
use tracing::error;

pub struct CacheLoader;

impl CacheLoader {
    pub async fn load_page_cache_configs(state: &ServerState) -> Result<(), RariError> {
        let pages_dir = std::path::Path::new("src/pages");
        if !pages_dir.exists() {
            return Ok(());
        }

        let mut loaded_count = 0;
        Self::scan_pages_directory(pages_dir, state, &mut loaded_count).await?;

        Ok(())
    }

    async fn scan_pages_directory(
        dir: &std::path::Path,
        state: &ServerState,
        loaded_count: &mut usize,
    ) -> Result<(), RariError> {
        let mut dirs_to_scan = vec![dir.to_path_buf()];

        while let Some(current_dir) = dirs_to_scan.pop() {
            let entries = std::fs::read_dir(&current_dir)
                .map_err(|e| RariError::io(format!("Failed to read pages directory: {e}")))?;

            for entry in entries {
                let entry = entry
                    .map_err(|e| RariError::io(format!("Failed to read directory entry: {e}")))?;
                let path = entry.path();

                if path.is_dir() {
                    dirs_to_scan.push(path);
                } else if path.is_file()
                    && let Some(extension) = path.extension()
                    && (extension == "tsx"
                        || extension == "jsx"
                        || extension == "ts"
                        || extension == "js")
                {
                    if let Err(e) = Self::load_page_cache_config(&path, state).await {
                        error!("Failed to load page cache config for {:?}: {}", path, e);
                    }
                    *loaded_count += 1;
                }
            }
        }

        Ok(())
    }

    async fn load_page_cache_config(
        page_path: &std::path::Path,
        state: &ServerState,
    ) -> Result<(), RariError> {
        let content = std::fs::read_to_string(page_path)
            .map_err(|e| RariError::io(format!("Failed to read page file: {e}")))?;

        if let Some(cache_config) = Self::extract_cache_config_from_content(&content) {
            let route_path = Self::page_path_to_route(page_path)?;

            let mut page_configs = state.page_cache_configs.write().await;
            page_configs.insert(route_path.clone(), cache_config);
        }

        Ok(())
    }

    fn page_path_to_route(page_path: &std::path::Path) -> Result<String, RariError> {
        let pages_dir = std::path::Path::new("src/pages");
        let relative_path = page_path.strip_prefix(pages_dir).map_err(|_| {
            RariError::configuration("Page path is not within pages directory".to_string())
        })?;

        let route =
            relative_path.with_extension("").to_string_lossy().cow_replace('\\', "/").into_owned();

        let route = if route == "index" { "/".to_string() } else { format!("/{}", route) };

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

    pub fn find_matching_cache_config<'a>(
        page_configs: &'a FxHashMap<String, FxHashMap<String, String>>,
        route_path: &str,
    ) -> Option<&'a FxHashMap<String, String>> {
        if let Some(config) = page_configs.get(route_path) {
            return Some(config);
        }

        for (pattern, config) in page_configs {
            if Self::matches_route_pattern(pattern, route_path) {
                return Some(config);
            }
        }

        None
    }

    fn matches_route_pattern(pattern: &str, path: &str) -> bool {
        if pattern == path {
            return true;
        }

        if let Some(prefix) = pattern.strip_suffix("/*") {
            return path.starts_with(prefix)
                && (path.len() == prefix.len() || path.chars().nth(prefix.len()) == Some('/'));
        }

        if pattern.contains('*') {
            let regex_pattern = pattern.cow_replace('*', ".*").cow_replace('/', "\\/").into_owned();
            if let Ok(regex) = regex::Regex::new(&format!("^{}$", regex_pattern)) {
                return regex.is_match(path);
            }
        }

        false
    }

    pub async fn load_vite_cache_config(state: &ServerState) -> Result<(), RariError> {
        let cache_config_path = std::path::Path::new("dist/cache-config.json");

        if !cache_config_path.exists() {
            return Ok(());
        }

        if let Ok(content) = std::fs::read_to_string(cache_config_path)
            && let Ok(config_json) = serde_json::from_str::<serde_json::Value>(&content)
            && let Some(routes) = config_json.get("routes").and_then(|r| r.as_object())
        {
            let mut page_configs = state.page_cache_configs.write().await;

            for (route, cache_control) in routes {
                if let Some(cache_str) = cache_control.as_str()
                    && !page_configs.contains_key(route)
                {
                    let mut cache_config = FxHashMap::default();
                    cache_config.insert("cache-control".to_string(), cache_str.to_string());
                    page_configs.insert(route.clone(), cache_config);
                }
            }
        }

        Ok(())
    }
}
