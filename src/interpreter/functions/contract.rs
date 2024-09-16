use std::{hash::Hash, sync::Arc};

use alloy::{
    contract::{CallBuilder, ContractInstance, Interface},
    eips::{BlockId, BlockNumberOrTag},
    json_abi::StateMutability,
    network::{Network, TransactionBuilder},
    primitives::{keccak256, Address, Bytes, FixedBytes, U256},
    providers::{ext::DebugApi, Provider},
    rpc::types::{
        trace::geth::{self, GethDebugTracingCallOptions},
        BlockTransactionsKind, TransactionInput, TransactionRequest,
    },
    transports::Transport,
};
use anyhow::{anyhow, bail, Result};
use futures::{future::BoxFuture, FutureExt};
use itertools::Itertools;

use crate::interpreter::{
    tracing::format_call_frame, types::HashableIndexMap, utils::decode_error, ContractInfo, Env,
    Type, Value,
};

use super::{Function, FunctionDef, FunctionParam};

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum ContractCallMode {
    Default,
    Encode,
    Call,
    TraceCall,
    Send,
}

impl std::fmt::Display for ContractCallMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContractCallMode::Default => write!(f, "default"),
            ContractCallMode::Encode => write!(f, "encode"),
            ContractCallMode::Call => write!(f, "call"),
            ContractCallMode::TraceCall => write!(f, "trace_call"),
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
            "trace_call" => Ok(ContractCallMode::TraceCall),
            "send" => Ok(ContractCallMode::Send),
            _ => bail!("{} does not exist for contract call", s),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CallOptions {
    value: Option<U256>,
    block: Option<BlockId>,
    from: Option<Address>,
    gas_limit: Option<u128>,
    max_fee: Option<u128>,
    priority_fee: Option<u128>,
    gas_price: Option<u128>,
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

    pub fn validate_call(&self) -> Result<()> {
        if self.max_fee.is_some() {
            bail!("maxFee is only available for sends");
        } else if self.priority_fee.is_some() {
            bail!("priorityFee is only available for sends");
        } else if self.gas_price.is_some() {
            bail!("gasPrice is only available for sends");
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
                "value" => opts.value = Some(v.as_u256()?),
                "block" => opts.block = Some(v.as_block_id()?),
                "from" => opts.from = Some(v.as_address()?),
                "gasLimit" => opts.gas_limit = Some(v.as_u128()?),
                "gasPrice" => opts.gas_price = Some(v.as_u128()?),
                "maxFee" => opts.max_fee = Some(v.as_u128()?),
                "priorityFee" => opts.priority_fee = Some(v.as_u128()?),
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
            } else if self.mode == ContractCallMode::TraceCall {
                _execute_contract_trace_call(&addr, func, &call_options, env).await
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
        tx_req = tx_req.with_value(*value);
    }
    if let Some(gas) = opts.gas_limit.as_ref() {
        tx_req = tx_req.with_gas_limit(*gas);
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
    if let Some(gas_price) = opts.gas_price.as_ref() {
        tx_req = tx_req.with_gas_price(*gas_price);
    }
    if let Some(max_fee) = opts.max_fee.as_ref() {
        tx_req = tx_req.with_max_fee_per_gas(*max_fee);
    }
    if let Some(priority_fee) = opts.priority_fee.as_ref() {
        tx_req = tx_req.with_max_priority_fee_per_gas(*priority_fee);
    }

    let provider = env.get_provider();
    let tx = provider.send_transaction(tx_req).await?;
    Ok(Value::Transaction(*tx.tx_hash()))
}

fn _decode_output<T, P, N>(
    return_bytes: Bytes,
    func: CallBuilder<T, P, alloy::json_abi::Function, N>,
) -> Result<Value>
where
    T: Transport + Clone,
    P: Provider<T, N>,
    N: Network,
{
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
    opts.validate_call()?;
    let mut tx_req = _build_transaction(addr, &func, opts)?;
    if let Some(from_) = opts.from {
        tx_req = tx_req.with_from(from_);
    }
    let block = opts.block.unwrap_or(env.block());
    let provider = env.get_provider();
    let return_bytes = provider.call(&tx_req).block(block).await?;
    _decode_output(return_bytes, func)
}

async fn _execute_contract_trace_call<T, P, N>(
    addr: &Address,
    func: CallBuilder<T, P, alloy::json_abi::Function, N>,
    opts: &CallOptions,
    env: &mut Env,
) -> Result<Value>
where
    T: Transport + Clone,
    P: Provider<T, N>,
    N: Network,
{
    let data = func.calldata();
    let input = TransactionInput::new(data.clone());
    let mut tx_req = TransactionRequest::default().with_to(*addr).input(input);

    if let Some(from_) = opts.from {
        tx_req = tx_req.with_from(from_);
    } else if let Some(acc) = env.get_default_sender() {
        tx_req = tx_req.with_from(acc);
    }

    let (provider, previous_url) = if env.is_fork() {
        (env.get_provider(), None)
    } else {
        let url = env.get_rpc_url();
        env.fork(url.as_str())?;
        (env.get_provider(), Some(url))
    };

    let mut options = GethDebugTracingCallOptions::default();
    let mut tracing_options = options.tracing_options.clone();
    tracing_options = tracing_options.with_tracer(geth::GethDebugTracerType::BuiltInTracer(
        geth::GethDebugBuiltInTracerType::CallTracer,
    ));
    options = options.with_tracing_options(tracing_options);
    // options.with_tracing_options(options)
    let block_tag = env.block();
    let block = provider
        .get_block(block_tag, BlockTransactionsKind::Hashes)
        .await?
        .ok_or(anyhow!("could not get block {:?}", block_tag))?;
    let block_num =
        BlockNumberOrTag::Number(block.header.number.ok_or(anyhow!("no block number"))?);

    let maybe_tx = provider.debug_trace_call(tx_req, block_num, options).await;
    if let Some(url) = previous_url {
        env.set_provider_url(url.as_str())?;
    }
    let call_frame = maybe_tx?.try_into_call_frame()?;

    println!("{}", format_call_frame(env, &call_frame));

    if let Some(err) = call_frame.error {
        if let Some(output) = call_frame.output {
            if let Ok(err_val) = decode_error(env, &output) {
                bail!("revert: {}", err_val);
            } else {
                bail!("revert: {}", output);
            }
        }
        bail!("revert: {}", err);
    } else if let Some(output) = call_frame.output {
        _decode_output(output, func)
    } else {
        Ok(Value::Null)
    }
}
