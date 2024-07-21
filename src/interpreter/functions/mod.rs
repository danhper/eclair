mod call;
mod definition;
mod param;
mod user_defined;

pub use call::FunctionCall;
pub use definition::{FunctionDefinition, FunctionDefinitionBuilder};
pub use param::FunctionParam;

use std::fmt::Display;

use alloy::{
    contract::{CallBuilder, ContractInstance, Interface},
    dyn_abi::Specifier,
    json_abi::StateMutability,
    network::{Network, TransactionBuilder},
    primitives::Address,
    providers::Provider,
    rpc::types::{TransactionInput, TransactionRequest},
    transports::Transport,
};
use anyhow::{anyhow, bail, Result};
use solang_parser::pt::Statement;

use super::{types::ContractInfo, Env, StatementResult, Type, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserDefinedFunction {
    pub name: String,
    params: Vec<FunctionParam>,
    body: Statement,
}

impl std::hash::Hash for UserDefinedFunction {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.params.hash(state);
    }
}

impl TryFrom<solang_parser::pt::FunctionDefinition> for UserDefinedFunction {
    type Error = anyhow::Error;

    fn try_from(f: solang_parser::pt::FunctionDefinition) -> Result<Self> {
        let name = f.name.clone().ok_or(anyhow!("require function name"))?.name;
        let stmt = f.body.clone().ok_or(anyhow!("require function body"))?;
        let params = f
            .params
            .iter()
            .map(|(_, p)| {
                p.clone()
                    .ok_or(anyhow!("require param"))
                    .and_then(FunctionParam::try_from)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(UserDefinedFunction {
            name,
            params,
            body: stmt,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum ContractCallMode {
    Default,
    Encode,
    Call,
    Send,
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

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ContractCall {
    info: ContractInfo,
    addr: Address,
    func_name: String,
    mode: ContractCallMode,
    options: CallOptions,
}

impl ContractCall {
    pub fn new(info: ContractInfo, addr: Address, func_name: String) -> Self {
        ContractCall {
            info,
            addr,
            func_name,
            mode: ContractCallMode::Default,
            options: CallOptions::default(),
        }
    }

    pub fn with_options(self, options: CallOptions) -> Self {
        ContractCall { options, ..self }
    }

    pub fn with_mode(self, mode: ContractCallMode) -> Self {
        ContractCall { mode, ..self }
    }
}

#[derive(Debug, Clone, Default, Hash, PartialEq, Eq)]
pub struct CallOptions {
    value: Option<Box<Value>>,
}

impl Display for CallOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(v) = &self.value {
            write!(f, "value: {}", v)
        } else {
            write!(f, "")
        }
    }
}

impl TryFrom<Value> for CallOptions {
    type Error = anyhow::Error;

    fn try_from(value: Value) -> std::result::Result<Self, Self::Error> {
        match value {
            Value::NamedTuple(_, m) => {
                let mut opts = CallOptions::default();
                for (k, v) in m.0.iter() {
                    match k.as_str() {
                        "value" => opts.value = Some(Box::new(v.clone())),
                        _ => bail!("unexpected key {}", k),
                    }
                }
                Ok(opts)
            }
            _ => bail!("expected indexed map but got {}", value),
        }
    }
}

impl TryFrom<StatementResult> for CallOptions {
    type Error = anyhow::Error;

    fn try_from(value: StatementResult) -> std::result::Result<Self, Self::Error> {
        match value {
            StatementResult::Value(v) => v.try_into(),
            _ => bail!("expected indexed map but got {}", value),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Function {
    ContractCall(ContractCall),
    Call(Box<FunctionCall>),
}

impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Function::Call(call) => write!(f, "{}", call),
            Function::ContractCall(ContractCall {
                info: ContractInfo(name, abi),
                addr,
                func_name,
                mode,
                options,
            }) => {
                let arg_types = abi
                    .function(func_name)
                    .map(|f| {
                        f[0].inputs
                            .iter()
                            .map(|t| t.to_string())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let suffix = if mode == &ContractCallMode::Encode {
                    ".encode"
                } else {
                    ""
                };
                write!(f, "{}({}).{}", name, addr, func_name)?;
                let formatted_options = format!("{}", options);
                if !formatted_options.is_empty() {
                    write!(f, "{{{}}}", formatted_options)?;
                }
                write!(f, "({}){}", arg_types.join(","), suffix)
            }
        }
    }
}

impl Function {
    pub fn with_opts(self, opts: CallOptions) -> Self {
        match self {
            Function::ContractCall(call) => Function::ContractCall(call.with_options(opts)),
            v => v,
        }
    }

    pub async fn execute_in_current_scope(&self, args: &[Value], env: &mut Env) -> Result<Value> {
        match self {
            Function::ContractCall(call) => {
                self._execute_contract_interaction(call, args, env).await
            }
            Function::Call(call) => call.execute(env, args).await,
        }
    }

    pub fn is_property(&self) -> bool {
        match self {
            Function::ContractCall(_) => false,
            Function::Call(c) => c.is_property(),
        }
    }

    pub async fn execute(&self, args: &[Value], env: &mut Env) -> Result<Value> {
        env.push_scope();
        let result = self.execute_in_current_scope(args, env).await;
        env.pop_scope();
        result
    }

    async fn _execute_contract_interaction(
        &self,
        call: &ContractCall,
        args: &[Value],
        env: &Env,
    ) -> Result<Value> {
        let ContractInfo(name, abi) = &call.info;
        let funcs = abi.function(&call.func_name).ok_or(anyhow!(
            "function {} not found in {}",
            call.func_name,
            name
        ))?;
        let contract = ContractInstance::new(
            call.addr,
            env.get_provider().root().clone(),
            Interface::new(abi.clone()),
        );
        let mut call_result = Ok(Value::Null);
        for func_abi in funcs.iter() {
            let types = func_abi
                .inputs
                .iter()
                .map(|t| t.resolve().map(Type::from).map_err(|e| anyhow!(e)))
                .collect::<Result<Vec<Type>>>()?;
            match self._unify_types(args, &types) {
                Ok(values) => {
                    let tokens = values
                        .iter()
                        .map(|arg| arg.try_into())
                        .collect::<Result<Vec<_>>>()?;
                    let func = contract.function_from_selector(&func_abi.selector(), &tokens)?;
                    let is_view = func_abi.state_mutability == StateMutability::Pure
                        || func_abi.state_mutability == StateMutability::View;
                    match call.mode {
                        ContractCallMode::Default => {
                            if is_view {
                                call_result = self._execute_contract_call(func).await;
                            } else {
                                call_result = self
                                    ._execute_contract_send(&call.addr, func, &call.options, env)
                                    .await
                            }
                        }
                        ContractCallMode::Encode => {
                            let encoded = func.calldata();
                            call_result = Ok(Value::Bytes(encoded[..].to_vec()));
                        }
                        ContractCallMode::Call => {
                            call_result = self._execute_contract_call(func).await
                        }
                        ContractCallMode::Send => {
                            call_result = self
                                ._execute_contract_send(&call.addr, func, &call.options, env)
                                .await
                        }
                    }
                    break;
                }
                Err(e) => call_result = Err(e),
            }
        }
        call_result
    }

    async fn _execute_contract_send<T, P, N>(
        &self,
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
        let data = func.calldata();
        let input = TransactionInput::new(data.clone());
        let from_ = env
            .get_default_sender()
            .ok_or(anyhow!("no wallet connected"))?;
        let mut tx_req = TransactionRequest::default()
            .with_from(from_)
            .with_to(*addr)
            .input(input);
        if let Some(value) = opts.value.as_ref() {
            let value = value.as_u256()?;
            tx_req = tx_req.with_value(value);
        }

        let provider = env.get_provider();
        let tx = provider.send_transaction(tx_req).await?;
        Ok(Value::Transaction(*tx.tx_hash()))
    }

    async fn _execute_contract_call<T, P, N>(
        &self,
        func: CallBuilder<T, P, alloy::json_abi::Function, N>,
    ) -> Result<Value>
    where
        T: Transport + Clone,
        P: Provider<T, N>,
        N: Network,
    {
        let result = func.call().await?;
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

    fn _unify_types(&self, args: &[Value], types: &[Type]) -> Result<Vec<Value>> {
        if args.len() != types.len() {
            bail!(
                "function {} expects {} arguments, but got {}",
                self,
                types.len(),
                args.len()
            );
        }
        let mut result = Vec::new();
        for (i, (arg, type_)) in args.iter().zip(types).enumerate() {
            match type_.cast(arg) {
                Ok(v) => result.push(v),
                Err(e) => bail!(
                    "expected {} argument {} to be {}, but got {} ({})",
                    self,
                    i,
                    type_,
                    arg.get_type(),
                    e
                ),
            }
        }
        Ok(result)
    }
}
