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
    pub fn all() -> Vec<String> {
        ["types", "vars", "rpc", "debug", "exec", "connected"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    pub fn is_property(&self) -> bool {
        matches!(
            self,
            Directive::Connected | Directive::ListVars | Directive::ListTypes
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
                [url] => env.set_provider(&url.as_string()?),
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
        }

        Ok(Value::Null)
    }

    pub fn from_name(name: &str) -> Result<Self> {
        match name {
            "vars" => Ok(Directive::ListVars),
            "types" => Ok(Directive::ListTypes),
            "rpc" => Ok(Directive::Rpc),
            "debug" => Ok(Directive::Debug),
            "exec" => Ok(Directive::Exec),
            "connected" => Ok(Directive::Connected),
            _ => Err(anyhow::anyhow!("Invalid directive")),
        }
    }
}
