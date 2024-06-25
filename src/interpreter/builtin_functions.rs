use core::fmt;

use alloy::{
    dyn_abi::JsonAbiExt,
    json_abi::JsonAbi,
    primitives::{Address, FixedBytes, I256, U256},
    providers::{Provider, RootProvider},
    transports::http::{Client, Http},
};
use anyhow::{bail, Result};
use futures::{future::BoxFuture, FutureExt};
use itertools::Itertools;

use super::{types::Type, Env, Value};

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

fn get_type(args: &[Value]) -> Result<Type> {
    if let [arg] = args {
        Ok(arg.get_type())
    } else {
        bail!("type function expects one argument")
    }
}

fn decode_calldata(name: &str, abi: &JsonAbi, args: &[Value]) -> Result<Value> {
    let data = match args.first() {
        Some(Value::Bytes(data)) => data,
        _ => bail!("decode function expects bytes as an argument"),
    };
    let selector = FixedBytes::<4>::from_slice(&data[..4]);
    let function = abi
        .functions()
        .find(|f| f.selector() == selector)
        .ok_or(anyhow::anyhow!(
            "function with selector {} not found for {}",
            selector,
            name
        ))?;
    let decoded = function.abi_decode_input(&data[4..], true)?;
    let values = decoded
        .into_iter()
        .map(Value::try_from)
        .collect::<Result<Vec<_>>>()?;
    Ok(Value::Tuple(values))
}

fn map<'a>(
    target: Vec<Value>,
    ty: Type,
    args: Vec<Value>,
    env: &'a mut Env,
    provider: &'a RootProvider<Http<Client>>,
) -> BoxFuture<'a, Result<Value>> {
    async move {
        let func = if let [Value::Func(f)] = &args[..] {
            f
        } else {
            bail!("map function expects a single function as an argument")
        };
        let mut values = vec![];
        for v in target {
            let value = func.execute(&[v], env, provider).await?;
            values.push(value);
        }
        match ty {
            Type::Tuple(_) => Ok(Value::Tuple(values)),
            Type::Array(_) => Ok(Value::Array(values)),
            _ => bail!("cannot map to type {}", ty),
        }
    }
    .boxed()
}

fn format(value: &Value, args: &[Value]) -> Result<String> {
    match value {
        Value::Uint(n) => to_decimals(*n, args, uint_to_decimals),
        Value::Int(n) => to_decimals(*n, args, int_to_decimals),
        Value::Str(s) => Ok(s.clone()),
        _ => bail!("cannot format {}", value),
    }
}

fn format_func(args: &[Value]) -> Result<String> {
    let receiver = args
        .first()
        .ok_or_else(|| anyhow::anyhow!("format function expects at least one argument"))?;
    format(receiver, &args[1..])
}

#[derive(Debug, Clone)]
pub enum BuiltinFunction {
    Balance(Address),
    FormatFunc,
    Format(Box<Value>),
    Concat(String),
    Decode(String, JsonAbi),
    Map(Vec<Value>, Type),
    GetType,
}

impl fmt::Display for BuiltinFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Balance(addr) => write!(f, "{}.balance", addr),
            Self::Format(v) => write!(f, "{}.format", v),
            Self::Concat(s) => write!(f, "{}.concat", s),
            Self::Decode(name, _) => write!(f, "{}.decode(bytes)", name),
            Self::Map(v, _) => {
                let items = v.iter().map(|v| format!("{}", v)).join(", ");
                write!(f, "{}.map", items)
            }
            Self::GetType => write!(f, "type"),
            Self::FormatFunc => write!(f, "format"),
        }
    }
}

impl BuiltinFunction {
    pub fn from_name(name: &str) -> Result<Self> {
        match name {
            "format" => Ok(Self::FormatFunc),
            "type" => Ok(Self::GetType),
            _ => bail!("no function {}", name),
        }
    }

    pub fn with_receiver(receiver: &Value, name: &str) -> Result<Self> {
        let method = match (receiver, name) {
            (Value::Addr(addr), "balance") => Self::Balance(*addr),
            (v, "format") => Self::Format(Box::new(v.clone())),
            (Value::Str(s), "concat") => Self::Concat(s.clone()),
            (Value::TypeObject(Type::Contract(name, abi)), "decode") => {
                Self::Decode(name.clone(), abi.clone())
            }
            (Value::Tuple(values), "map") => Self::Map(
                values.clone(),
                Type::Tuple(values.iter().map(Value::get_type).collect()),
            ),
            (Value::Array(values), "map") => Self::Map(
                values.clone(),
                values
                    .first()
                    .map(Value::get_type)
                    .unwrap_or(Type::Uint(256)),
            ),
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
        env: &mut Env,
        provider: &RootProvider<Http<Client>>,
    ) -> Result<Value> {
        match self {
            Self::Balance(addr) => Ok(Value::Uint(get_balance(*addr, provider).await?)),
            Self::FormatFunc => format_func(args).map(Value::Str),
            Self::Format(v) => format(v, args).map(Value::Str),
            Self::Concat(s) => concat_strings(s.clone(), args).await.map(Value::Str),
            Self::Decode(name, abi) => decode_calldata(name, abi, args),
            Self::Map(values, type_) => {
                let result = map(
                    values.clone(),
                    type_.clone(),
                    Vec::from(args),
                    env,
                    provider,
                )
                .await?;
                Ok(result)
            }
            Self::GetType => get_type(args).map(Value::TypeObject),
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
