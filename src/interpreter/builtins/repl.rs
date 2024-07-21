use std::process::Command;

use alloy::providers::Provider;
use anyhow::{anyhow, bail, Ok, Result};
use futures::{future::BoxFuture, FutureExt};
use lazy_static::lazy_static;

use crate::{
    interpreter::{
        functions::{FunctionDefinition, FunctionDefinitionBuilder, FunctionParam},
        ContractInfo, Env, Type, Value,
    },
    loaders,
};

fn list_vars<'a>(
    _def: &'a FunctionDefinition,
    env: &'a mut Env,
    _args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        let mut vars = env.list_vars();
        vars.sort();
        for k in vars.iter() {
            println!("{}: {}", k, env.get_var(k).unwrap());
        }
        Ok(Value::Null)
    }
    .boxed()
}

fn list_types<'a>(
    _def: &'a FunctionDefinition,
    env: &'a mut Env,
    _args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        let mut types = env.list_types();
        types.sort();
        for k in types.iter() {
            println!("{}", k);
        }
        Ok(Value::Null)
    }
    .boxed()
}

fn is_connected<'a>(
    _def: &'a FunctionDefinition,
    env: &'a mut Env,
    _args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        let res = env.get_provider().root().get_chain_id().await.is_ok();
        Ok(Value::Bool(res))
    }
    .boxed()
}

fn rpc<'a>(
    _def: &'a FunctionDefinition,
    env: &'a mut Env,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        match args {
            [_] => Ok(Value::Str(env.get_rpc_url())),
            [_, url] => {
                env.set_provider_url(&url.as_string()?)?;
                Ok(Value::Null)
            }
            _ => bail!("rpc: invalid arguments"),
        }
    }
    .boxed()
}

fn debug<'a>(
    _def: &'a FunctionDefinition,
    env: &'a mut Env,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        match args {
            [_] => Ok(Value::Bool(env.is_debug())),
            [_, Value::Bool(b)] => {
                env.set_debug(*b);
                Ok(Value::Null)
            }
            _ => bail!("debug: invalid arguments"),
        }
    }
    .boxed()
}

fn exec<'a>(
    _def: &'a FunctionDefinition,
    _env: &'a mut Env,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        let cmd = args
            .get(1)
            .ok_or(anyhow!("exec: missing command"))?
            .as_string()?;

        let splitted = cmd.split_whitespace().collect::<Vec<_>>();
        let mut cmd = Command::new(splitted[0]).args(&splitted[1..]).spawn()?;
        let res = cmd.wait()?;
        let code = res.code().ok_or(anyhow!("exec: command failed"))?;
        Ok(code.into())
    }
    .boxed()
}

fn load_abi<'a>(
    _def: &'a FunctionDefinition,
    env: &'a mut Env,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        let (name, filepath, key) = match args {
            [_, Value::Str(name), Value::Str(filepath)] => (name, filepath, None),
            [_, Value::Str(name), Value::Str(filepath), Value::Str(key)] => {
                (name, filepath, Some(key.as_str()))
            }
            _ => bail!("loadAbi: invalid arguments"),
        };
        let abi = loaders::file::load_abi(filepath, key)?;
        let contract_info = ContractInfo(name.to_string(), abi);
        env.set_type(name, Type::Contract(contract_info.clone()));
        Ok(Value::Null)
    }
    .boxed()
}

fn fetch_abi<'a>(
    _def: &'a FunctionDefinition,
    env: &'a mut Env,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        match args {
            [_, Value::Str(name), Value::Addr(address)] => {
                let chain_id = env.get_chain_id().await?;
                let etherscan_config = env.config.get_etherscan_config(chain_id)?;
                let abi =
                    loaders::etherscan::load_abi(etherscan_config, &address.to_string()).await?;
                let contract_info = ContractInfo(name.to_string(), abi);
                env.set_type(name, Type::Contract(contract_info.clone()));
                Ok(Value::Contract(contract_info, *address))
            }
            _ => bail!("fetchAbi: invalid arguments"),
        }
    }
    .boxed()
}

fn get_account<'a>(
    _def: &'a FunctionDefinition,
    env: &'a mut Env,
    _args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        let account = env.get_default_sender();
        Ok(account.map(Value::Addr).unwrap_or(Value::Null))
    }
    .boxed()
}

