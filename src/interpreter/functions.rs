use std::fmt::Display;

use alloy::{
    contract::{ContractInstance, Interface},
    providers::RootProvider,
    transports::http::{Client, Http},
};
use anyhow::{bail, Result};

use super::{types::Type, value::ContractInfo, Env, Value};

#[derive(Debug, Clone)]
pub enum Function {
    Cast(Type),
    ContractCall(ContractInfo, String),
}

impl Display for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Function::Cast(t) => write!(f, "{}", t),
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
        }
    }
}

impl Function {
    pub async fn execute(
        &self,
        args: &[Value],
        _env: &mut Env,
        provider: &RootProvider<Http<Client>>,
    ) -> Result<Value> {
        match self {
            Function::Cast(type_) => self._execute_cast(type_, args).await,
            Function::ContractCall(contract_info, func_name) => {
                self._execute_contract_call(contract_info, func_name, args, provider)
                    .await
            }
        }
    }

    async fn _execute_cast(&self, type_: &Type, args: &[Value]) -> Result<Value> {
        match (type_, args) {
            (Type::Contract(name, abi), [Value::Addr(addr)]) => Ok(Value::Contract(ContractInfo(
                name.clone(),
                *addr,
                abi.clone(),
            ))),
            _ => bail!("Invalid arguments for contract cast"),
        }
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
            bail!(
                "Multiple return values not supported yet: {:?}",
                return_values
            );
        }
    }
}
