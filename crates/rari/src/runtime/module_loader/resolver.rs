use dashmap::DashMap;

#[derive(Debug)]
pub struct ModuleResolver {
    resolved_packages: DashMap<String, String>,
}

impl ModuleResolver {
    pub fn new() -> Self {
        Self { resolved_packages: DashMap::new() }
    }

    pub fn cache_package_resolution(&self, package_name: String, resolved_path: String) {
        self.resolved_packages.insert(package_name, resolved_path);
    }

    pub fn get_cached_package(&self, package_name: &str) -> Option<String> {
        self.resolved_packages.get(package_name).map(|entry| entry.value().clone())
    }

    pub fn clear_cache(&self) {
        self.resolved_packages.clear();
    }

    pub fn cache_size(&self) -> usize {
        self.resolved_packages.len()
    }

    pub fn contains_path(&self, path: &str) -> bool {
        self.resolved_packages.iter().any(|entry| path.contains(entry.value()))
    }

    pub fn get_package_base(&self, referrer: &str) -> Option<String> {
        for entry in self.resolved_packages.iter() {
            let package_path = entry.value();
            if referrer.contains(package_path.as_str()) {
                if let Some(base_dir) = package_path.rsplit_once('/') {
                    return Some(base_dir.0.to_string());
                }
                return Some(package_path.clone());
            }
        }
        None
    }
}

impl Default for ModuleResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolver_cache() {
        let resolver = ModuleResolver::new();

        resolver.cache_package_resolution("react".to_string(), "/node_modules/react".to_string());

        assert_eq!(resolver.get_cached_package("react"), Some("/node_modules/react".to_string()));
        assert_eq!(resolver.cache_size(), 1);
    }

    #[test]
    fn test_resolver_clear() {
        let resolver = ModuleResolver::new();

        resolver.cache_package_resolution("react".to_string(), "/node_modules/react".to_string());
        assert_eq!(resolver.cache_size(), 1);

        resolver.clear_cache();
        assert_eq!(resolver.cache_size(), 0);
    }
}