fn get_default_sender(env: &Env) -> Value {
    env.get_default_sender()
        .map(Value::Addr)
        .unwrap_or(Value::Null)
}

fn load_private_key<'a>(
    _def: &'a FunctionDefinition,
    env: &'a mut Env,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        let key = match args {
            [_, Value::Str(key)] => key.clone(),
            [_] => rpassword::prompt_password("Enter private key: ")?,
            _ => bail!("loadPrivateKey: invalid arguments"),
        };
        env.set_private_key(key.as_str())?;
        Ok(get_default_sender(env))
    }
    .boxed()
}

fn list_ledgers<'a>(
    _def: &'a FunctionDefinition,
    env: &'a mut Env,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        let count = match args {
            [_] => 5,
            [_, value] => value.as_usize()?,
            _ => bail!("listLedgerWallets: invalid arguments"),
        };
        let wallets = env.list_ledger_wallets(count).await?;
        Ok(Value::Array(
            wallets.into_iter().map(Value::Addr).collect(),
            Box::new(Type::Address),
        ))
    }
    .boxed()
}

fn load_ledger<'a>(
    _def: &'a FunctionDefinition,
    env: &'a mut Env,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        let index = match args {
            [_] => 0,
            [_, value] => value.as_usize()?,
            _ => bail!("loadLedger: invalid arguments"),
        };
        env.load_ledger(index).await?;
        Ok(get_default_sender(env))
    }
    .boxed()
}

lazy_static! {
    pub static ref REPL_LIST_VARS: FunctionDefinition =
        FunctionDefinitionBuilder::property("vars", list_vars).build();
    pub static ref REPL_LIST_TYPES: FunctionDefinition =
        FunctionDefinitionBuilder::property("types", list_types).build();
    pub static ref REPL_IS_CONNECTED: FunctionDefinition =
        FunctionDefinitionBuilder::property("connected", is_connected).build();
    pub static ref REPL_RPC: FunctionDefinition = FunctionDefinitionBuilder::new("rpc", rpc)
        .add_valid_args(&[])
        .add_valid_args(&[FunctionParam::new("url", Type::String)])
        .build();
    pub static ref REPL_DEBUG: FunctionDefinition = FunctionDefinitionBuilder::new("debug", debug)
        .add_valid_args(&[])
        .add_valid_args(&[FunctionParam::new("debug", Type::Bool)])
        .build();
    pub static ref REPL_EXEC: FunctionDefinition = FunctionDefinitionBuilder::new("exec", exec)
        .add_valid_args(&[FunctionParam::new("command", Type::String)])
        .build();
    pub static ref REPL_LOAD_ABI: FunctionDefinition =
        FunctionDefinitionBuilder::new("loadAbi", load_abi)
            .add_valid_args(&[
                FunctionParam::new("name", Type::String),
                FunctionParam::new("filepath", Type::String)
            ])
            .add_valid_args(&[
                FunctionParam::new("name", Type::String),
                FunctionParam::new("filepath", Type::String),
                FunctionParam::new("jsonKey", Type::String)
            ])
            .build();
    pub static ref REPL_FETCH_ABI: FunctionDefinition =
        FunctionDefinitionBuilder::new("fetchAbi", fetch_abi)
            .add_valid_args(&[
                FunctionParam::new("name", Type::String),
                FunctionParam::new("address", Type::Address)
            ])
            .build();
    pub static ref REPL_ACCOUNT: FunctionDefinition =
        FunctionDefinitionBuilder::property("account", get_account).build();
    pub static ref REPL_LOAD_PRIVATE_KEY: FunctionDefinition =
        FunctionDefinitionBuilder::new("loadPrivateKey", load_private_key)
            .add_valid_args(&[])
            .add_valid_args(&[FunctionParam::new("privateKey", Type::String)])
            .build();
    pub static ref REPL_LIST_LEDGER_WALLETS: FunctionDefinition =
        FunctionDefinitionBuilder::new("listLedgerWallets", list_ledgers)
            .add_valid_args(&[])
            .add_valid_args(&[FunctionParam::new("count", Type::Uint(256))])
            .build();
    pub static ref REPL_LOAD_LEDGER: FunctionDefinition =
        FunctionDefinitionBuilder::new("loadLedger", load_ledger)
            .add_valid_args(&[])
            .add_valid_args(&[FunctionParam::new("index", Type::Uint(256))])
            .build();
}
