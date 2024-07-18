use std::process::Command;

use alloy::providers::Provider;
use anyhow::{anyhow, bail, Ok, Result};
use futures::{future::BoxFuture, FutureExt};
use lazy_static::lazy_static;

use crate::{
    interpreter::{
        function_definitions::{FunctionDefinition, FunctionParam},
        ContractInfo, Env, Type, Value,
    },
    loaders,
};

fn list_vars<'a>(env: &'a mut Env, _args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
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

fn list_types<'a>(env: &'a mut Env, _args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
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

fn is_connected<'a>(env: &'a mut Env, _args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move {
        let res = env.get_provider().root().get_chain_id().await.is_ok();
        Ok(Value::Bool(res))
    }
    .boxed()
}

fn rpc<'a>(env: &'a mut Env, args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move {
        match args {
            [] => Ok(Value::Str(env.get_rpc_url())),
            [url] => {
                env.set_provider_url(&url.as_string()?)?;
                Ok(Value::Null)
            }
            _ => bail!("rpc: invalid arguments"),
        }
    }
    .boxed()
}

fn debug<'a>(env: &'a mut Env, args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move {
        match args {
            [] => Ok(Value::Bool(env.is_debug())),
            [Value::Bool(b)] => {
                env.set_debug(*b);
                Ok(Value::Null)
            }
            _ => bail!("debug: invalid arguments"),
        }
    }
    .boxed()
}

fn exec<'a>(_env: &'a mut Env, args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move {
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
    .boxed()
}

fn load_abi<'a>(env: &'a mut Env, args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move {
        let (name, filepath, key) = match args {
            [Value::Str(name), Value::Str(filepath)] => (name, filepath, None),
            [Value::Str(name), Value::Str(filepath), Value::Str(key)] => {
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

fn fetch_abi<'a>(env: &'a mut Env, args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move {
        match args {
            [Value::Str(name), Value::Addr(address)] => {
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

fn get_account<'a>(env: &'a mut Env, _args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
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

fn load_private_key<'a>(env: &'a mut Env, args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move {
        let key = match args {
            [Value::Str(key)] => key.clone(),
            [] => rpassword::prompt_password("Enter private key: ")?,
            _ => bail!("loadPrivateKey: invalid arguments"),
        };
        env.set_private_key(key.as_str())?;
        Ok(get_default_sender(env))
    }
    .boxed()
}

fn list_ledgers<'a>(env: &'a mut Env, args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move {
        let count = match args {
            [] => 5,
            [value] => value.as_usize()?,
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

fn load_ledger<'a>(env: &'a mut Env, args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move {
        let index = match args {
            [] => 0,
            [value] => value.as_usize()?,
            _ => bail!("loadLedger: invalid arguments"),
        };
        env.load_ledger(index).await?;
        Ok(get_default_sender(env))
    }
    .boxed()
}

lazy_static! {
    pub static ref REPL_LIST_VARS: FunctionDefinition = FunctionDefinition {
        name_: "vars".to_string(),
        property: true,
        valid_args: vec![vec![]],
        execute_fn: list_vars,
    };
    pub static ref REPL_LIST_TYPES: FunctionDefinition = FunctionDefinition {
        name_: "types".to_string(),
        property: true,
        valid_args: vec![vec![]],
        execute_fn: list_types,
    };
    pub static ref REPL_IS_CONNECTED: FunctionDefinition = FunctionDefinition {
        name_: "connected".to_string(),
        property: true,
        valid_args: vec![vec![]],
        execute_fn: is_connected,
    };
    pub static ref REPL_RPC: FunctionDefinition = FunctionDefinition {
        name_: "rpc".to_string(),
        property: false,
        valid_args: vec![vec![], vec![FunctionParam::new("url", Type::String)]],
        execute_fn: rpc,
    };
    pub static ref REPL_DEBUG: FunctionDefinition = FunctionDefinition {
        name_: "debug".to_string(),
        property: false,
        valid_args: vec![vec![], vec![FunctionParam::new("debug", Type::Bool)]],
        execute_fn: debug,
    };
    pub static ref REPL_EXEC: FunctionDefinition = FunctionDefinition {
        name_: "exec".to_string(),
        property: false,
        valid_args: vec![vec![FunctionParam::new("command", Type::String)]],
        execute_fn: exec,
    };
    pub static ref REPL_LOAD_ABI: FunctionDefinition = FunctionDefinition {
        name_: "loadAbi".to_string(),
        property: false,
        valid_args: vec![
            vec![
                FunctionParam::new("name", Type::String),
                FunctionParam::new("filepath", Type::String)
            ],
            vec![
                FunctionParam::new("name", Type::String),
                FunctionParam::new("filepath", Type::String),
                FunctionParam::new("jsonKey", Type::String)
            ]
        ],
        execute_fn: load_abi,
    };
    pub static ref REPL_FETCH_ABI: FunctionDefinition = FunctionDefinition {
        name_: "fetchAbi".to_string(),
        property: false,
        valid_args: vec![vec![
            FunctionParam::new("name", Type::String),
            FunctionParam::new("address", Type::Address)
        ]],
        execute_fn: fetch_abi,
    };
    pub static ref REPL_ACCOUNT: FunctionDefinition = FunctionDefinition {
        name_: "account".to_string(),
        property: true,
        valid_args: vec![vec![]],
        execute_fn: get_account,
    };
    pub static ref REPL_LOAD_PRIVATE_KEY: FunctionDefinition = FunctionDefinition {
        name_: "loadPrivateKey".to_string(),
        property: false,
        valid_args: vec![vec![], vec![FunctionParam::new("privateKey", Type::String)]],
        execute_fn: load_private_key,
    };
    pub static ref REPL_LIST_LEDGER_WALLETS: FunctionDefinition = FunctionDefinition {
        name_: "listLedgerWallets".to_string(),
        property: false,
        valid_args: vec![vec![], vec![FunctionParam::new("count", Type::Uint(256))]],
        execute_fn: list_ledgers,
    };
    pub static ref REPL_LOAD_LEDGER: FunctionDefinition = FunctionDefinition {
        name_: "loadLedger".to_string(),
        property: false,
        valid_args: vec![vec![], vec![FunctionParam::new("index", Type::Uint(256))]],
        execute_fn: load_ledger,
    };
}
