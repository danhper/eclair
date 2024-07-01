use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use alloy::{
    network::{Ethereum, EthereumWallet},
    providers::{Provider, ProviderBuilder},
    signers::local::PrivateKeySigner,
    transports::http::{Client, Http},
};
use anyhow::Result;

use super::{types::Type, Value};

pub struct Env {
    variables: Vec<HashMap<String, Value>>,
    types: HashMap<String, Type>,
    debug: bool,
    provider: Arc<dyn Provider<Http<Client>, Ethereum>>,
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

    pub fn get_provider(&self) -> Arc<dyn Provider<Http<Client>, Ethereum>> {
        self.provider.clone()
    }

    pub fn set_provider_url(&mut self, url: &str) -> Result<()> {
        let rpc_url = url.parse()?;
        let provider = ProviderBuilder::new().on_http(rpc_url);
        self.provider = Arc::new(provider);
        Ok(())
    }

    pub fn get_rpc_url(&self) -> String {
        self.provider.client().transport().url().to_string()
    }

    pub fn set_private_key(&mut self, private_key: &str) -> Result<()> {
        let rpc_url = self.get_rpc_url().parse()?;
        let signer: PrivateKeySigner = private_key.parse()?;
        let wallet = EthereumWallet::from(signer);
        let provider = ProviderBuilder::new().wallet(wallet).on_http(rpc_url);
        self.provider = Arc::new(provider);
        Ok(())
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
        for scope in self.variables.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Some(value);
            }
        }
        None
    }

    pub fn get_var_mut(&mut self, name: &str) -> Option<&mut Value> {
        for scope in self.variables.iter_mut().rev() {
            if let Some(value) = scope.get_mut(name) {
                return Some(value);
            }
        }
        None
    }

    pub fn delete_var(&mut self, name: &str) {
        for scope in self.variables.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.remove(name);
                return;
            }
        }
    }

    pub fn set_var(&mut self, name: &str, value: Value) {
        let scope = self.variables.last_mut().unwrap();
        scope.insert(name.to_string(), value);
    }
}
