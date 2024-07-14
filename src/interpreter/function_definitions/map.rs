use anyhow::{bail, Result};
use futures::{future::BoxFuture, FutureExt};

use crate::interpreter::{Env, Type, Value};

fn map<'a>(
    target: &'a [Value],
    ty: Type,
    args: &'a [Value],
    env: &'a mut Env,
) -> BoxFuture<'a, Result<Value>> {
    async move {
        let func_value = args
            .first()
            .ok_or_else(|| anyhow::anyhow!("map function expects a single argument"))?;
        let mut values = vec![];
        for v in target {
            let value = match func_value {
                Value::Func(func) => func.execute(&[v.clone()], env).await?,
                Value::TypeObject(type_) => type_.cast(v)?,
                _ => bail!("map function expects a function or type as an argument"),
            };
            values.push(value);
        }
        match ty {
            Type::Tuple(_) => Ok(Value::Tuple(values)),
            Type::Array(t) => Ok(Value::Array(values, t.clone())),
            _ => bail!("cannot map to type {}", ty),
        }
    }
    .boxed()
}
