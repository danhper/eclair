use anyhow::{bail, Result};
use futures::{future::BoxFuture, FutureExt};
use lazy_static::lazy_static;

use crate::interpreter::{
    functions::{FunctionDefinition, FunctionDefinitionBuilder, FunctionParam},
    Env, Type, Value,
};

fn concat_strings(string: String, args: &[Value]) -> Result<String> {
    if let Some(Value::Str(s)) = args.first() {
        Ok(format!("{}{}", string, s))
    } else {
        bail!("cannot concat {} with {:?}", string, args)
    }
}

fn concat_arrays(arr: Vec<Value>, args: &[Value]) -> Result<Vec<Value>> {
    if let Some(Value::Array(other, _)) = args.first() {
        let mut new_arr = arr.clone();
        new_arr.extend(other.clone());
        Ok(new_arr)
    } else {
        bail!("cannot concat {:?} with {:?}", arr, args)
    }
}

fn concat_bytes(bytes: Vec<u8>, args: &[Value]) -> Result<Vec<u8>> {
    if let Some(Value::Bytes(other)) = args.first() {
        let mut new_bytes = bytes.clone();
        new_bytes.extend(other.clone());
        Ok(new_bytes)
    } else {
        bail!("cannot concat {:?} with {:?}", bytes, args)
    }
}

fn concat(_env: &mut Env, args: &[Value]) -> Result<Value> {
    match args.first() {
        Some(Value::Str(s)) => concat_strings(s.clone(), &args[1..]).map(Value::Str),
        Some(Value::Array(arr, t)) => {
            concat_arrays(arr.clone(), &args[1..]).map(|items| Value::Array(items, t.clone()))
        }
        Some(Value::Bytes(b)) => concat_bytes(b.clone(), &args[1..]).map(Value::Bytes),
        _ => bail!("cannot concat {}", args[0]),
    }
}

fn concat_async<'a>(
    _def: &'a FunctionDefinition,
    _env: &'a mut Env,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async { concat(_env, args) }.boxed()
}

lazy_static! {
    pub static ref CONCAT_STRING: FunctionDefinition =
        FunctionDefinitionBuilder::new("concat", concat_async)
            .add_valid_args(&[FunctionParam::new("other", Type::String)])
            .build();
    pub static ref CONCAT_BYTES: FunctionDefinition =
        FunctionDefinitionBuilder::new("concat", concat_async)
            .add_valid_args(&[FunctionParam::new("other", Type::Bytes)])
            .build();
    pub static ref CONCAT_ARRAY: FunctionDefinition =
        FunctionDefinitionBuilder::new("concat", concat_async)
            .add_valid_args(&[FunctionParam::new(
                "other",
                Type::Array(Box::new(Type::Any))
            )])
            .build();
}
