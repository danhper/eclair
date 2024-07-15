use alloy::transports::BoxFuture;
use anyhow::Result;
use futures::FutureExt;
use lazy_static::lazy_static;

use crate::interpreter::{function_definitions::FunctionDefinition, Env, Value};

fn balance<'a>(env: &'a mut Env, args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move {
        Ok(Value::Uint(
            env.get_provider()
                .get_balance(args[0].as_address()?)
                .await?,
            256,
        ))
    }
    .boxed()
}

lazy_static! {
    pub static ref ADDRESS_BALANCE: FunctionDefinition = FunctionDefinition {
        name_: "balance".to_string(),
        property: true,
        valid_args: vec![vec![]],
        execute_fn: balance,
    };
}
