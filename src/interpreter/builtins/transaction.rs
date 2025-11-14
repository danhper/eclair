use std::sync::Arc;

use alloy::{
    consensus::{transaction::Recovered, EthereumTxEnvelope, Transaction, TxEip4844Variant},
    providers::{PendingTransactionBuilder, Provider},
};
use anyhow::{bail, Result};
use futures::{future::BoxFuture, FutureExt};
use lazy_static::lazy_static;

use crate::interpreter::{
    functions::{AsyncMethod, AsyncProperty, FunctionDef, FunctionParam},
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
        let tx = PendingTransactionBuilder::new(provider.root().clone(), tx);
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

async fn get_tx(
    env: &Env,
    value: &Value,
) -> Result<Recovered<EthereumTxEnvelope<TxEip4844Variant>>> {
    let local_tx = match value {
        Value::Transaction(tx) => *tx,
        _ => bail!("expected a transaction as argument"),
    };
    let provider = env.get_provider();
    let tx = provider
        .get_transaction_by_hash(local_tx)
        .await?
        .ok_or(anyhow::anyhow!("Transaction not found"))?;

    Ok(tx.inner)
}

fn get_input_data<'a>(env: &'a Env, receiver: &'a Value) -> BoxFuture<'a, Result<Value>> {
    async move {
        let tx = get_tx(env, receiver).await?;
        Ok(Value::Bytes(tx.input().to_vec()))
    }
    .boxed()
}

fn get_from<'a>(env: &'a Env, receiver: &'a Value) -> BoxFuture<'a, Result<Value>> {
    async move {
        let tx = get_tx(env, receiver).await?;
        Ok(Value::Addr(tx.signer()))
    }
    .boxed()
}

fn get_to<'a>(env: &'a Env, receiver: &'a Value) -> BoxFuture<'a, Result<Value>> {
    async move {
        let tx = get_tx(env, receiver).await?;
        if let Some(to) = tx.to() {
            Ok(Value::Addr(to))
        } else {
            Ok(Value::Null)
        }
    }
    .boxed()
}

lazy_static! {
    pub static ref TX_GET_RECEIPT: Arc<dyn FunctionDef> = AsyncMethod::arc(
        "getReceipt",
        wait_for_receipt,
        vec![vec![], vec![FunctionParam::new("timeout", Type::Uint(256))]]
    );
    pub static ref TX_GET_INPUT_DATA: Arc<dyn FunctionDef> =
        AsyncProperty::arc("input", get_input_data);
    pub static ref TX_GET_FROM: Arc<dyn FunctionDef> = AsyncProperty::arc("from", get_from);
    pub static ref TX_GET_TO: Arc<dyn FunctionDef> = AsyncProperty::arc("to", get_to);
}
