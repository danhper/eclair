use std::sync::Arc;

use anyhow::Result;
use lazy_static::lazy_static;

use crate::interpreter::{
    functions::{FunctionDef, SyncMethod},
    Env, Value,
};

fn log(_env: &mut Env, _receiver: &Value, args: &[Value]) -> Result<Value> {
    args.iter().for_each(|arg| println!("{}", arg));
    Ok(Value::Null)
}

lazy_static! {
    pub static ref CONSOLE_LOG: Arc<dyn FunctionDef> = SyncMethod::arc("log", log, vec![]);
}
