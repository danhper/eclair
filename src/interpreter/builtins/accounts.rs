use std::sync::Arc;

use crate::interpreter::{
    functions::{AsyncMethod, FunctionDef, FunctionParam, SyncMethod, SyncProperty},
    Env, Type, Value,
};
use alloy::signers::local::{LocalSigner, PrivateKeySigner};
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
    let signer: PrivateKeySigner = match args {
        [Value::Str(key)] => key.parse()?,
        [Value::FixBytes(bytes, 32)] => PrivateKeySigner::from_bytes(bytes)?,
        [] => rpassword::prompt_password("Enter private key: ")?.parse()?,
        _ => bail!("loadPrivateKey: invalid arguments"),
    };
    env.set_signer(signer)?;
    Ok(get_default_sender(env))
}

fn load_keystore(env: &mut Env, _receiver: &Value, args: &[Value]) -> Result<Value> {
    let (account, password) = match args {
        [Value::Str(account)] => (
            account.clone(),
            rpassword::prompt_password("Enter password: ")?,
        ),
        [Value::Str(account), Value::Str(password)] => (account.clone(), password.clone()),
        _ => bail!("loadKeystore: invalid arguments"),
    };
    let foundry_dir =
        foundry_config::Config::foundry_dir().ok_or(anyhow!("foundry dir not found"))?;
    let keystore_file_path = foundry_dir.join("keystores").join(account.as_str());
    let signer = LocalSigner::decrypt_keystore(keystore_file_path, password)?;
    env.set_signer(signer)?;
    Ok(get_default_sender(env))
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

fn load_ledger<'a>(
    env: &'a mut Env,
    _receiver: &'a Value,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
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
    pub static ref ACCOUNT_CURRENT: Arc<dyn FunctionDef> =
        SyncProperty::arc("current", get_account);
    pub static ref ACCOUNT_LOAD_PRIVATE_KEY: Arc<dyn FunctionDef> = SyncMethod::arc(
        "loadPrivateKey",
        load_private_key,
        vec![
            vec![],
            vec![FunctionParam::new("privateKey", Type::String)],
            vec![FunctionParam::new("privateKey", Type::FixBytes(32))]
        ]
    );
    pub static ref ACCOUNT_LOAD_KEYSTORE: Arc<dyn FunctionDef> = SyncMethod::arc(
        "loadKeystore",
        load_keystore,
        vec![
            vec![FunctionParam::new("account", Type::String)],
            vec![
                FunctionParam::new("account", Type::String),
                FunctionParam::new("password", Type::String)
            ]
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
        vec![vec![], vec![FunctionParam::new("index", Type::Uint(256))]]
    );
}
