use futures_util::lock::Mutex;
use solang_parser::pt::{Expression, Identifier};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use url::Url;

use alloy::{
    eips::BlockId,
    json_abi,
    network::{AnyNetwork, Ethereum, EthereumWallet, NetworkWallet, TxSigner},
    node_bindings::{Anvil, AnvilInstance},
    primitives::{Address, FixedBytes, B256},
    providers::{
        ext::AnvilApi,
        fillers::{FillProvider, JoinFill, RecommendedFiller},
        Provider, ProviderBuilder, RootProvider, WalletProvider,
    },
    signers::{ledger::HDPath, Signature},
    transports::http::{Client, Http},
};
use anyhow::{anyhow, bail, Result};
use coins_ledger::{transports::LedgerAsync, Ledger};

use crate::{
    interpreter::Config,
    vendor::{ledger_signer::LedgerSigner, optional_wallet_filler::OptionalWalletFiller},
};

use super::{evaluate_expression, types::Type, ContractInfo, Value};

type RecommendedFillerWithWallet =
    JoinFill<RecommendedFiller, OptionalWalletFiller<EthereumWallet>>;
type EclairProvider =
    FillProvider<RecommendedFillerWithWallet, RootProvider<Http<Client>>, Http<Client>, Ethereum>;

pub struct Env {
    variables: Vec<HashMap<String, Value>>,
    types: HashMap<String, Type>,
    provider: EclairProvider,
    is_wallet_connected: bool,
    ledger: Option<Arc<Mutex<Ledger>>>,
    loaded_wallets: HashMap<Address, EthereumWallet>,
    block_id: BlockId,
    contract_names: HashMap<Address, String>,
    events: HashMap<B256, json_abi::Event>,
    errors: HashMap<FixedBytes<4>, json_abi::Error>,
    functions: HashMap<FixedBytes<4>, json_abi::Function>,
    impersonating: Option<Address>,
    anvil: Option<AnvilInstance>,
    pub config: Config,
    account_aliases: HashMap<String, Address>,
}

unsafe impl std::marker::Send for Env {}

