use anyhow::{anyhow, bail, Result};
use futures::{future::BoxFuture, FutureExt};
use lazy_static::lazy_static;

use crate::interpreter::{
    builtins::{FunctionDefinition, FunctionParam},
    Env, Type, Value,
};

fn map<'a>(env: &'a mut Env, args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move {
        let mut args_iter = args.iter();
        let receiver = args_iter
            .next()
            .ok_or(anyhow!("map function expects a receiver"))?;
        let func = args_iter
            .next()
            .ok_or(anyhow!("map expects a single argument"))?;

        let mut values = vec![];
        for v in receiver.get_items()? {
            let value = match func {
                Value::Func(func) => func.execute(&[v.clone()], env).await?,
                Value::TypeObject(type_) => type_.cast(&v)?,
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

fn length<'a>(_env: &'a mut Env, args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move { args.first().expect("no receiver").len().map(Into::into) }.boxed()
}

lazy_static! {
    pub static ref ITER_MAP: FunctionDefinition = FunctionDefinition {
        name_: "map".to_string(),
        property: false,
        valid_args: vec![vec![FunctionParam::new("f", Type::Function)]],
        execute_fn: map,
    };
    pub static ref ITER_LENGTH: FunctionDefinition = FunctionDefinition {
        name_: "length".to_string(),
        property: true,
        valid_args: vec![vec![]],
        execute_fn: length,
    };
}
