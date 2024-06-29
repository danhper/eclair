use std::fmt::Display;

use alloy::{
    contract::{ContractInstance, Interface},
    providers::RootProvider,
    transports::http::{Client, Http},
};
use anyhow::{anyhow, bail, Result};
use solang_parser::pt::{Expression, Identifier, Parameter, Statement};

use super::{
    builtin_functions::BuiltinFunction, evaluate_statement, value::ContractInfo, Env, Type, Value,
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

#[derive(Debug, Clone)]
pub enum Function {
    ContractCall(ContractInfo, String),
    Builtin(BuiltinFunction),
    UserDefined(UserDefinedFunction),
}

impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Function::ContractCall(ContractInfo(name, addr, abi), func_name) => {
                let arg_types = abi
                    .function(func_name)
                    .map(|f| {
                        f[0].inputs
                            .iter()
                            .map(|t| t.to_string())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                write!(
                    f,
                    "{}({}).{}({})",
                    name,
                    addr,
                    func_name,
                    arg_types.join(",")
                )
            }
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
    pub async fn execute_in_current_scope(&self, args: &[Value], env: &mut Env) -> Result<Value> {
        match self {
            Function::ContractCall(contract_info, func_name) => {
                self._execute_contract_call(contract_info, func_name, args, &env.get_provider())
                    .await
            }
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

    pub async fn execute(&self, args: &[Value], env: &mut Env) -> Result<Value> {
        env.push_scope();
        let result = self.execute_in_current_scope(args, env).await;
        env.pop_scope();
        result
    }

    async fn _execute_contract_call(
        &self,
        contract_info: &ContractInfo,
        func_name: &str,
        args: &[Value],
        provider: &RootProvider<Http<Client>>,
    ) -> Result<Value> {
        let ContractInfo(_name, addr, abi) = &contract_info;
        let contract = ContractInstance::new(*addr, provider.clone(), Interface::new(abi.clone()));
        let tokens = args
            .iter()
            .map(|arg| arg.try_into())
            .collect::<Result<Vec<_>>>()?;
        let result = contract.function(func_name, &tokens)?.call().await?;
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
}
