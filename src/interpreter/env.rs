use std::collections::HashMap;
use std::sync::Arc;

use ethers::abi::Abi;
use ethers::providers::{Http, Provider};

use super::Value;

pub struct Env {
    variables: HashMap<String, Value>,
    types: HashMap<String, Abi>,
    pub provider: Arc<Provider<Http>>,
}

impl Env {
    pub fn new() -> Self {
        Env {
            variables: HashMap::new(),
            types: HashMap::new(),
            provider: Arc::new(
                Provider::<Http>::try_from("http://localhost:8545")
                    .expect("could not create provider"),
            ),
        }
    }

    pub fn set_type(&mut self, name: &str, abi: Abi) {
        self.types.insert(name.to_string(), abi);
    }

    pub fn get_type(&self, name: &str) -> Option<&Abi> {
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
