use alloy::{
    hex,
    primitives::{I256, U256},
};
use anyhow::Result;
use futures::{future::BoxFuture, FutureExt};
use lazy_static::lazy_static;

use crate::interpreter::{
    function_definitions::{FunctionDefinition, FunctionParam},
    Env, Type, Value,
};

fn common_to_decimals<T, F, G>(
    value: T,
    decimals: Option<i32>,
    precision: Option<i32>,
    to_f64: F,
    pow: G,
) -> Result<String>
where
    T: Copy + std::ops::Div<Output = T>,
    F: Fn(T) -> Result<f64>,
    G: Fn(u32) -> T,
{
    let decimals = decimals.unwrap_or(18);
    let precision = precision.unwrap_or(2);
    let result = if decimals > precision {
        let downscaled = value / pow((decimals - precision - 1) as u32);
        match to_f64(downscaled) {
            Ok(res) => Ok(res / 10f64.powi(precision + 1)),
            _ => to_f64(value / pow(decimals as u32)),
        }
    } else {
        to_f64(value / pow(decimals as u32))
    };
    result.map(|result| format!("{:.prec$}", result, prec = precision as usize))
}

fn uint_to_decimals(value: U256, decimals: Option<i32>, precision: Option<i32>) -> Result<String> {
    common_to_decimals(
        value,
        decimals,
        precision,
        |v: U256| Ok(TryInto::<u64>::try_into(v).map(|v| v as f64)?),
        |exp| U256::from(10u64).pow(U256::from(exp)),
    )
}

fn int_to_decimals(value: I256, decimals: Option<i32>, precision: Option<i32>) -> Result<String> {
    common_to_decimals(
        value,
        decimals,
        precision,
        |v: I256| Ok(TryInto::<i64>::try_into(v).map(|v| v as f64)?),
        |exp| I256::from_raw(U256::from(10u64).pow(U256::from(exp))),
    )
}

fn to_decimals<T, F>(value: T, args: &[Value], func: F) -> Result<String>
where
    F: Fn(T, Option<i32>, Option<i32>) -> Result<String>,
{
    let decimals = args.first().map(|v| v.as_i32()).transpose()?;
    let precision = args.get(1).map(|v| v.as_i32()).transpose()?;
    func(value, decimals, precision)
}

fn format_bytes(bytes: &[u8]) -> String {
    let mut stripped_bytes = bytes;
    let last_0 = bytes.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
    if last_0 > 0 {
        stripped_bytes = &bytes[..last_0];
    }
    let is_diplayable = bytes.iter().all(|c| c.is_ascii());
    if is_diplayable {
        return String::from_utf8_lossy(stripped_bytes).to_string();
    } else {
        format!("0x{}", hex::encode(bytes))
    }
}

fn format(value: &Value, args: &[Value]) -> Result<String> {
    match value {
        Value::Uint(n, _) => to_decimals(*n, args, uint_to_decimals),
        Value::Int(n, _) => to_decimals(*n, args, int_to_decimals),
        Value::Str(s) => Ok(s.clone()),
        Value::Bytes(b) => Ok(format_bytes(b)),
        Value::FixBytes(b, _) => Ok(format_bytes(&b.0)),
        v => Ok(format!("{}", v)),
    }
}

fn async_format<'a>(_env: &'a mut Env, args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move { Ok(Value::Str(format(&args[0], &args[1..])?)) }.boxed()
}

lazy_static! {
    pub static ref NUM_FORMAT: FunctionDefinition = FunctionDefinition {
        name_: "format".to_string(),
        property: false,
        valid_args: vec![
            vec![],
            vec![FunctionParam::new("decimals", Type::Uint(8))],
            vec![
                FunctionParam::new("decimals", Type::Uint(8)),
                FunctionParam::new("precision", Type::Uint(8))
            ]
        ],
        execute_fn: async_format,
    };
    pub static ref NON_NUM_FORMAT: FunctionDefinition = FunctionDefinition {
        name_: "format".to_string(),
        property: false,
        valid_args: vec![vec![]],
        execute_fn: async_format,
    };
    pub static ref FORMAT_FUNCTION: FunctionDefinition = FunctionDefinition {
        name_: "format".to_string(),
        property: false,
        valid_args: vec![
            vec![FunctionParam::new("value", Type::Any)],
            vec![
                FunctionParam::new("value", Type::Any),
                FunctionParam::new("decimals", Type::Uint(8))
            ],
            vec![
                FunctionParam::new("value", Type::Any),
                FunctionParam::new("decimals", Type::Uint(8)),
                FunctionParam::new("precision", Type::Uint(8))
            ]
        ],
        execute_fn: async_format,
    };
}
