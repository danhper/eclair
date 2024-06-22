use std::collections::HashMap;

use alloy::json_abi::JsonAbi;

use super::Value;

#[derive(Debug)]
pub struct Env {
    variables: HashMap<String, Value>,
    types: HashMap<String, JsonAbi>,
}

unsafe impl std::marker::Send for Env {}

impl Env {
    pub fn new() -> Self {
        Env {
            variables: HashMap::new(),
            types: HashMap::new(),
        }
    }

    pub fn set_type(&mut self, name: &str, abi: JsonAbi) {
        self.types.insert(name.to_string(), abi);
    }

    pub fn get_type(&self, name: &str) -> Option<&JsonAbi> {
        self.types.get(name)
    }

    pub fn list_types(&self) -> Vec<String> {
        self.types.keys().cloned().collect()
    }

    pub fn list_vars(&self) -> Vec<String> {
        self.variables.keys().cloned().collect()
    }

    pub fn get_var(&self, name: &str) -> Option<&Value> {
        self.variables.get(name)
    }

    pub fn set_var(&mut self, name: &str, value: Value) {
        self.variables.insert(name.to_string(), value);
    }
}

impl Default for Env {
    fn default() -> Self {
        Self::new()
    }
}
