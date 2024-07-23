use std::sync::Arc;

use anyhow::{bail, Result};
use futures::{future::BoxFuture, FutureExt};
use lazy_static::lazy_static;

use crate::interpreter::{
    functions::{AsyncMethod, FunctionDef, FunctionParam, SyncProperty},
    Env, Type, Value,
};

fn map<'a>(
    env: &'a mut Env,
    receiver: &'a Value,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        let mut values = vec![];
        for v in receiver.get_items()? {
            let value = match args.first() {
                Some(Value::Func(func)) => func.execute(env, &[v.clone()]).await?,
                Some(Value::TypeObject(type_)) => type_.cast(&v)?,
                _ => bail!("map function expects a function or type as an argument"),
            };
            values.push(value);
        }
        match receiver.get_type() {
            Type::Tuple(_) => Ok(Value::Tuple(values)),
            Type::Array(t) => Ok(Value::Array(values, t.clone())),
            ty => bail!("cannot map to type {}", ty),
        }
    }
    .boxed()
}

pub fn iter_len(_env: &Env, arg: &Value) -> Result<Value> {
    arg.len().map(Into::into)
}

lazy_static! {
    pub static ref ITER_MAP: Arc<dyn FunctionDef> = AsyncMethod::arc(
        "map",
        map,
        vec![vec![FunctionParam::new("f", Type::Function)]]
    );
    pub static ref ITER_LEN: Arc<dyn FunctionDef> = SyncProperty::arc("length", iter_len);
}
