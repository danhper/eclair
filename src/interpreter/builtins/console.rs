use alloy::transports::BoxFuture;
use anyhow::Result;
use futures::FutureExt;
use lazy_static::lazy_static;

use crate::interpreter::{
    functions::{FunctionDefinition, FunctionDefinitionBuilder},
    Env, Value,
};

fn log<'a>(
    _def: &'a FunctionDefinition,
    _env: &'a mut Env,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        args.iter().skip(1).for_each(|arg| println!("{}", arg));
        Ok(Value::Null)
    }
    .boxed()
}

lazy_static! {
    pub static ref CONSOLE_LOG: FunctionDefinition =
        FunctionDefinitionBuilder::new("log", log).build();
}
