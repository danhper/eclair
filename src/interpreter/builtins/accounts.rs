use std::{collections::HashMap, fs::DirEntry, sync::Arc};

use crate::interpreter::{
    functions::{AsyncMethod, FunctionDef, FunctionParam, SyncMethod, SyncProperty},
    types::{HashableIndexMap, WALLET_TYPE},
    Env, Type, Value,
};
use alloy::{
    primitives::Address,
    signers::local::{LocalSigner, PrivateKeySigner},
};
use anyhow::{anyhow, bail, Result};
use futures::{future::BoxFuture, FutureExt};
use lazy_static::lazy_static;

fn get_account(env: &Env, _receiver: &Value) -> Result<Value> {
    let account = env.get_default_sender();
    Ok(account.map(Value::Addr).unwrap_or(Value::Null))
}

fn get_default_sender(env: &Env) -> Value {
    env.get_default_sender()
        .map(Value::Addr)
        .unwrap_or(Value::Null)
}

fn load_private_key(env: &mut Env, _receiver: &Value, args: &[Value]) -> Result<Value> {
    let (signer, alias): (PrivateKeySigner, Option<String>) = match args {
        [Value::Str(key)] => (key.parse()?, None),
        [Value::Str(key), Value::Str(alias)] => (key.parse()?, Some(alias.clone())),
        [Value::FixBytes(bytes, 32)] => (PrivateKeySigner::from_bytes(bytes)?, None),
        [Value::FixBytes(bytes, 32), Value::Str(alias)] => {
            (PrivateKeySigner::from_bytes(bytes)?, Some(alias.clone()))
        }
        [] => {
            let signer = rpassword::prompt_password("Enter private key: ")?.parse()?;
            (signer, None)
        }
        _ => bail!("loadPrivateKey: invalid arguments"),
    };
    env.set_signer(signer)?;
    if let (Some(alias), Some(address)) = (alias, env.get_default_sender()) {
        env.set_account_alias(alias.as_str(), address);
    }
    Ok(get_default_sender(env))
}

fn get_loaded_wallets(env: &Env, _receiver: &Value) -> Result<Value> {
    let loaded_wallets = env.get_loaded_wallets();
    let aliases = env.list_account_aliases();
    let reversed_aliases: HashMap<Address, String> = HashMap::from_iter(
        aliases
            .iter()
            .map(|(alias, address)| (*address, alias.to_string())),
    );
    let mut wallets = vec![];
    for wallet in loaded_wallets {
        let alias = reversed_aliases.get(&wallet);
        let alias_value = alias.cloned().map(Value::Str).unwrap_or(Value::Null);
        wallets.push(Value::NamedTuple(
            "Wallet".to_string(),
            HashableIndexMap::from_iter([
                ("address".to_string(), Value::Addr(wallet)),
                ("alias".to_string(), alias_value),
            ]),
        ));
    }
    Ok(Value::Array(wallets, Box::new(WALLET_TYPE.clone())))
}

fn select_wallet(env: &mut Env, _receiver: &Value, args: &[Value]) -> Result<Value> {
    match args {
        [Value::Addr(address)] => env.select_wallet(*address)?,
        [Value::Str(alias)] => env.select_wallet_by_alias(alias)?,
        _ => bail!("selectWallet: invalid arguments"),
    };
    Ok(get_default_sender(env))
}

fn load_keystore(env: &mut Env, _receiver: &Value, args: &[Value]) -> Result<Value> {
    let (account, alias, password) = match args {
        [Value::Str(account)] => (account.clone(), None, None),
        [Value::Str(account), Value::Str(alias)] => (account.clone(), Some(alias.clone()), None),
        [Value::Str(account), Value::Null, Value::Str(password)] => {
            (account.clone(), None, Some(password.clone()))
        }
        [Value::Str(account), Value::Str(alias), Value::Str(password)] => {
            (account.clone(), Some(alias.clone()), Some(password.clone()))
        }
        _ => bail!("loadKeystore: invalid arguments"),
    };
    let password = if let Some(password) = password {
        password
    } else {
        rpassword::prompt_password("Enter password: ")?
    };
    let foundry_dir =
        foundry_config::Config::foundry_dir().ok_or(anyhow!("foundry dir not found"))?;
    let keystore_file_path = foundry_dir.join("keystores").join(account.as_str());
    let signer = LocalSigner::decrypt_keystore(keystore_file_path, password)?;
    env.set_signer(signer)?;
    if let (Some(alias), Some(address)) = (alias, env.get_default_sender()) {
        env.set_account_alias(alias.as_str(), address);
    }
    Ok(get_default_sender(env))
}

