use std::sync::Arc;

use anyhow::Result;
use lazy_static::lazy_static;

use crate::interpreter::{
    functions::{FunctionDef, FunctionParam, SyncMethod},
    Env, Type, Value,
};

fn stringify(_env: &mut Env, _receiver: &Value, args: &[Value]) -> Result<Value> {
    let value = args.first().unwrap();
    let s = serde_json::to_string(value)?;
    Ok(Value::Str(s))
}

lazy_static! {
    pub static ref JSON_STRINGIFY: Arc<dyn FunctionDef> = SyncMethod::arc(
        "stringify",
        stringify,
        vec![vec![FunctionParam::new("value", Type::Any)]]
    );
}
