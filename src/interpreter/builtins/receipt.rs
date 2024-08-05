use std::sync::Arc;

use alloy::providers::PendingTransactionBuilder;
use anyhow::{bail, Result};
use futures::{future::BoxFuture, FutureExt};
use lazy_static::lazy_static;

use crate::interpreter::{
    functions::{AsyncMethod, FunctionDef, FunctionParam},
    utils::receipt_to_value,
    Env, Type, Value,
};

fn wait_for_receipt<'a>(
    env: &'a mut Env,
    receiver: &'a Value,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        let tx = match receiver {
            Value::Transaction(tx) => *tx,
            _ => bail!("wait_for_receipt function expects a transaction as argument"),
        };
        let provider = env.get_provider();
        let tx = PendingTransactionBuilder::new(provider.root(), tx);
        if args.len() > 1 {
            bail!("get_receipt function expects at most one argument")
        }
        let timeout = args.first().map_or(Ok(30), |v| v.as_u64())?;
        let receipt = tx
            .with_required_confirmations(1)
            .with_timeout(Some(std::time::Duration::from_secs(timeout)))
            .get_receipt()
            .await?;
        receipt_to_value(env, receipt)
    }
    .boxed()
}

lazy_static! {
    pub static ref TX_GET_RECEIPT: Arc<dyn FunctionDef> = AsyncMethod::arc(
        "getReceipt",
        wait_for_receipt,
        vec![vec![], vec![FunctionParam::new("timeout", Type::Uint(256))]]
    );
}
