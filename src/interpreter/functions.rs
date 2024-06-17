use anyhow::{anyhow, bail, Result};

use super::{Env, Value};

pub(crate) enum CallType {
    ContractCast(String),
    RegularCall(String),
    ContractCall(String, String),
    ModuleCall(String, String),
}

impl CallType {
    pub fn execute(&self, env: &mut Env, args: &[Value]) -> Result<Value> {
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
            CallType::ContractCall(_, _) => {
                bail!("Contract call not supported");
            }
            CallType::ModuleCall(_, _) => {
                bail!("Module call not supported");
            }
        }
    }
}
