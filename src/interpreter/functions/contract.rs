use std::{hash::Hash, sync::Arc};

use alloy::{
    contract::{CallBuilder, ContractInstance, Interface},
    eips::BlockId,
    json_abi::StateMutability,
    network::{Network, TransactionBuilder},
    primitives::{keccak256, Address, FixedBytes},
    providers::Provider,
    rpc::types::{TransactionInput, TransactionRequest},
    transports::Transport,
};
use anyhow::{anyhow, bail, Result};
use futures::{future::BoxFuture, FutureExt};
use itertools::Itertools;

use crate::interpreter::{types::HashableIndexMap, ContractInfo, Env, Type, Value};

use super::{Function, FunctionDef, FunctionParam};

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum ContractCallMode {
    Default,
    Encode,
    Call,
    Send,
}

impl std::fmt::Display for ContractCallMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContractCallMode::Default => write!(f, "default"),
            ContractCallMode::Encode => write!(f, "encode"),
            ContractCallMode::Call => write!(f, "call"),
            ContractCallMode::Send => write!(f, "send"),
        }
    }
}

impl TryFrom<&str> for ContractCallMode {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self> {
        match s {
            "encode" => Ok(ContractCallMode::Encode),
            "call" => Ok(ContractCallMode::Call),
            "send" => Ok(ContractCallMode::Send),
            _ => bail!("{} does not exist for contract call", s),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CallOptions {
    value: Option<Box<Value>>,
    block: Option<BlockId>,
    from: Option<Address>,
}

impl CallOptions {
    pub fn validate_send(&self) -> Result<()> {
        if self.block.is_some() {
            bail!("block is only available for calls");
        } else if self.from.is_some() {
            bail!("from is only available for calls");
        } else {
            Ok(())
        }
    }
}

impl Hash for CallOptions {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
        match self.block {
            Some(BlockId::Hash(h)) => h.block_hash.hash(state),
            Some(BlockId::Number(n)) => n.hash(state),
            None => 0.hash(state),
        }
    }
}

impl std::fmt::Display for CallOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(v) = &self.value {
            write!(f, "value: {}", v)
        } else {
            write!(f, "")
        }
    }
}

impl TryFrom<&HashableIndexMap<String, Value>> for CallOptions {
    type Error = anyhow::Error;

    fn try_from(value: &HashableIndexMap<String, Value>) -> std::result::Result<Self, Self::Error> {
        let mut opts = CallOptions::default();
        for (k, v) in value.0.iter() {
            match k.as_str() {
                "value" => opts.value = Some(Box::new(v.clone())),
                "block" => opts.block = Some(v.as_block_id()?),
                "from" => opts.from = Some(v.as_address()?),
                _ => bail!("unexpected key {}", k),
            }
        }
        Ok(opts)
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ContractFunction {
    func_name: String,
    mode: ContractCallMode,
}

impl ContractFunction {
    pub fn arc(name: &str) -> Arc<dyn FunctionDef> {
        Arc::new(Self {
            func_name: name.to_string(),
            mode: ContractCallMode::Default,
        })
    }

    pub fn with_mode(&self, mode: ContractCallMode) -> Self {
        let mut new = self.clone();
        new.mode = mode;
        new
    }

    pub fn get_signature(&self, types_: &[Type]) -> String {
        let mut selector = self.func_name.clone();
        selector.push('(');
        let args_str = types_
            .iter()
            .map(|t| t.canonical_string().expect("canonical string"))
            .join(",");
        selector.push_str(&args_str);
        selector.push(')');
        selector
    }

    pub fn get_selector(&self, types_: &[Type]) -> FixedBytes<4> {
        let signature_hash = keccak256(self.get_signature(types_));
        FixedBytes::<4>::from_slice(&signature_hash[..4])
    }
}

impl FunctionDef for ContractFunction {
    fn name(&self) -> String {
        match self.mode {
            ContractCallMode::Default => self.func_name.clone(),
            _ => format!("{}.{}", self.func_name, self.mode),
        }
    }

    fn get_valid_args(&self, receiver: &Option<Value>) -> Vec<Vec<FunctionParam>> {
        let (ContractInfo(_, abi), _) = receiver.clone().unwrap().as_contract().unwrap();
        let functions = abi.function(&self.func_name).cloned().unwrap_or(vec![]);

        functions
            .into_iter()
            .filter_map(|f| {
                f.inputs
                    .into_iter()
                    .map(FunctionParam::try_from)
                    .collect::<Result<Vec<_>>>()
                    .ok()
            })
            .collect()
    }

    fn is_property(&self) -> bool {
        false
    }

    fn member_access(&self, receiver: &Option<Value>, member: &str) -> Option<Value> {
        ContractCallMode::try_from(member)
            .map(|m| Function::new(Arc::new(self.with_mode(m)), receiver.as_ref()).into())
            .ok()
    }

    fn execute<'a>(
        &'a self,
        env: &'a mut Env,
        values: &'a [Value],
        options: &'a HashableIndexMap<String, Value>,
    ) -> BoxFuture<'a, Result<Value>> {
        let (ContractInfo(_, abi), addr) = values[0].as_contract().unwrap();
        let types_ = values[1..].iter().map(Value::get_type).collect::<Vec<_>>();
        let selector = self.get_selector(&types_);

        async move {
            let abi_func = abi
                .functions()
                .find(|f| f.selector() == selector)
                .ok_or_else(|| anyhow!("function {} not found", self.get_signature(&types_)))?;
            let interface = Interface::new(abi.clone());
            let contract =
                ContractInstance::new(addr, env.get_provider().root().clone(), interface);
            let call_options: CallOptions = options.try_into()?;
            let tokens = values[1..]
                .iter()
                .map(|arg| arg.try_into())
                .collect::<Result<Vec<_>>>()?;
            let func = contract.function_from_selector(&selector, &tokens)?;
            let is_view = abi_func.state_mutability == StateMutability::Pure
                || abi_func.state_mutability == StateMutability::View;

            if self.mode == ContractCallMode::Encode {
                let encoded = func.calldata();
                Ok(Value::Bytes(encoded[..].to_vec()))
            } else if self.mode == ContractCallMode::Call
                || (self.mode == ContractCallMode::Default && is_view)
            {
                _execute_contract_call(&addr, func, &call_options, env).await
            } else {
                _execute_contract_send(&addr, func, &call_options, env).await
            }
        }
        .boxed()
    }
}

