use std::path::PathBuf;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct ModuleReloadRequest {
    pub component_id: String,
    pub file_path: PathBuf,
    pub timestamp: Instant,
}

impl ModuleReloadRequest {
    pub fn new(component_id: String, file_path: PathBuf) -> Self {
        Self { component_id, file_path, timestamp: Instant::now() }
    }

    pub fn is_older_than(&self, duration: std::time::Duration) -> bool {
        self.timestamp.elapsed() > duration
    }

    pub fn age(&self) -> std::time::Duration {
        self.timestamp.elapsed()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.component_id.is_empty() {
            return Err("Component ID cannot be empty".to_string());
        }

        if !self.file_path.exists() {
            return Err(format!("File path does not exist: {:?}", self.file_path));
        }

        Ok(())
    }
}
