use std::sync::Arc;

use crate::interpreter::{
    functions::{FunctionDef, SyncProperty},
    Env, Type, Value,
};
use alloy::primitives::B256;
use anyhow::{bail, Result};
use lazy_static::lazy_static;

pub fn contract_function_selector(_env: &Env, receiver: &Value) -> Result<Value> {
    let function_abi = match receiver {
        Value::TypeObject(Type::ContractFunction(func)) => func,
        _ => bail!("selector function expects receiver to be an contract function"),
    };

    let mut bytes = [0u8; 32];
    bytes[..4].copy_from_slice(function_abi.selector().as_slice());
    Ok(Value::FixBytes(B256::from(bytes), 4))
}

lazy_static! {
    pub static ref CONTRACT_FUNCTION_SELECTOR: Arc<dyn FunctionDef> =
        SyncProperty::arc("selector", contract_function_selector);
}
