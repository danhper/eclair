use alloy::{eips::BlockId, rpc::types::BlockTransactionsKind};
use anyhow::{anyhow, Ok, Result};
use futures::{future::BoxFuture, FutureExt};
use lazy_static::lazy_static;

use crate::interpreter::{builtins::FunctionDefinition, Env, Value};

fn get_chain_id<'a>(env: &'a mut Env, _args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move { Ok(env.get_provider().get_chain_id().await?.into()) }.boxed()
}

fn get_base_fee<'a>(env: &'a mut Env, _args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move { Ok(env.get_provider().get_gas_price().await?.into()) }.boxed()
}

fn get_block_number<'a>(env: &'a mut Env, _args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move { Ok(env.get_provider().get_block_number().await?.into()) }.boxed()
}

fn get_timestamp<'a>(env: &'a mut Env, _args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
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
    pub static ref BLOCK_CHAIN_ID: FunctionDefinition = FunctionDefinition {
        name_: "chainid".to_string(),
        property: true,
        valid_args: vec![vec![]],
        execute_fn: get_chain_id,
    };
    pub static ref BLOCK_BASE_FEE: FunctionDefinition = FunctionDefinition {
        name_: "basefee".to_string(),
        property: true,
        valid_args: vec![vec![]],
        execute_fn: get_base_fee,
    };
    pub static ref BLOCK_NUMBER: FunctionDefinition = FunctionDefinition {
        name_: "number".to_string(),
        property: true,
        valid_args: vec![vec![]],
        execute_fn: get_block_number,
    };
    pub static ref BLOCK_TIMESTAMP: FunctionDefinition = FunctionDefinition {
        name_: "timestamp".to_string(),
        property: true,
        valid_args: vec![vec![]],
        execute_fn: get_timestamp,
    };
}