fn _get_filename(file: Result<DirEntry, std::io::Error>) -> Result<Value> {
    let file = file?;
    if !file.file_type()?.is_file() {
        return Err(anyhow!("not a file"));
    }
    if let Some(name) = file.file_name().to_str() {
        Ok(Value::Str(name.to_string()))
    } else {
        Err(anyhow!("Invalid UTF-8 in filename"))
    }
}
fn list_keystores(_env: &mut Env, _receiver: &Value, _args: &[Value]) -> Result<Value> {
    let foundry_dir =
        foundry_config::Config::foundry_dir().ok_or(anyhow!("foundry dir not found"))?;
    let keystore_dir = foundry_dir.join("keystores");
    let files = std::fs::read_dir(keystore_dir)?;
    let valid_files: Vec<_> = files.filter_map(|file| _get_filename(file).ok()).collect();
    Ok(Value::Array(valid_files, Box::new(Type::String)))
}

fn list_ledgers<'a>(
    env: &'a mut Env,
    _receiver: &Value,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
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

fn alias_wallet(env: &mut Env, _receiver: &Value, args: &[Value]) -> Result<Value> {
    match args {
        [Value::Addr(address), Value::Str(alias)] => {
            env.set_account_alias(alias, *address);
            Ok(Value::Null)
        }
        _ => bail!("aliasWallet: invalid arguments"),
    }
}

fn load_ledger<'a>(
    env: &'a mut Env,
    _receiver: &'a Value,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        let (index, alias) = match args {
            [] => (0, None),
            [value] => (value.as_usize()?, None),
            [value, Value::Str(alias)] => (value.as_usize()?, Some(alias.clone())),
            _ => bail!("loadLedger: invalid arguments"),
        };
        env.load_ledger(index).await?;
        if let (Some(alias), Some(address)) = (alias, env.get_default_sender()) {
            env.set_account_alias(alias.as_str(), address);
        }
        Ok(get_default_sender(env))
    }
    .boxed()
}

lazy_static! {
    pub static ref ACCOUNT_CURRENT: Arc<dyn FunctionDef> =
        SyncProperty::arc("current", get_account);
    pub static ref ACCOUNT_LOAD_PRIVATE_KEY: Arc<dyn FunctionDef> = SyncMethod::arc(
        "loadPrivateKey",
        load_private_key,
        vec![
            vec![],
            vec![FunctionParam::new("privateKey", Type::String)],
            vec![
                FunctionParam::new("privateKey", Type::String),
                FunctionParam::new("alias", Type::String)
            ],
            vec![
                FunctionParam::new("privateKey", Type::FixBytes(32)),
                FunctionParam::new("alias", Type::String)
            ],
        ]
    );
    pub static ref ACCOUNT_LIST_KEYSTORES: Arc<dyn FunctionDef> =
        SyncMethod::arc("listKeystores", list_keystores, vec![vec![]]);
    pub static ref ACCOUNT_LOAD_KEYSTORE: Arc<dyn FunctionDef> = SyncMethod::arc(
        "loadKeystore",
        load_keystore,
        vec![
            vec![FunctionParam::new("account", Type::String)],
            vec![
                FunctionParam::new("account", Type::String),
                FunctionParam::new("alias", Type::String)
            ],
            vec![
                FunctionParam::new("account", Type::String),
                FunctionParam::new("alias", Type::String),
                FunctionParam::new("password", Type::String)
            ],
            vec![
                FunctionParam::new("account", Type::String),
                FunctionParam::new("alias", Type::Null),
                FunctionParam::new("password", Type::String)
            ],
        ]
    );
    pub static ref ACCOUNT_LIST_LEDGER_WALLETS: Arc<dyn FunctionDef> = AsyncMethod::arc(
        "listLedgerWallets",
        list_ledgers,
        vec![vec![], vec![FunctionParam::new("count", Type::Uint(256))]]
    );
    pub static ref ACCOUNT_LOAD_LEDGER: Arc<dyn FunctionDef> = AsyncMethod::arc(
        "loadLedger",
        load_ledger,
        vec![
            vec![],
            vec![FunctionParam::new("index", Type::Uint(256))],
            vec![
                FunctionParam::new("index", Type::Uint(256)),
                FunctionParam::new("alias", Type::String)
            ]
        ]
    );
    pub static ref ACCOUNT_GET_LOADED: Arc<dyn FunctionDef> =
        SyncProperty::arc("loaded", get_loaded_wallets);
    pub static ref ACCOUNT_SELECT: Arc<dyn FunctionDef> = SyncMethod::arc(
        "select",
        select_wallet,
        vec![
            vec![FunctionParam::new("address", Type::Address)],
            vec![FunctionParam::new("alias", Type::String)],
        ]
    );
    pub static ref ACCOUNT_ALIAS: Arc<dyn FunctionDef> = SyncMethod::arc(
        "alias",
        alias_wallet,
        vec![vec![
            FunctionParam::new("address", Type::Address),
            FunctionParam::new("alias", Type::String)
        ],]
    );
}
