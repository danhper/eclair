use alloy::{eips::BlockId, rpc::types::BlockTransactionsKind};
use anyhow::{anyhow, Ok, Result};
use futures::{future::BoxFuture, FutureExt};
use lazy_static::lazy_static;

use crate::interpreter::{
    functions::{FunctionDefinition, FunctionDefinitionBuilder},
    Env, Value,
};

fn get_chain_id<'a>(
    _def: &'a FunctionDefinition,
    env: &'a mut Env,
    _args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move { Ok(env.get_provider().get_chain_id().await?.into()) }.boxed()
}

fn get_base_fee<'a>(
    _def: &'a FunctionDefinition,
    env: &'a mut Env,
    _args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move { Ok(env.get_provider().get_gas_price().await?.into()) }.boxed()
}

fn get_block_number<'a>(
    _def: &'a FunctionDefinition,
    env: &'a mut Env,
    _args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move { Ok(env.get_provider().get_block_number().await?.into()) }.boxed()
}

fn get_timestamp<'a>(
    _def: &'a FunctionDefinition,
    env: &'a mut Env,
    _args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
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
    pub static ref BLOCK_CHAIN_ID: FunctionDefinition =
        FunctionDefinitionBuilder::property("chainid", get_chain_id).build();
    pub static ref BLOCK_BASE_FEE: FunctionDefinition =
        FunctionDefinitionBuilder::property("basefee", get_base_fee).build();
    pub static ref BLOCK_NUMBER: FunctionDefinition =
        FunctionDefinitionBuilder::property("number", get_block_number).build();
    pub static ref BLOCK_TIMESTAMP: FunctionDefinition =
        FunctionDefinitionBuilder::property("timestamp", get_timestamp).build();
}
