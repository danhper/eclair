use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use alloy::{
    providers::{ProviderBuilder, RootProvider},
    transports::http::{Client, Http},
};

use super::{types::Type, Value};

#[derive(Debug)]
pub struct Env {
    variables: Vec<HashMap<String, Value>>,
    types: HashMap<String, Type>,
    debug: bool,
    provider: Arc<RootProvider<Http<Client>>>,
}

unsafe impl std::marker::Send for Env {}

impl Env {
    pub fn new(provider_url: &str, debug: bool) -> Self {
        let rpc_url = provider_url.parse().unwrap();
        let provider = ProviderBuilder::new().on_http(rpc_url);
        Env {
            variables: vec![HashMap::new()],
            types: HashMap::new(),
            provider: Arc::new(provider),
            debug,
        }
    }

    pub fn push_scope(&mut self) {
        self.variables.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.variables.pop();
    }

    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
    }

    pub fn is_debug(&self) -> bool {
        self.debug
    }

    pub fn get_provider(&self) -> Arc<RootProvider<Http<Client>>> {
        self.provider.clone()
    }

    pub fn set_provider(&mut self, url: &str) {
        let rpc_url = url.parse().unwrap();
        let provider = ProviderBuilder::new().on_http(rpc_url);
        self.provider = Arc::new(provider);
    }

    pub fn set_type(&mut self, name: &str, type_: Type) {
        self.types.insert(name.to_string(), type_);
    }

    pub fn get_type(&self, name: &str) -> Option<&Type> {
        self.types.get(name)
    }

    pub fn list_types(&self) -> Vec<String> {
        self.types.keys().cloned().collect()
    }

    pub fn list_vars(&self) -> Vec<String> {
        let mut vars = HashSet::new();
        for scope in &self.variables {
            for var in scope.keys() {
                vars.insert(var.clone());
            }
        }
        Vec::from_iter(vars)
    }

    pub fn get_var(&self, name: &str) -> Option<&Value> {
        let scope = self.variables.last().unwrap();
        scope.get(name)
    }

    pub fn get_var_mut(&mut self, name: &str) -> Option<&mut Value> {
        let scope = self.variables.last_mut().unwrap();
        scope.get_mut(name)
    }

    pub fn set_var(&mut self, name: &str, value: Value) {
        let scope = self.variables.last_mut().unwrap();
        scope.insert(name.to_string(), value);
    }
}
