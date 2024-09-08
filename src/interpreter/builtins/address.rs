use std::sync::Arc;

use alloy::{providers::Provider, transports::BoxFuture};
use anyhow::Result;
use futures::FutureExt;
use lazy_static::lazy_static;

use crate::interpreter::{
    functions::{AsyncProperty, FunctionDef},
    Env, Value,
};

fn get_balance<'a>(env: &'a Env, receiver: &'a Value) -> BoxFuture<'a, Result<Value>> {
    async move {
        Ok(Value::Uint(
            env.get_provider()
                .get_balance(receiver.as_address()?)
                .await?,
            256,
        ))
    }
    .boxed()
}

lazy_static! {
    pub static ref ADDRESS_BALANCE: Arc<dyn FunctionDef> =
        AsyncProperty::arc("balance", get_balance);
}
