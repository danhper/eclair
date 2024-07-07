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
use solang_parser::pt::{Expression, Identifier, Parameter, Statement};

use super::{
    builtin_functions::BuiltinFunction, evaluate_statement, types::ContractInfo, Env, Type, Value,
};

#[derive(Debug, Clone)]
pub struct FunctionParam {
    name: String,
    type_: Option<Type>,
}

impl Display for FunctionParam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.type_ {
            Some(t) => write!(f, "{} {}", self.name, t),
            None => write!(f, "{}", self.name),
        }
    }
}

impl TryFrom<Parameter> for FunctionParam {
    type Error = anyhow::Error;

    fn try_from(p: Parameter) -> Result<Self> {
        match (p.name, p.ty) {
            (Some(Identifier { name, .. }), Expression::Type(_, t)) => {
                let type_ = Some(t.try_into()?);
                Ok(FunctionParam { name, type_ })
            }
            (None, Expression::Variable(Identifier { name, .. })) => {
                Ok(FunctionParam { name, type_: None })
            }
            _ => bail!("require param name or type and name"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UserDefinedFunction {
    pub name: String,
    params: Vec<FunctionParam>,
    body: Statement,
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

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone)]
pub enum Function {
    ContractCall(ContractInfo, Address, String, ContractCallMode),
    Builtin(BuiltinFunction),
    UserDefined(UserDefinedFunction),
    FieldAccess(Box<Value>, String),
}

impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Function::ContractCall(ContractInfo(name, abi), addr, func_name, mode) => {
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
                write!(
                    f,
                    "{}({}).{}({}){}",
                    name,
                    addr,
                    func_name,
                    arg_types.join(","),
                    suffix
                )
            }
            Function::FieldAccess(v, n) => write!(f, "{}.{}", v, n),
            Function::Builtin(m) => write!(f, "{}", m),
            Function::UserDefined(func) => {
                let formatted_params = func
                    .params
                    .iter()
                    .map(|p| format!("{}", p))
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "{}({})", func.name, formatted_params)
            }
        }
    }
}

impl Function {
    pub fn with_receiver(receiver: &Value, name: &str) -> Result<Self> {
        let func = match receiver {
            Value::Contract(c, addr) => c.create_call(name, *addr)?,
            v @ Value::NamedTuple(..) => {
                Function::FieldAccess(Box::new(v.clone()), name.to_string())
            }

            Value::Func(Function::ContractCall(contract, addr, method, _mode)) => {
                Function::ContractCall(
                    contract.clone(),
                    *addr,
                    method.to_string(),
                    ContractCallMode::try_from(name)?,
                )
            }

            v => {
                let method = BuiltinFunction::with_receiver(v, name)?;
                Function::Builtin(method)
            }
        };
        Ok(func)
    }

    pub async fn execute_in_current_scope(&self, args: &[Value], env: &mut Env) -> Result<Value> {
        match self {
            Function::ContractCall(contract_info, addr, func_name, mode) => {
                self._execute_contract_interaction(contract_info, addr, func_name, mode, args, env)
                    .await
            }
            Function::FieldAccess(f, v) => f.get_field(v),
            Function::Builtin(m) => m.execute(args, env).await,
            Function::UserDefined(func) => {
                if args.len() != func.params.len() {
                    bail!(
                        "function {} expect {} arguments, but got {}",
                        func.name,
                        func.params.len(),
                        args.len()
                    );
                }
                for (param, arg) in func.params.iter().zip(args.iter()) {
                    if let Some(type_) = param.type_.clone() {
                        if type_ != arg.get_type() {
                            bail!(
                                "function {} expect {} to be {}, but got {}",
                                func.name,
                                param.name,
                                type_,
                                arg.get_type()
                            );
                        }
                    }
                    env.set_var(&param.name, arg.clone());
                }
                evaluate_statement(env, Box::new(func.body.clone()))
                    .await
                    .map(|v| v.unwrap_or(Value::Null))
            }
        }
    }

    pub fn is_property(&self) -> bool {
        match self {
            Function::ContractCall(_, _, _, _) => false,
            Function::FieldAccess(_, _) => true,
            Function::Builtin(m) => m.is_property(),
            Function::UserDefined(_) => false,
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
        contract_info: &ContractInfo,
        addr: &Address,
        func_name: &str,
        mode: &ContractCallMode,
        args: &[Value],
        env: &Env,
    ) -> Result<Value> {
        let ContractInfo(name, abi) = &contract_info;
        let funcs = abi.function(func_name).ok_or(anyhow!(
            "function {} not found in {}",
            func_name,
            name
        ))?;
        let contract = ContractInstance::new(
            *addr,
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
                    match mode {
                        ContractCallMode::Default => {
                            if is_view {
                                call_result = self._execute_contract_call(func).await;
                            } else {
                                call_result = self._execute_contract_send(addr, func, env).await
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
                            call_result = self._execute_contract_send(addr, func, env).await
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
        let tx_req = TransactionRequest::default()
            .with_from(from_)
            .with_to(*addr)
            .input(input);
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
