use dashmap::DashMap;

#[derive(Debug)]
pub struct ModuleStorage {
    code: DashMap<String, String>,
    meta: DashMap<String, bool>,
    versions: DashMap<String, u64>,
}

impl ModuleStorage {
    pub fn new() -> Self {
        Self { code: DashMap::new(), meta: DashMap::new(), versions: DashMap::new() }
    }

    pub fn get_module_code(&self, specifier: &str) -> Option<String> {
        self.code.get(specifier).map(|entry| entry.value().clone())
    }

    pub fn set_module_code(&self, specifier: String, code: String) {
        self.code.insert(specifier, code);
    }

    pub fn contains_module_code(&self, specifier: &str) -> bool {
        self.code.contains_key(specifier)
    }

    pub fn get_module_meta(&self, key: &str) -> Option<bool> {
        self.meta.get(key).map(|entry| *entry.value())
    }

    pub fn set_module_meta(&self, key: String, value: bool) {
        self.meta.insert(key, value);
    }

    pub fn get_version(&self, key: &str) -> Option<u64> {
        self.versions.get(key).map(|entry| *entry.value())
    }

    pub fn set_version(&self, key: String, version: u64) {
        self.versions.insert(key, version);
    }
}

impl Default for ModuleStorage {
    fn default() -> Self {
        Self::new()
    }
}