impl Env {
    pub fn new(config: Config) -> Self {
        let rpc_url = config.rpc_url.parse().unwrap();
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .filler(OptionalWalletFiller::<EthereumWallet>::new())
            .on_http(rpc_url);
        Env {
            variables: vec![HashMap::new()],
            types: HashMap::new(),
            provider,
            is_wallet_connected: false,
            ledger: None,
            loaded_wallets: HashMap::new(),
            block_id: BlockId::latest(),
            contract_names: HashMap::new(),
            events: HashMap::new(),
            errors: HashMap::new(),
            functions: HashMap::new(),
            impersonating: None,
            anvil: None,
            config,
            account_aliases: HashMap::new(),
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

    pub fn get_event(&self, selector: &B256) -> Option<&json_abi::Event> {
        self.events.get(selector)
    }

    pub fn get_error(&self, selector: &FixedBytes<4>) -> Option<&json_abi::Error> {
        self.errors.get(selector)
    }

    pub fn get_function(&self, selector: &FixedBytes<4>) -> Option<&json_abi::Function> {
        self.functions.get(selector)
    }

    pub fn add_contract(&mut self, name: &str, abi: json_abi::JsonAbi) -> ContractInfo {
        for event in abi.events() {
            self.register_event(event.clone());
        }
        for error in abi.errors() {
            self.register_error(error.clone());
        }
        for function in abi.functions() {
            self.register_function(function.clone());
        }
        let contract_info = ContractInfo(name.to_string(), abi);
        self.set_type(name, Type::Contract(contract_info.clone()));
        contract_info
    }

    pub fn list_events(&mut self) -> Vec<&json_abi::Event> {
        self.events.values().collect()
    }

    pub fn register_event(&mut self, event: json_abi::Event) {
        self.events.insert(event.selector(), event);
    }

    pub fn register_error(&mut self, error: json_abi::Error) {
        self.errors.insert(error.selector(), error);
    }

    pub fn register_function(&mut self, function: json_abi::Function) {
        self.functions.insert(function.selector(), function);
    }

    pub fn set_block(&mut self, block: BlockId) {
        self.block_id = block;
    }

    pub fn block(&self) -> BlockId {
        self.block_id
    }

    pub async fn get_block_number(&self) -> Result<u64> {
        self.provider.get_block_number().await.map_err(Into::into)
    }

    pub fn get_provider(&self) -> EclairProvider {
        self.provider.clone()
    }

    pub fn set_provider_url(&mut self, url: &str) -> Result<()> {
        self.set_provider(None, url)
    }

    pub async fn get_chain_id(&self) -> Result<u64> {
        self.provider.get_chain_id().await.map_err(Into::into)
    }

    pub async fn fork(&mut self, url: &str, block_num: Option<u64>) -> Result<()> {
        let anvil_setup = Anvil::new().arg("--steps-tracing").fork(url);
        let anvil = if let Some(block_num) = block_num {
            anvil_setup.fork_block_number(block_num)
        } else {
            anvil_setup.fork_block_number(self.get_block_number().await?)
        }
        .try_spawn()?;
        let endpoint = anvil.endpoint();
        self.set_provider_url(endpoint.as_str())?;
        self.anvil = Some(anvil);
        Ok(())
    }

    pub fn is_fork(&self) -> bool {
        self.anvil.is_some()
    }

    pub async fn impersonate(&mut self, address: Address) -> Result<()> {
        if let Some(addr) = self.impersonating {
            bail!("already impersonating {}", addr);
        }
        self.provider.anvil_impersonate_account(address).await?;
        self.impersonating = Some(address);
        Ok(())
    }

    pub async fn stop_impersonate(&mut self) -> Result<()> {
        if self.anvil.is_none() {
            bail!("can only impersonate in forks");
        }
        let address = self.impersonating.ok_or(anyhow!("not impersonating"))?;
        self.provider.anvil_impersonate_account(address).await?;
        self.impersonating = None;
        Ok(())
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
        if let addr @ Some(_) = self.impersonating {
            addr
        } else if self.is_wallet_connected {
            Some(NetworkWallet::<AnyNetwork>::default_signer_address(
                self.provider.wallet(),
            ))
        } else {
            None
        }
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

    pub fn get_contract_name(&self, addr: &Address) -> Option<&String> {
        self.contract_names.get(addr)
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
        let is_top_level = self.variables.len() == 1;
        let scope = self.variables.last_mut().unwrap();
        if is_top_level {
            if let Value::Contract(ContractInfo(name, _), addr) = &value {
                self.contract_names.insert(*addr, name.clone());
            }
        }
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

    pub fn select_wallet(&mut self, address: Address) -> Result<()> {
        let wallet = self
            .loaded_wallets
            .get(&address)
            .ok_or_else(|| anyhow!("no wallet loaded for address {}", address))?
            .clone();
        self._select_wallet(wallet)
    }

    pub fn select_wallet_by_alias(&mut self, alias: &str) -> Result<()> {
        let address = self
            .account_aliases
            .get(alias)
            .ok_or_else(|| anyhow!("no alias found for {}", alias))?;
        self.select_wallet(*address)
    }

    fn set_wallet<S>(&mut self, signer: S) -> Result<()>
    where
        S: TxSigner<Signature> + Send + Sync + 'static,
    {
        let wallet = EthereumWallet::from(signer);
        let address = NetworkWallet::<AnyNetwork>::default_signer_address(&wallet);
        self.loaded_wallets.insert(address, wallet.clone());
        self.is_wallet_connected = true;
        self._select_wallet(wallet)
    }

    pub fn get_loaded_wallets(&self) -> Vec<Address> {
        self.loaded_wallets.keys().cloned().collect()
    }

    fn _select_wallet(&mut self, wallet: EthereumWallet) -> Result<()> {
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

        let mut wallet_filler = OptionalWalletFiller::new();
        if let Some(w) = wallet {
            wallet_filler.set_wallet(w);
        } else if self.is_wallet_connected {
            wallet_filler.set_wallet(self.provider.wallet().clone());
        }
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .filler(wallet_filler)
            .on_http(rpc_url);
        self.provider = provider;
        self.anvil = None;
        Ok(())
    }

    async fn init_ledger(&mut self) -> Result<()> {
        if self.ledger.is_none() {
            let ledger = Ledger::init().await?;
            self.ledger = Some(Arc::new(Mutex::new(ledger)));
        }
        Ok(())
    }

    pub fn set_account_alias(&mut self, alias: &str, address: Address) {
        self.account_aliases.insert(alias.to_string(), address);
    }

    pub fn list_account_aliases(&self) -> HashMap<String, Address> {
        self.account_aliases.clone()
    }
}
