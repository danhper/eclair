use futures_util::lock::Mutex;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use alloy::{
    network::{AnyNetwork, Ethereum, EthereumWallet, NetworkWallet, TxSigner},
    primitives::Address,
    providers::{Provider, ProviderBuilder},
    signers::{ledger::HDPath, local::PrivateKeySigner, Signature},
    transports::http::{Client, Http},
};
use anyhow::Result;
use coins_ledger::{transports::LedgerAsync, Ledger};

use crate::vendor::ledger_signer::LedgerSigner;

use super::{types::Type, Value};

pub struct Env {
    variables: Vec<HashMap<String, Value>>,
    types: HashMap<String, Type>,
    debug: bool,
    provider: Arc<dyn Provider<Http<Client>, Ethereum>>,
    wallet: Option<EthereumWallet>,
    ledger: Option<Arc<Mutex<Ledger>>>,
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
            wallet: None,
            ledger: None,
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
        self.set_provider(self.wallet.clone(), url)
    }

    pub async fn get_chain_id(&self) -> Result<u64> {
        self.provider
            .root()
            .get_chain_id()
            .await
            .map_err(Into::into)
    }

    pub async fn load_ledger(&mut self, index: usize) -> Result<()> {
        self.init_ledger().await?;
        let chain_id = self.get_chain_id().await?;
        let signer = LedgerSigner::new(
            self.ledger.as_ref().unwrap().clone(),
            HDPath::LedgerLive(index),
            Some(chain_id),
        )
        .await?;
        self.set_wallet(signer)
    }

    pub async fn list_ledger_wallets(&mut self, count: usize) -> Result<Vec<Address>> {
        self.init_ledger().await?;
        let signer = LedgerSigner::new(
            self.ledger.as_ref().unwrap().clone(),
            HDPath::LedgerLive(0),
            None,
        )
        .await?;
        let mut wallets = vec![signer.address()];
        for i in 1..count {
            let addr = signer.get_address_with_path(&HDPath::LedgerLive(i)).await?;
            wallets.push(addr);
        }
        Ok(wallets)
    }

    pub fn get_rpc_url(&self) -> String {
        self.provider.client().transport().url().to_string()
    }

    pub fn get_default_sender(&self) -> Option<Address> {
        self.wallet
            .as_ref()
            .map(NetworkWallet::<AnyNetwork>::default_signer_address)
    }

    pub fn set_private_key(&mut self, private_key: &str) -> Result<()> {
        let signer: PrivateKeySigner = private_key.parse()?;
        self.set_wallet(signer)
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

    fn set_wallet<S>(&mut self, signer: S) -> Result<()>
    where
        S: TxSigner<Signature> + Send + Sync + 'static,
    {
        let wallet = EthereumWallet::from(signer);
        self.wallet = Some(wallet.clone());
        self.set_provider(Some(wallet), &self.get_rpc_url())
    }

    fn set_provider(&mut self, wallet: Option<EthereumWallet>, url: &str) -> Result<()> {
        let rpc_url = url.parse()?;

        let builder = ProviderBuilder::new().with_recommended_fillers();
        let provider = if let Some(wallet) = wallet {
            Arc::new(builder.wallet(wallet.clone()).on_http(rpc_url))
                as Arc<dyn Provider<Http<Client>, Ethereum>>
        } else {
            Arc::new(builder.on_http(rpc_url))
        };
        self.provider = provider;
        Ok(())
    }

    async fn init_ledger(&mut self) -> Result<()> {
        if self.ledger.is_none() {
            let ledger = Ledger::init().await?;
            self.ledger = Some(Arc::new(Mutex::new(ledger)));
        }
        Ok(())
    }
}
