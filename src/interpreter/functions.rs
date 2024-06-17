use anyhow::{anyhow, bail, Result};
use ethers::abi::Tokenizable;
use ethers::contract::Contract;

use super::{Env, Value};

pub(crate) enum CallType {
    ContractCast(String),
    RegularCall(String),
    ContractCall(String, String),
    ModuleCall(String, String),
}

impl CallType {
    pub async fn execute(&self, env: &mut Env, args: &[Value]) -> Result<Value> {
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
            CallType::RegularCall(_) => {
                bail!("Regular call not supported");
            }
            CallType::ContractCall(type_, func_name) => {
                let var = env
                    .get_var(type_)
                    .ok_or_else(|| anyhow!("Variable not found: {}", type_))?;
                let contract = match var {
                    Value::Contract(_, addr, abi) => {
                        Contract::new(*addr, abi.clone(), env.provider.clone())
                    }
                    _ => bail!("Invalid contract"),
                };
                let tokens = args
                    .iter()
                    .map(|arg| arg.clone().into_token())
                    .collect::<Vec<_>>();
                let result = contract.method(&func_name, tokens)?.call().await?;
                // Contract::new(address, abi, client)
                bail!("Contract call not supported");
            }
            CallType::ModuleCall(_, _) => {
                bail!("Module call not supported");
            }
        }
    }
}
