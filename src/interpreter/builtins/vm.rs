use std::sync::Arc;

use crate::interpreter::{
    functions::{AsyncMethod, AsyncProperty, FunctionDef, FunctionParam, SyncMethod},
    Env, Type, Value,
};
use alloy::{
    primitives::ruint::UintTryTo,
    providers::{ext::AnvilApi, Provider},
};
use anyhow::{bail, Result};
use futures::{future::BoxFuture, FutureExt};
use lazy_static::lazy_static;

fn impersonate<'a>(
    env: &'a mut Env,
    _receiver: &'a Value,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        let address = match args {
            [Value::Addr(address)] => *address,
            _ => bail!("impersonate: invalid arguments"),
        };
        env.impersonate(address).await?;
        Ok(Value::Addr(address))
    }
    .boxed()
}

fn stop_impersonate<'a>(
    env: &'a mut Env,
    _receiver: &'a Value,
    _args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        env.stop_impersonate().await?;
        Ok(Value::Null)
    }
    .boxed()
}

fn rpc(env: &mut Env, _receiver: &Value, args: &[Value]) -> Result<Value> {
    match args {
        [] => Ok(Value::Str(env.get_rpc_url())),
        [url] => {
            env.set_provider_url(&url.as_string()?)?;
            Ok(Value::Null)
        }
        _ => bail!("rpc: invalid arguments"),
    }
}

fn fork<'a>(
    env: &'a mut Env,
    _receiver: &'a Value,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        let url = match args {
            [Value::Str(url)] => url.clone(),
            [] => env.get_rpc_url(),
            _ => bail!("fork: invalid arguments"),
        };
        env.fork(&url, None).await?;
        Ok(Value::Str(env.get_rpc_url()))
    }
    .boxed()
}

fn set_balance<'a>(
    env: &'a mut Env,
    _receiver: &'a Value,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        let (address, balance) = match args {
            [Value::Addr(address), Value::Uint(b, 256)] => (*address, *b),
            _ => bail!("setBalance: invalid arguments"),
        };
        env.get_provider()
            .anvil_set_balance(address, balance)
            .await?;
        Ok(Value::Null)
    }
    .boxed()
}

fn is_connected<'a>(env: &'a Env, _receiver: &'a Value) -> BoxFuture<'a, Result<Value>> {
    async move {
        let res = env.get_provider().root().get_chain_id().await.is_ok();
        Ok(Value::Bool(res))
    }
    .boxed()
}

fn skip<'a>(
    env: &'a mut Env,
    _receiver: &'a Value,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        let time = match args {
            [Value::Uint(time, 256)] => time.uint_try_to()?,
            _ => bail!("skip: invalid arguments"),
        };
        env.get_provider().anvil_increase_time(time).await?;
        Ok(Value::Null)
    }
    .boxed()
}

fn mine<'a>(
    env: &'a mut Env,
    _receiver: &'a Value,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        let num_of_blocks = match args {
            [] => None,
            [Value::Uint(num, 256)] => Some(num.uint_try_to()?),
            _ => bail!("skip: invalid arguments"),
        };
        env.get_provider().anvil_mine(num_of_blocks, None).await?;
        Ok(Value::Null)
    }
    .boxed()
}

fn get_env_var(_env: &mut Env, _receiver: &Value, args: &[Value]) -> Result<Value> {
    let key = match args {
        [Value::Str(key)] => key.clone(),
        _ => bail!("getEnvVar: invalid arguments"),
    };
    Ok(Value::Str(std::env::var(&key)?))
}

fn block(env: &mut Env, _receiver: &Value, args: &[Value]) -> Result<Value> {
    match args {
        [] => Ok(env.block().into()),
        [value] => {
            env.set_block(value.as_block_id()?);
            Ok(Value::Null)
        }
        _ => bail!("block: invalid arguments"),
    }
}

lazy_static! {
    pub static ref VM_START_PRANK: Arc<dyn FunctionDef> = AsyncMethod::arc(
        "startPrank",
        impersonate,
        vec![vec![FunctionParam::new("adddress", Type::Address)]]
    );
    pub static ref VM_STOP_PRANK: Arc<dyn FunctionDef> =
        AsyncMethod::arc("stopPrank", stop_impersonate, vec![vec![]]);
    pub static ref VM_RPC: Arc<dyn FunctionDef> = SyncMethod::arc(
        "rpc",
        rpc,
        vec![vec![], vec![FunctionParam::new("url", Type::String)]]
    );
    pub static ref VM_FORK: Arc<dyn FunctionDef> = AsyncMethod::arc(
        "fork",
        fork,
        vec![vec![], vec![FunctionParam::new("url", Type::String)]]
    );
    pub static ref VM_DEAL: Arc<dyn FunctionDef> = AsyncMethod::arc(
        "deal",
        set_balance,
        vec![vec![
            FunctionParam::new("adddress", Type::Address),
            FunctionParam::new("balance", Type::Uint(256))
        ]]
    );
    pub static ref VM_SKIP: Arc<dyn FunctionDef> = AsyncMethod::arc(
        "skip",
        skip,
        vec![vec![FunctionParam::new("time", Type::Uint(256))]]
    );
    pub static ref VM_MINE: Arc<dyn FunctionDef> = AsyncMethod::arc(
        "mine",
        mine,
        vec![vec![], vec![FunctionParam::new("blocks", Type::Uint(256))]]
    );
    pub static ref VM_BLOCK: Arc<dyn FunctionDef> = SyncMethod::arc(
        "block",
        block,
        vec![
            vec![],
            vec![FunctionParam::new("block", Type::Uint(256))],
            vec![FunctionParam::new("block", Type::String)],
            vec![FunctionParam::new("block", Type::FixBytes(32))],
        ]
    );
    pub static ref VM_IS_CONNECTED: Arc<dyn FunctionDef> =
        AsyncProperty::arc("connected", is_connected);
    pub static ref VM_ENV: Arc<dyn FunctionDef> = SyncMethod::arc(
        "getEnv",
        get_env_var,
        vec![vec![FunctionParam::new("key", Type::String)]]
    );
}
