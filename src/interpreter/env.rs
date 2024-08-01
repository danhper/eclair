use futures_util::lock::Mutex;
use solang_parser::pt::{Expression, Identifier};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use url::Url;

use alloy::{
    eips::BlockId,
    network::{AnyNetwork, Ethereum, EthereumWallet, NetworkWallet, TxSigner},
    primitives::Address,
    providers::{Provider, ProviderBuilder},
    signers::{ledger::HDPath, Signature},
    transports::http::{Client, Http},
};
use anyhow::{anyhow, bail, Result};
use coins_ledger::{transports::LedgerAsync, Ledger};

use crate::{interpreter::Config, vendor::ledger_signer::LedgerSigner};

use super::{evaluate_expression, types::Type, Value};

pub struct Env {
    variables: Vec<HashMap<String, Value>>,
    types: HashMap<String, Type>,
    provider: Arc<dyn Provider<Http<Client>, Ethereum>>,
    wallet: Option<EthereumWallet>,
    ledger: Option<Arc<Mutex<Ledger>>>,
    block_id: BlockId,
    pub config: Config,
}

unsafe impl std::marker::Send for Env {}

impl Env {
    pub fn new(config: Config) -> Self {
        let rpc_url = config.rpc_url.parse().unwrap();
        let provider = ProviderBuilder::new().on_http(rpc_url);
        Env {
            variables: vec![HashMap::new()],
            types: HashMap::new(),
            provider: Arc::new(provider),
            wallet: None,
            ledger: None,
            block_id: BlockId::latest(),
            config,
        }
    }

    pub fn push_scope(&mut self) {
        self.variables.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.variables.pop();
    }

    pub fn set_debug(&mut self, debug: bool) {
        self.config.debug = debug;
    }

    pub fn is_debug(&self) -> bool {
        self.config.debug
    }

    pub fn set_block(&mut self, block: BlockId) {
        self.block_id = block;
    }

    pub fn block(&self) -> BlockId {
        self.block_id
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

    pub fn set_signer<S>(&mut self, signer: S) -> Result<()>
    where
        S: TxSigner<Signature> + Send + Sync + 'static,
    {
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

    pub async fn init_variable(
        &mut self,
        name: &Option<Identifier>,
        type_: &Expression,
        initializer: &Option<Expression>,
    ) -> Result<()> {
        let id = name.clone().ok_or(anyhow!("invalid declaration"))?.name;
        let type_ = match evaluate_expression(self, Box::new(type_.clone())).await? {
            Value::TypeObject(t) => t,
            v => bail!("invalid type for variable, expected type, got {}", v),
        };
        let value = if let Some(e) = initializer {
            evaluate_expression(self, Box::new(e.clone())).await?
        } else {
            type_.default_value()?
        };
        self.set_var(&id, value);
        Ok(())
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
        let rpc_url = match url.parse() {
            Ok(u) => u,
            Err(_) => self
                .config
                .rpc_endpoints
                .get(url)
                .ok_or(anyhow!("invalid URL and no config for {}", url))
                .and_then(|u| u.parse::<Url>().map_err(Into::into))?,
        };
        self.config.rpc_url = rpc_url.to_string();

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
