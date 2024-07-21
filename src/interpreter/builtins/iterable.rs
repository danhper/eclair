use anyhow::{anyhow, bail, Result};
use futures::{future::BoxFuture, FutureExt};
use lazy_static::lazy_static;

use crate::interpreter::{
    functions::{FunctionDefinition, FunctionDefinitionBuilder, FunctionParam},
    Env, Type, Value,
};

fn map<'a>(
    _def: &'a FunctionDefinition,
    env: &'a mut Env,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
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

fn length<'a>(
    _def: &'a FunctionDefinition,
    _env: &'a mut Env,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move { args.first().expect("no receiver").len().map(Into::into) }.boxed()
}

lazy_static! {
    pub static ref ITER_MAP: FunctionDefinition = FunctionDefinitionBuilder::new("map", map)
        .add_valid_args(&[FunctionParam::new("f", Type::Function)])
        .build();
    pub static ref ITER_LENGTH: FunctionDefinition =
        FunctionDefinitionBuilder::property("length", length).build();
}
