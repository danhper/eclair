use anyhow::{anyhow, bail, Result};
use futures::{future::BoxFuture, FutureExt};
use lazy_static::lazy_static;

use crate::interpreter::{
    function_definitions::{FunctionDefinition, FunctionParam},
    Env, Type, Value,
};

fn keccak256<'a>(_env: &'a mut Env, args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move {
        let data = match args.first() {
            Some(Value::Bytes(data)) => data,
            _ => bail!("keccak256 function expects bytes as an argument"),
        };
        Ok(Value::FixBytes(alloy::primitives::keccak256(data), 32))
    }
    .boxed()
}

fn get_type<'a>(_env: &'a mut Env, args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move {
        args.first()
            .map(|v| Value::TypeObject(v.get_type()))
            .ok_or(anyhow!("get_type function expects one argument"))
    }
    .boxed()
}

fn mapping_keys<'a>(_env: &'a mut Env, args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move {
        match args.first() {
            Some(Value::Mapping(mapping, key_type, _)) => {
                let keys = mapping.0.keys().cloned().collect();
                Ok(Value::Array(keys, key_type.clone()))
            }
            _ => bail!("mapping_keys function expects a mapping as an argument"),
        }
    }
    .boxed()
}

lazy_static! {
    pub static ref KECCAK256: FunctionDefinition = FunctionDefinition {
        name_: "keccak256".to_string(),
        property: false,
        valid_args: vec![vec![FunctionParam::new("data", Type::Bytes)]],
        execute_fn: keccak256,
    };
    pub static ref GET_TYPE: FunctionDefinition = FunctionDefinition {
        name_: "get_type".to_string(),
        property: false,
        valid_args: vec![vec![FunctionParam::new("value", Type::Any)]],
        execute_fn: get_type,
    };
    pub static ref MAPPING_KEYS: FunctionDefinition = FunctionDefinition {
        name_: "keys".to_string(),
        property: true,
        valid_args: vec![vec![]],
        execute_fn: mapping_keys,
    };
}
