use std::sync::Arc;

use crate::interpreter::{
    functions::{FunctionDef, SyncProperty},
    Env, Type, Value,
};
use anyhow::{bail, Result};
use lazy_static::lazy_static;

pub fn event_selector(_env: &Env, receiver: &Value) -> Result<Value> {
    let event_abi = match receiver {
        Value::TypeObject(Type::Event(event)) => event,
        _ => bail!("selector function expects receiver to be an event"),
    };

    Ok(event_abi.selector().into())
}

lazy_static! {
    pub static ref EVENT_SELECTOR: Arc<dyn FunctionDef> =
        SyncProperty::arc("selector", event_selector);
}
