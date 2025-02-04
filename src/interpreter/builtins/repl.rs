use std::{process::Command, sync::Arc};

use alloy::providers::Provider;
use anyhow::{anyhow, bail, Ok, Result};
use futures::{future::BoxFuture, FutureExt};
use lazy_static::lazy_static;

use crate::{
    interpreter::{
        functions::{
            AsyncMethod, AsyncProperty, FunctionDef, FunctionParam, SyncMethod, SyncProperty,
        },
        Env, Type, Value,
    },
    loaders,
};

fn list_vars(env: &Env, _receiver: &Value) -> Result<Value> {
    let mut vars = env.list_vars();
    vars.sort();
    for k in vars.iter() {
        println!("{}: {}", k, env.get_var(k).unwrap());
    }
    Ok(Value::Null)
}

fn list_types(env: &Env, _receiver: &Value) -> Result<Value> {
    let mut types = env.list_types();
    types.sort();
    for k in types.iter() {
        println!("{}", k);
    }
    Ok(Value::Null)
}

fn is_connected<'a>(env: &'a Env, _receiver: &'a Value) -> BoxFuture<'a, Result<Value>> {
    async move {
        let res = env.get_provider().root().get_chain_id().await.is_ok();
        Ok(Value::Bool(res))
    }
    .boxed()
}

fn debug(env: &mut Env, _receiver: &Value, args: &[Value]) -> Result<Value> {
    match args {
        [] => Ok(Value::Bool(env.is_debug())),
        [Value::Bool(b)] => {
            env.set_debug(*b);
            Ok(Value::Null)
        }
        _ => bail!("debug: invalid arguments"),
    }
}

fn exec(_env: &mut Env, _receiver: &Value, args: &[Value]) -> Result<Value> {
    let cmd = args
        .first()
        .ok_or(anyhow!("exec: missing command"))?
        .as_string()?;

    let splitted = cmd.split_whitespace().collect::<Vec<_>>();
    let mut cmd = Command::new(splitted[0]).args(&splitted[1..]).spawn()?;
    let res = cmd.wait()?;
    let code = res.code().ok_or(anyhow!("exec: command failed"))?;
    Ok(code.into())
}

fn load_abi(env: &mut Env, _receiver: &Value, args: &[Value]) -> Result<Value> {
    let (name, filepath, key) = match args {
        [Value::Str(name), Value::Str(filepath)] => (name, filepath, None),
        [Value::Str(name), Value::Str(filepath), Value::Str(key)] => {
            (name, filepath, Some(key.as_str()))
        }
        _ => bail!("loadAbi: invalid arguments"),
    };
    let abi = loaders::file::load_abi(filepath, key)?;
    env.add_contract(name, abi);
    Ok(Value::Null)
}

fn fetch_abi<'a>(
    env: &'a mut Env,
    _receiver: &'a Value,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        match args {
            [Value::Str(name), Value::Addr(address)] => {
                let chain_id = env.get_chain_id().await?;
                let etherscan_config = env.config.get_etherscan_config(chain_id)?;
                let abi =
                    loaders::etherscan::load_abi(etherscan_config, &address.to_string()).await?;
                let contract_info = env.add_contract(name, abi);
                Ok(Value::Contract(contract_info, *address))
            }
            _ => bail!("fetchAbi: invalid arguments"),
        }
    }
    .boxed()
}

lazy_static! {
    pub static ref REPL_LIST_VARS: Arc<dyn FunctionDef> = SyncProperty::arc("vars", list_vars);
    pub static ref REPL_LIST_TYPES: Arc<dyn FunctionDef> = SyncProperty::arc("types", list_types);
    pub static ref REPL_IS_CONNECTED: Arc<dyn FunctionDef> =
        AsyncProperty::arc("connected", is_connected);
    pub static ref REPL_DEBUG: Arc<dyn FunctionDef> = SyncMethod::arc(
        "debug",
        debug,
        vec![vec![], vec![FunctionParam::new("debug", Type::Bool)]]
    );
    pub static ref REPL_EXEC: Arc<dyn FunctionDef> = SyncMethod::arc(
        "exec",
        exec,
        vec![vec![FunctionParam::new("command", Type::String)]]
    );
    pub static ref REPL_LOAD_ABI: Arc<dyn FunctionDef> = SyncMethod::arc(
        "loadAbi",
        load_abi,
        vec![
            vec![
                FunctionParam::new("name", Type::String),
                FunctionParam::new("filepath", Type::String)
            ],
            vec![
                FunctionParam::new("name", Type::String),
                FunctionParam::new("filepath", Type::String),
                FunctionParam::new("jsonKey", Type::String)
            ]
        ]
    );
    pub static ref REPL_FETCH_ABI: Arc<dyn FunctionDef> = AsyncMethod::arc(
        "fetchAbi",
        fetch_abi,
        vec![vec![
            FunctionParam::new("name", Type::String),
            FunctionParam::new("address", Type::Address)
        ]]
    );
}
