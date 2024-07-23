use std::sync::Arc;

use anyhow::{bail, Result};
use futures::{future::BoxFuture, FutureExt};
use lazy_static::lazy_static;

use crate::interpreter::{
    functions::{FunctionDef, FunctionParam},
    types::HashableIndexMap,
    Env, Value,
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

fn concat(args: &[Value]) -> Result<Value> {
    match args.first() {
        Some(Value::Str(s)) => concat_strings(s.clone(), &args[1..]).map(Value::Str),
        Some(Value::Array(arr, t)) => {
            concat_arrays(arr.clone(), &args[1..]).map(|items| Value::Array(items, t.clone()))
        }
        Some(Value::Bytes(b)) => concat_bytes(b.clone(), &args[1..]).map(Value::Bytes),
        _ => bail!("cannot concat {}", args[0]),
    }
}

#[derive(Debug)]
pub struct Concat;

impl FunctionDef for Concat {
    fn name(&self) -> &str {
        "concat"
    }

    fn get_valid_args(&self, receiver: &Option<Value>) -> Vec<Vec<FunctionParam>> {
        receiver.as_ref().map_or(vec![], |r| {
            vec![vec![FunctionParam::new("other", r.get_type().clone())]]
        })
    }

    fn is_property(&self) -> bool {
        false
    }

    fn execute<'a>(
        &'a self,
        _env: &'a mut Env,
        args: &'a [Value],
        _options: &'a HashableIndexMap<String, Value>,
    ) -> BoxFuture<'a, Result<Value>> {
        async { concat(args) }.boxed()
    }
}

lazy_static! {
    pub static ref CONCAT: Arc<dyn FunctionDef> = Arc::new(Concat);
}
