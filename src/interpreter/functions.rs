use alloy::{
    contract::{ContractInstance, Interface},
    providers::RootProvider,
    transports::http::{Client, Http},
};
use anyhow::{anyhow, bail, Result};

use super::{Env, Value};

pub(crate) enum CallType {
    ContractCast(String),
    ContractCall(String, String),
    RegularCall(String),
    ModuleCall(String, String),
}

impl CallType {
    pub async fn execute(
        &self,
        env: &mut Env,
        args: &[Value],
        provider: &RootProvider<Http<Client>>,
    ) -> Result<Value> {
        match self {
            CallType::ContractCast(id) => match args {
                [Value::Addr(addr)] => {
                    let type_ = env
                        .get_type(id)
                        .ok_or_else(|| anyhow!("Type not found: {}", id))?;
                    Ok(Value::Contract(id.clone(), *addr, type_.clone()))
                }
                _ => bail!("Invalid arguments for contract cast"),
            },
            CallType::ContractCall(type_, func_name) => {
                let var = env
                    .get_var(type_)
                    .ok_or_else(|| anyhow!("Variable not found: {}", type_))?;
                let contract = match var {
                    Value::Contract(_, addr, abi) => {
                        ContractInstance::new(*addr, provider.clone(), Interface::new(abi.clone()))
                    }
                    _ => bail!("Invalid contract"),
                };
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

            CallType::RegularCall(_func_name) => {
                bail!("Regular call not supported");
            }
            CallType::ModuleCall(_module_name, _func_name) => {
                bail!("Module call not supported");
            }
        }
    }
}
