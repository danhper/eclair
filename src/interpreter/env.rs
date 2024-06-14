use std::collections::HashMap;

use ethers::abi::Abi;

use super::Value;

pub(crate) struct Env {
    variables: HashMap<String, Value>,
    types: HashMap<String, Abi>,
}

impl Env {
    pub(crate) fn new() -> Self {
        Env {
            variables: HashMap::new(),
            types: HashMap::new(),
        }
    }

    pub fn set_type(&mut self, name: &str, abi: Abi) {
        self.types.insert(name.to_string(), abi);
    }

    pub fn get_type(&self, name: &str) -> Option<&Abi> {
        self.types.get(name)
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
