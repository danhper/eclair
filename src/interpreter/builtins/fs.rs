use std::sync::Arc;

use anyhow::{anyhow, Result};
use lazy_static::lazy_static;

use crate::interpreter::{
    functions::{FunctionDef, FunctionParam, SyncMethod},
    Env, Type, Value,
};

fn write(_env: &mut Env, _receiver: &Value, args: &[Value]) -> Result<Value> {
    let filepath = args
        .first()
        .ok_or(anyhow!("missing filepath"))?
        .as_string()?;
    let value = args.get(1).ok_or(anyhow!("missing value"))?.as_string()?;
    std::fs::write(filepath, value)?;
    Ok(Value::Null)
}

lazy_static! {
    pub static ref FS_WRITE: Arc<dyn FunctionDef> = SyncMethod::arc(
        "write",
        write,
        vec![vec![
            FunctionParam::new("filepath", Type::String),
            FunctionParam::new("content", Type::String)
        ]]
    );
}
