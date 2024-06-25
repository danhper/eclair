use core::fmt;

use alloy::{
    primitives::{Address, I256, U256},
    providers::{Provider, RootProvider},
    transports::http::{Client, Http},
};
use anyhow::{bail, Result};

use super::Value;

fn common_to_decimals<T, F, G>(
    value: T,
    decimals: Option<i32>,
    precision: Option<i32>,
    to_f64: F,
    pow: G,
) -> String
where
    T: Copy + std::ops::Div<Output = T>,
    F: Fn(T) -> f64,
    G: Fn(u32) -> T,
{
    let decimals = decimals.unwrap_or(18);
    let precision = precision.unwrap_or(2);
    let result = if decimals > precision {
        let downscaled = value / pow((decimals - precision - 1) as u32);
        to_f64(downscaled) / 10f64.powi(precision + 1)
    } else {
        to_f64(value / pow(decimals as u32))
    };

    format!("{:.prec$}", result, prec = precision as usize)
}

fn uint_to_decimals(value: U256, decimals: Option<i32>, precision: Option<i32>) -> String {
    common_to_decimals(
        value,
        decimals,
        precision,
        |v: U256| v.to::<u64>() as f64,
        |exp| U256::from(10u64).pow(U256::from(exp)),
    )
}

fn int_to_decimals(value: I256, decimals: Option<i32>, precision: Option<i32>) -> String {
    common_to_decimals(
        value,
        decimals,
        precision,
        |v: I256| v.as_i64() as f64,
        |exp| I256::from_raw(U256::from(10u64).pow(U256::from(exp))),
    )
}

fn to_decimals<T, F>(value: T, args: &[Value], func: F) -> Result<String>
where
    F: Fn(T, Option<i32>, Option<i32>) -> String,
{
    let decimals = args.first().map(|v| v.to_i32()).transpose()?;
    let precision = args.get(1).map(|v| v.to_i32()).transpose()?;
    Ok(func(value, decimals, precision))
}

async fn get_balance(addr: Address, provider: &RootProvider<Http<Client>>) -> Result<U256> {
    Ok(provider.get_balance(addr).await?)
}

async fn concat_strings(string: String, args: &[Value]) -> Result<String> {
    if let Some(Value::Str(s)) = args.first() {
        Ok(format!("{}{}", string, s))
    } else {
        bail!("cannot concat {} with {:?}", string, args)
    }
}

#[derive(Debug, Clone)]
pub enum BuiltinMethod {
    Balance(Address),
    FormatU256(U256),
    FormatI256(I256),
    Concat(String),
}

impl fmt::Display for BuiltinMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Balance(addr) => write!(f, "{}.balance", addr),
            Self::FormatU256(n) => write!(f, "{}.format", n),
            Self::FormatI256(n) => write!(f, "{}.format", n),
            Self::Concat(s) => write!(f, "{}.concat", s),
        }
    }
}

impl BuiltinMethod {
    pub fn new(receiver: &Value, name: &str) -> Result<Self> {
        let method = match (receiver, name) {
            (Value::Addr(addr), "balance") => Self::Balance(*addr),
            (Value::Uint(n), "format") => Self::FormatU256(*n),
            (Value::Int(n), "format") => Self::FormatI256(*n),
            (Value::Str(s), "concat") => Self::Concat(s.clone()),
            _ => bail!("no method {} for type {}", name, receiver.get_type()),
        };
        Ok(method)
    }

    pub fn is_property(&self) -> bool {
        matches!(self, Self::Balance(_))
    }

    pub async fn execute(
        &self,
        args: &[Value],
        provider: &RootProvider<Http<Client>>,
    ) -> Result<Value> {
        match self {
            Self::Balance(addr) => Ok(Value::Uint(get_balance(*addr, provider).await?)),
            Self::FormatU256(n) => to_decimals(*n, args, uint_to_decimals).map(Value::Str),
            Self::FormatI256(n) => to_decimals(*n, args, int_to_decimals).map(Value::Str),
            Self::Concat(s) => concat_strings(s.clone(), args).await.map(Value::Str),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uint_to_decimals() {
        let value = U256::from(10).pow(U256::from(18));
        assert_eq!(uint_to_decimals(value, None, None), "1.00");

        assert_eq!(
            uint_to_decimals(U256::from(12348000), Some(6), None),
            "12.35"
        );
        assert_eq!(
            uint_to_decimals(U256::from(12348000), Some(6), Some(3)),
            "12.348"
        );
    }
}
