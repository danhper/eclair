use std::process::Command;

use alloy::providers::Provider;
use anyhow::{bail, Result};

use super::{Env, Value};

#[derive(Debug, PartialEq, Clone)]
pub enum Directive {
    ListVars,
    ListTypes,
    Rpc,
    Debug,
    Exec,
    Connected,
    Account,
    LoadPrivateKey,
    LoadLedger,
    ListLedgerWallets,
}

impl std::fmt::Display for Directive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Directive::ListVars => write!(f, "vars"),
            Directive::ListTypes => write!(f, "types"),
            Directive::Rpc => write!(f, "rpc"),
            Directive::Debug => write!(f, "debug"),
            Directive::Exec => write!(f, "exec"),
            Directive::Connected => write!(f, "connected"),
            Directive::LoadPrivateKey => write!(f, "loadPrivateKey"),
            Directive::Account => write!(f, "account"),
            Directive::LoadLedger => write!(f, "loadLedger"),
            Directive::ListLedgerWallets => write!(f, "listLedgerWallets"),
        }
    }
}

fn list_vars(env: &Env) {
    let mut vars = env.list_vars();
    vars.sort();
    for k in vars.iter() {
        println!("{}: {}", k, env.get_var(k).unwrap());
    }
}

fn list_types(env: &Env) {
    let mut types = env.list_types();
    types.sort();
    for k in types.iter() {
        println!("{}", k);
    }
}

impl Directive {
    pub fn all() -> Vec<Directive> {
        vec![
            Directive::ListVars,
            Directive::ListTypes,
            Directive::Rpc,
            Directive::Debug,
            Directive::Exec,
            Directive::Connected,
            Directive::Account,
            Directive::LoadPrivateKey,
            Directive::LoadLedger,
            Directive::ListLedgerWallets,
        ]
    }

    pub fn is_property(&self) -> bool {
        matches!(
            self,
            Directive::Connected | Directive::ListVars | Directive::ListTypes | Directive::Account
        )
    }

    pub async fn execute(&self, args: &[Value], env: &mut Env) -> Result<Value> {
        match self {
            Directive::ListVars => list_vars(env),
            Directive::ListTypes => list_types(env),
            Directive::Connected => {
                let res = env.get_provider().root().get_chain_id().await.is_ok();
                return Ok(Value::Bool(res));
            }
            Directive::Rpc => match args {
                [] => println!("{}", env.get_provider().root().client().transport().url()),
                [url] => env.set_provider_url(&url.as_string()?)?,
                _ => bail!("rpc: invalid arguments"),
            },
            Directive::Debug => match args {
                [] => return Ok(Value::Bool(env.is_debug())),
                [Value::Bool(b)] => env.set_debug(*b),
                _ => bail!("debug: invalid arguments"),
            },
            Directive::Exec => match args {
                [Value::Str(cmd)] => {
                    let splitted = cmd.split_whitespace().collect::<Vec<_>>();
                    Command::new(splitted[0]).args(&splitted[1..]).spawn()?;
                }
                _ => bail!("exec: invalid arguments"),
            },
            Directive::Account => {
                let account = env.get_default_sender();
                return Ok(account.map(Value::Addr).unwrap_or(Value::Null));
            }
            Directive::LoadPrivateKey => match args {
                [Value::Str(key)] => {
                    env.set_private_key(key.as_str())?;
                    return Ok(self.get_default_sender(env));
                }
                [] => {
                    let key = rpassword::prompt_password("Enter private key: ")?;
                    env.set_private_key(key.as_str())?;
                    return Ok(self.get_default_sender(env));
                }
                _ => bail!("loadPrivateKey: invalid arguments"),
            },
            Directive::ListLedgerWallets => {
                let count = match args {
                    [] => 5,
                    [value] => value.as_usize()?,
                    _ => bail!("listLedgerWallets: invalid arguments"),
                };
                let wallets = env.list_ledger_wallets(count).await?;
                return Ok(Value::Array(wallets.into_iter().map(Value::Addr).collect()));
            }
            Directive::LoadLedger => {
                let index = match args {
                    [] => 0,
                    [value] => value.as_usize()?,
                    _ => bail!("loadLedger: invalid arguments"),
                };
                env.load_ledger(index).await?;
                return Ok(self.get_default_sender(env));
            }
        }

        Ok(Value::Null)
    }

    fn get_default_sender(&self, env: &Env) -> Value {
        env.get_default_sender()
            .map(Value::Addr)
            .unwrap_or(Value::Null)
    }

    pub fn from_name(name: &str) -> Result<Self> {
        match name {
            "vars" => Ok(Directive::ListVars),
            "types" => Ok(Directive::ListTypes),
            "rpc" => Ok(Directive::Rpc),
            "debug" => Ok(Directive::Debug),
            "exec" => Ok(Directive::Exec),
            "connected" => Ok(Directive::Connected),
            "account" => Ok(Directive::Account),
            "loadPrivateKey" => Ok(Directive::LoadPrivateKey),
            "listLedgerWallets" => Ok(Directive::ListLedgerWallets),
            "loadLedger" => Ok(Directive::LoadLedger),
            _ => Err(anyhow::anyhow!("Invalid directive")),
        }
    }
}
