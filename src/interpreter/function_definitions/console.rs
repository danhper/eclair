use alloy::transports::BoxFuture;
use anyhow::Result;
use futures::FutureExt;
use lazy_static::lazy_static;

use crate::interpreter::{function_definitions::FunctionDefinition, Env, Value};

fn log<'a>(_env: &'a mut Env, args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move {
        args.iter().for_each(|arg| println!("{}", arg));
        Ok(Value::Null)
    }
    .boxed()
}

lazy_static! {
    pub static ref CONSOLE_LOG: FunctionDefinition = FunctionDefinition {
        name_: "log".to_string(),
        property: false,
        valid_args: vec![],
        execute_fn: log,
    };
}
