use std::sync::Arc;

use alloy::{
    network::TransactionBuilder, providers::Provider, rpc::types::TransactionRequest,
    transports::BoxFuture,
};
use anyhow::{anyhow, Result};
use futures::FutureExt;
use lazy_static::lazy_static;

use crate::interpreter::{
    functions::{AsyncMethod, AsyncProperty, FunctionDef, FunctionParam},
    Env, Type, Value,
};

fn get_balance<'a>(env: &'a Env, receiver: &'a Value) -> BoxFuture<'a, Result<Value>> {
    async move {
        Ok(Value::Uint(
            env.get_provider()
                .get_balance(receiver.as_address()?)
                .block_id(env.block())
                .await?,
            256,
        ))
    }
    .boxed()
}

fn transfer<'a>(
    env: &'a mut Env,
    receiver: &'a Value,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        let provider = env.get_provider();
        let value = args
            .first()
            .ok_or(anyhow!("Missing value"))
            .and_then(|v| v.as_u256())?;
        let addr = receiver.as_address()?;
        let tx_req = TransactionRequest::default().with_to(addr).value(value);
        let tx = provider.send_transaction(tx_req).await?;
        Ok(Value::Transaction(*tx.tx_hash()))
    }
    .boxed()
}

lazy_static! {
    pub static ref ADDRESS_BALANCE: Arc<dyn FunctionDef> =
        AsyncProperty::arc("balance", get_balance);
    pub static ref ADDRESS_TRANSFER: Arc<dyn FunctionDef> = AsyncMethod::arc(
        "transfer",
        transfer,
        vec![vec![FunctionParam::new("amount", Type::Uint(256))]]
    );
}
