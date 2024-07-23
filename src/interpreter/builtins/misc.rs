use std::sync::Arc;

use anyhow::{anyhow, bail, Result};
use lazy_static::lazy_static;

use crate::interpreter::{
    functions::{FunctionDef, FunctionParam, SyncFunction, SyncProperty},
    Env, Type, Value,
};

fn keccak256(_env: &Env, args: &[Value]) -> Result<Value> {
    let data = match args.first() {
        Some(Value::Bytes(data)) => data,
        _ => bail!("keccak256 function expects bytes as an argument"),
    };
    Ok(Value::FixBytes(alloy::primitives::keccak256(data), 32))
}

fn get_type(_env: &Env, args: &[Value]) -> Result<Value> {
    args.first()
        .map(|v| Value::TypeObject(v.get_type()))
        .ok_or(anyhow!("get_type function expects one argument"))
}

fn mapping_keys(_env: &Env, receiver: &Value) -> Result<Value> {
    match receiver {
        Value::Mapping(mapping, key_type, _) => {
            let keys = mapping.0.keys().cloned().collect();
            Ok(Value::Array(keys, key_type.clone()))
        }
        _ => bail!("mapping_keys function expects a mapping as an argument"),
    }
}

lazy_static! {
    pub static ref KECCAK256: Arc<dyn FunctionDef> = SyncFunction::arc(
        "keccak256",
        keccak256,
        vec![vec![FunctionParam::new("data", Type::Bytes)]]
    );
    pub static ref GET_TYPE: Arc<dyn FunctionDef> = SyncFunction::arc(
        "type",
        get_type,
        vec![vec![FunctionParam::new("value", Type::Any)]]
    );
    pub static ref MAPPING_KEYS: Arc<dyn FunctionDef> = SyncProperty::arc("keys", mapping_keys);
}
