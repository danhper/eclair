use std::sync::Arc;

use alloy::{eips::BlockId, rpc::types::BlockTransactionsKind};
use anyhow::{anyhow, Ok, Result};
use futures::{future::BoxFuture, FutureExt};
use lazy_static::lazy_static;

use crate::interpreter::{
    functions::{AsyncProperty, FunctionDef},
    Env, Value,
};

fn get_chain_id<'a>(env: &'a Env, _arg: &'a Value) -> BoxFuture<'a, Result<Value>> {
    async move { Ok(env.get_provider().get_chain_id().await?.into()) }.boxed()
}

fn get_base_fee<'a>(env: &'a Env, _arg: &'a Value) -> BoxFuture<'a, Result<Value>> {
    async move { Ok(env.get_provider().get_gas_price().await?.into()) }.boxed()
}

fn get_block_number<'a>(env: &'a Env, _arg: &'a Value) -> BoxFuture<'a, Result<Value>> {
    async move { Ok(env.get_provider().get_block_number().await?.into()) }.boxed()
}

fn get_timestamp<'a>(env: &'a Env, _arg: &'a Value) -> BoxFuture<'a, Result<Value>> {
    async move {
        let latest_block = env
            .get_provider()
            .get_block(BlockId::latest(), BlockTransactionsKind::Hashes)
            .await?
            .ok_or(anyhow!("latest block not found"))?;
        Ok(latest_block.header.timestamp.into())
    }
    .boxed()
}

lazy_static! {
    pub static ref BLOCK_CHAIN_ID: Arc<dyn FunctionDef> =
        AsyncProperty::arc("chainid", get_chain_id);
    pub static ref BLOCK_BASE_FEE: Arc<dyn FunctionDef> =
        AsyncProperty::arc("basefee", get_base_fee);
    pub static ref BLOCK_NUMBER: Arc<dyn FunctionDef> =
        AsyncProperty::arc("number", get_block_number);
    pub static ref BLOCK_TIMESTAMP: Arc<dyn FunctionDef> =
        AsyncProperty::arc("timestamp", get_timestamp);
}
