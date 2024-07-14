use alloy::transports::BoxFuture;
use anyhow::{bail, Result};
use futures::FutureExt;
use lazy_static::lazy_static;

use crate::interpreter::{
    function_definitions::{FunctionDefinition, FunctionParam},
    Env, Type, Value,
};

fn mul_div_args(args: &[Value]) -> Result<(Value, u64)> {
    match args {
        [v2] => Ok((v2.clone(), 18)),
        [v2, d] => Ok((v2.clone(), d.as_u64()?)),
        _ => bail!("mul function expects one or two arguments"),
    }
}

fn mul<'a>(_env: &'a mut Env, args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move {
        let (value, decimals) = mul_div_args(&args[1..])?;
        (args[0].clone() * value.clone())? / Value::decimal_multiplier(decimals as u8)
    }
    .boxed()
}

fn div<'a>(_env: &'a mut Env, args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move {
        let (value, decimals) = mul_div_args(&args[1..])?;
        (args[0].clone() * Value::decimal_multiplier(decimals as u8))? / value.clone()
    }
    .boxed()
}

fn type_min<'a>(_env: &'a mut Env, args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move { args[0].get_type().min().map(Into::into) }.boxed()
}

fn type_max<'a>(_env: &'a mut Env, args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move { args[0].get_type().max().map(Into::into) }.boxed()
}

lazy_static! {
    pub static ref NUM_MUL: FunctionDefinition = FunctionDefinition {
        name_: "mul".to_string(),
        property: false,
        valid_args: vec![
            vec![FunctionParam::new("factor", Type::Uint(256))],
            vec![
                FunctionParam::new("factor", Type::Uint(256)),
                FunctionParam::new("decimals", Type::Uint(8))
            ],
        ],
        execute_fn: mul,
    };
    pub static ref NUM_DIV: FunctionDefinition = FunctionDefinition {
        name_: "div".to_string(),
        property: false,
        valid_args: vec![
            vec![FunctionParam::new("factor", Type::Uint(256))],
            vec![
                FunctionParam::new("factor", Type::Uint(256)),
                FunctionParam::new("decimals", Type::Uint(8))
            ],
        ],
        execute_fn: div,
    };
    pub static ref TYPE_MAX: FunctionDefinition = FunctionDefinition {
        name_: "max".to_string(),
        property: true,
        valid_args: vec![vec![],],
        execute_fn: type_max,
    };
    pub static ref TYPE_MIN: FunctionDefinition = FunctionDefinition {
        name_: "min".to_string(),
        property: true,
        valid_args: vec![vec![],],
        execute_fn: type_min,
    };
}
