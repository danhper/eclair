use alloy::providers::PendingTransactionBuilder;
use anyhow::{bail, Result};
use futures::{future::BoxFuture, FutureExt};
use lazy_static::lazy_static;

use crate::interpreter::{
    builtins::{types::FunctionDefinitionBuilder, FunctionDefinition, FunctionParam},
    Env, Type, Value,
};

fn wait_for_receipt<'a>(env: &'a mut Env, args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move {
        let tx = match args.first() {
            Some(Value::Transaction(tx)) => *tx,
            _ => bail!("wait_for_receipt function expects a transaction as argument"),
        };
        let provider = env.get_provider();
        let tx = PendingTransactionBuilder::new(provider.root(), tx);
        if args.len() > 1 {
            bail!("get_receipt function expects at most one argument")
        }
        let timeout = args.get(1).map_or(Ok(30), |v| v.as_u64())?;
        let receipt = tx
            .with_required_confirmations(1)
            .with_timeout(Some(std::time::Duration::from_secs(timeout)))
            .get_receipt()
            .await?;
        Ok(Value::TransactionReceipt(receipt.into()))
    }
    .boxed()
}

lazy_static! {
    pub static ref TX_GET_RECEIPT: FunctionDefinition =
        FunctionDefinitionBuilder::new("getReceipt", wait_for_receipt)
            .add_valid_args(&[])
            .add_valid_args(&[FunctionParam::new("timeout", Type::Uint(256))])
            .build();
}
