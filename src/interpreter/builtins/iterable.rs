use std::sync::Arc;

use anyhow::{anyhow, bail, Result};
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

fn reduce<'a>(
    env: &'a mut Env,
    receiver: &'a Value,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        let items = receiver.get_items()?;
        let mut result = match args.get(1) {
            Some(v) => v,
            None => items.first().ok_or(anyhow!("empty array for reduce"))?,
        }
        .clone();
        for item in items {
            match args.first() {
                Some(Value::Func(func)) => result = func.execute(env, &[result, item]).await?,
                _ => bail!("reduce function expects a function as first argument"),
            };
        }

        Ok(result)
    }
    .boxed()
}

fn filter<'a>(
    env: &'a mut Env,
    receiver: &'a Value,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        let mut values = vec![];
        for v in receiver.get_items()? {
            match args.first() {
                Some(Value::Func(func)) => match func.execute(env, &[v.clone()]).await? {
                    Value::Bool(true) => values.push(v),
                    Value::Bool(false) => continue,
                    _ => bail!("filter function must return a boolean"),
                },
                _ => bail!("filter function expects a function as an argument"),
            };
        }
        match receiver.get_type() {
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
    pub static ref ITER_FILTER: Arc<dyn FunctionDef> = AsyncMethod::arc(
        "filter",
        filter,
        vec![vec![FunctionParam::new("p", Type::Function)]]
    );
    pub static ref ITER_REDUCE: Arc<dyn FunctionDef> = AsyncMethod::arc(
        "reduce",
        reduce,
        vec![
            vec![FunctionParam::new("f", Type::Function)],
            vec![
                FunctionParam::new("f", Type::Function),
                FunctionParam::new("init", Type::Any)
            ]
        ]
    );
    pub static ref ITER_LEN: Arc<dyn FunctionDef> = SyncProperty::arc("length", iter_len);
}
