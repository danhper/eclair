use alloy::{eips::BlockId, rpc::types::BlockTransactionsKind};
use anyhow::{anyhow, bail, Result};

use super::{Env, Value};

#[derive(Debug, PartialEq, Clone, Hash, Eq)]
pub enum BlockFunction {
    ChainId,
    BaseFee,
    Number,
    Timestamp,
}

impl std::fmt::Display for BlockFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockFunction::ChainId => write!(f, "chainid"),
            BlockFunction::BaseFee => write!(f, "basefee"),
            BlockFunction::Number => write!(f, "number"),
            BlockFunction::Timestamp => write!(f, "timestamp"),
        }
    }
}

impl BlockFunction {
    pub fn from_name(s: &str) -> Result<Self> {
        match s {
            "chainid" => Ok(BlockFunction::ChainId),
            "basefee" => Ok(BlockFunction::BaseFee),
            "number" => Ok(BlockFunction::Number),
            "timestamp" => Ok(BlockFunction::Timestamp),
            _ => bail!("unknown block function: {}", s),
        }
    }

    pub fn all() -> Vec<String> {
        ["chainid", "basefee", "number", "timestamp"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    pub async fn execute(&self, args: &[Value], env: &mut Env) -> Result<Value> {
        if !args.is_empty() {
            bail!("block.{} does not take arguments", self);
        }
        match self {
            BlockFunction::ChainId => Ok(env.get_provider().get_chain_id().await?.into()),
            BlockFunction::BaseFee => Ok(env.get_provider().get_gas_price().await?.into()),
            BlockFunction::Number => Ok(env.get_provider().get_block_number().await?.into()),
            BlockFunction::Timestamp => {
                let latest_block = env
                    .get_provider()
                    .get_block(BlockId::latest(), BlockTransactionsKind::Hashes)
                    .await?
                    .ok_or(anyhow!("latest block not found"))?;
                Ok(latest_block.header.timestamp.into())
            }
        }
    }
}
