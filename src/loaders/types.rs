use alloy::json_abi::JsonAbi;
use std::collections::HashMap;

pub struct Project {
    abis: HashMap<String, JsonAbi>,
}

impl Project {
    pub fn new(abis: HashMap<String, JsonAbi>) -> Self {
        Project { abis }
    }

    pub fn get_contract(&self, name: &str) -> JsonAbi {
        self.abis.get(name).expect("Contract not found").clone()
    }

    pub fn contract_names(&self) -> Vec<String> {
        self.abis.keys().cloned().collect()
    }
}
