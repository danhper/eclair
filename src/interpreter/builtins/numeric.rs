use alloy::transports::BoxFuture;
use anyhow::{bail, Result};
use futures::FutureExt;
use lazy_static::lazy_static;

use crate::interpreter::{
    functions::{FunctionDefinition, FunctionDefinitionBuilder, FunctionParam},
    Env, Type, Value,
};

fn mul_div_args(args: &[Value]) -> Result<(Value, u64)> {
    match args {
        [v2] => Ok((v2.clone(), 18)),
        [v2, d] => Ok((v2.clone(), d.as_u64()?)),
        _ => bail!("mul function expects one or two arguments"),
    }
}

fn mul<'a>(
    _def: &'a FunctionDefinition,
    _env: &'a mut Env,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        let (value, decimals) = mul_div_args(&args[1..])?;
        (args[0].clone() * value.clone())? / Value::decimal_multiplier(decimals as u8)
    }
    .boxed()
}

fn div<'a>(
    _def: &'a FunctionDefinition,
    _env: &'a mut Env,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        let (value, decimals) = mul_div_args(&args[1..])?;
        (args[0].clone() * Value::decimal_multiplier(decimals as u8))? / value.clone()
    }
    .boxed()
}

fn type_min<'a>(
    _def: &'a FunctionDefinition,
    _env: &'a mut Env,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move { args[0].get_type().min().map(Into::into) }.boxed()
}

fn type_max<'a>(
    _def: &'a FunctionDefinition,
    _env: &'a mut Env,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move { args[0].get_type().max().map(Into::into) }.boxed()
}

lazy_static! {
    pub static ref NUM_MUL: FunctionDefinition = FunctionDefinitionBuilder::new("mul", mul)
        .add_valid_args(&[FunctionParam::new("factor", Type::Uint(256))])
        .add_valid_args(&[
            FunctionParam::new("factor", Type::Uint(256)),
            FunctionParam::new("decimals", Type::Uint(8))
        ])
        .build();
    pub static ref NUM_DIV: FunctionDefinition = FunctionDefinitionBuilder::new("div", div)
        .add_valid_args(&[FunctionParam::new("divisor", Type::Uint(256))])
        .add_valid_args(&[
            FunctionParam::new("divisor", Type::Uint(256)),
            FunctionParam::new("decimals", Type::Uint(8))
        ])
        .build();
    pub static ref TYPE_MAX: FunctionDefinition =
        FunctionDefinitionBuilder::property("max", type_max).build();
    pub static ref TYPE_MIN: FunctionDefinition =
        FunctionDefinitionBuilder::property("min", type_min).build();
}