fn _build_transaction<T, P, N>(
    addr: &Address,
    func: &CallBuilder<T, P, alloy::json_abi::Function, N>,
    opts: &CallOptions,
) -> Result<TransactionRequest>
where
    T: Transport + Clone,
    P: Provider<T, N>,
    N: Network,
{
    let data = func.calldata();
    let input = TransactionInput::new(data.clone());

    let mut tx_req = TransactionRequest::default().with_to(*addr).input(input);
    if let Some(value) = opts.value.as_ref() {
        let value = value.as_u256()?;
        tx_req = tx_req.with_value(value);
    }

    Ok(tx_req)
}

async fn _execute_contract_send<T, P, N>(
    addr: &Address,
    func: CallBuilder<T, P, alloy::json_abi::Function, N>,
    opts: &CallOptions,
    env: &Env,
) -> Result<Value>
where
    T: Transport + Clone,
    P: Provider<T, N>,
    N: Network,
{
    opts.validate_send()?;
    let mut tx_req = _build_transaction(addr, &func, opts)?;
    let from_ = env
        .get_default_sender()
        .ok_or(anyhow!("no wallet connected"))?;
    tx_req = tx_req.with_from(from_);

    let provider = env.get_provider();
    let tx = provider.send_transaction(tx_req).await?;
    Ok(Value::Transaction(*tx.tx_hash()))
}

async fn _execute_contract_call<T, P, N>(
    addr: &Address,
    func: CallBuilder<T, P, alloy::json_abi::Function, N>,
    opts: &CallOptions,
    env: &Env,
) -> Result<Value>
where
    T: Transport + Clone,
    P: Provider<T, N>,
    N: Network,
{
    let mut tx_req = _build_transaction(addr, &func, opts)?;
    if let Some(from_) = opts.from {
        tx_req = tx_req.with_from(from_);
    }
    let block = opts.block.unwrap_or(env.block());
    let provider = env.get_provider();
    let return_bytes = provider.call(&tx_req).block(block).await?;
    let result = func.decode_output(return_bytes, true)?;
    let return_values = result
        .into_iter()
        .map(Value::try_from)
        .collect::<Result<Vec<_>>>()?;
    if return_values.len() == 1 {
        Ok(return_values.into_iter().next().unwrap())
    } else {
        Ok(Value::Tuple(return_values))
    }
}
