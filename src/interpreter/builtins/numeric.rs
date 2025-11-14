use std::sync::Arc;

use anyhow::{bail, Result};
use lazy_static::lazy_static;

use crate::interpreter::{
    functions::{FunctionDef, FunctionParam, SyncMethod, SyncProperty},
    Env, Type, Value,
};

fn mul_div_args(args: &[Value]) -> Result<(Value, u64)> {
    match args {
        [v2] => Ok((v2.clone(), 18)),
        [v2, d] => Ok((v2.clone(), d.as_u64()?)),
        _ => bail!("mul function expects one or two arguments"),
    }
}

fn mul(_env: &mut Env, receiver: &Value, args: &[Value]) -> Result<Value> {
    let (value, decimals) = mul_div_args(args)?;
    (receiver.clone() * value.clone())? / Value::decimal_multiplier(decimals as u8)
}

fn div(_env: &mut Env, receiver: &Value, args: &[Value]) -> Result<Value> {
    let (value, decimals) = mul_div_args(args)?;
    (receiver.clone() * Value::decimal_multiplier(decimals as u8))? / value.clone()
}

fn type_min(_env: &Env, receiver: &Value) -> Result<Value> {
    receiver.get_type().min()
}

fn type_max(_env: &Env, receiver: &Value) -> Result<Value> {
    receiver.get_type().max()
}

lazy_static! {
    pub static ref NUM_MUL: Arc<dyn FunctionDef> = SyncMethod::arc(
        "mul",
        mul,
        vec![
            vec![FunctionParam::new("factor", Type::Uint(256))],
            vec![
                FunctionParam::new("factor", Type::Uint(256)),
                FunctionParam::new("decimals", Type::Uint(8))
            ]
        ]
    );
    pub static ref NUM_DIV: Arc<dyn FunctionDef> = SyncMethod::arc(
        "div",
        div,
        vec![
            vec![FunctionParam::new("divisor", Type::Uint(256))],
            vec![
                FunctionParam::new("divisor", Type::Uint(256)),
                FunctionParam::new("decimals", Type::Uint(8))
            ]
        ]
    );
    pub static ref TYPE_MAX: Arc<dyn FunctionDef> = SyncProperty::arc("max", type_max);
    pub static ref TYPE_MIN: Arc<dyn FunctionDef> = SyncProperty::arc("max", type_min);
}
