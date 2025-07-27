use crate::error::RariError;
use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerFunction {
    pub id: String,
    pub name: String,
    pub module_path: String,
    pub code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterFunctionsRequest {
    pub functions: Vec<ServerFunctionRegistration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerFunctionRegistration {
    pub id: String,
    pub name: String,
    pub file_path: String,
}

pub struct ServerFunctionRegistry {
    functions: RwLock<FxHashMap<String, ServerFunction>>,
}

impl ServerFunctionRegistry {
    pub fn new() -> Self {
        Self { functions: RwLock::new(FxHashMap::default()) }
    }

    pub fn register(&self, function: ServerFunction) -> Result<(), RariError> {
        let mut functions = self.functions.write();

        functions.insert(function.id.clone(), function);
        Ok(())
    }

    pub fn register_batch(
        &self,
        registrations: Vec<ServerFunctionRegistration>,
    ) -> Result<(), RariError> {
        let mut functions = self.functions.write();

        for reg in registrations {
            functions.insert(
                reg.id.clone(),
                ServerFunction {
                    id: reg.id,
                    name: reg.name,
                    module_path: reg.file_path,
                    code: None,
                },
            );
        }

        Ok(())
    }

    pub fn get(&self, id: &str) -> Result<Option<ServerFunction>, RariError> {
        let functions = self.functions.read();

        Ok(functions.get(id).cloned())
    }

    pub fn list(&self) -> Result<Vec<ServerFunction>, RariError> {
        let functions = self.functions.read();

        Ok(functions.values().cloned().collect())
    }
}

impl Default for ServerFunctionRegistry {
    fn default() -> Self {
        Self::new()
    }
}
