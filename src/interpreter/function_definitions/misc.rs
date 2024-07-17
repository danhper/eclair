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
}
