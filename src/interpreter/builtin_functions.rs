use core::fmt;

use alloy::{
    dyn_abi::JsonAbiExt,
    hex,
    json_abi::JsonAbi,
    primitives::{Address, FixedBytes, I256, U256},
};
use anyhow::{bail, Result};
use futures::{future::BoxFuture, FutureExt};
use itertools::Itertools;

use super::{block_functions::BlockFunction, types::Type, Directive, Env, Value};

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

fn concat_strings(string: String, args: &[Value]) -> Result<String> {
    if let Some(Value::Str(s)) = args.first() {
        Ok(format!("{}{}", string, s))
    } else {
        bail!("cannot concat {} with {:?}", string, args)
    }
}

fn concat_arrays(arr: Vec<Value>, args: &[Value]) -> Result<Vec<Value>> {
    if let Some(Value::Array(other)) = args.first() {
        let mut new_arr = arr.clone();
        new_arr.extend(other.clone());
        Ok(new_arr)
    } else {
        bail!("cannot concat {:?} with {:?}", arr, args)
    }
}

fn concat(value: &Value, args: &[Value]) -> Result<Value> {
    match value {
        Value::Str(s) => concat_strings(s.clone(), args).map(Value::Str),
        Value::Array(arr) => concat_arrays(arr.clone(), args).map(Value::Array),
        _ => bail!("cannot concat {}", value),
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
            Type::Array(_) => Ok(Value::Array(values)),
            _ => bail!("cannot map to type {}", ty),
        }
    }
    .boxed()
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
        Value::Uint(n) => to_decimals(*n, args, uint_to_decimals),
        Value::Int(n) => to_decimals(*n, args, int_to_decimals),
        Value::Str(s) => Ok(s.clone()),
        Value::Bytes(b) => Ok(format_bytes(b)),
        Value::FixBytes(b, _) => Ok(format_bytes(&b.0)),
        v => Ok(format!("{}", v)),
    }
}

fn format_func(args: &[Value]) -> Result<String> {
    let receiver = args
        .first()
        .ok_or_else(|| anyhow::anyhow!("format function expects at least one argument"))?;
    format(receiver, &args[1..])
}

fn mul_div_args(args: &[Value]) -> Result<(Value, u64)> {
    match args {
        [v2] => Ok((v2.clone(), 18)),
        [v2, d] => Ok((v2.clone(), d.as_u64()?)),
        _ => bail!("mul function expects one or two arguments"),
    }
}

#[derive(Debug, Clone)]
pub enum BuiltinFunction {
    Balance(Address),
    FormatFunc,
    Format(Box<Value>),
    Mul(Box<Value>),
    Div(Box<Value>),
    Min(Type),
    Max(Type),
    Concat(Box<Value>),
    Decode(String, JsonAbi),
    Map(Vec<Value>, Type),
    Directive(Directive),
    GetType,
    Log,
    Block(BlockFunction),
}

impl fmt::Display for BuiltinFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Balance(addr) => write!(f, "{}.balance", addr),
            Self::Format(v) => write!(f, "{}.format", v),
            Self::Concat(s) => write!(f, "{}.concat", s),
            Self::Mul(v) => write!(f, "{}.mul", v),
            Self::Div(v) => write!(f, "{}.div", v),
            Self::Min(t) => write!(f, "{}.min", t),
            Self::Max(t) => write!(f, "{}.max", t),
            Self::Decode(name, _) => write!(f, "{}.decode(bytes)", name),
            Self::Map(v, _) => {
                let items = v.iter().map(|v| format!("{}", v)).join(", ");
                write!(f, "{}.map", items)
            }
            Self::GetType => write!(f, "type"),
            Self::FormatFunc => write!(f, "format"),
            Self::Directive(d) => write!(f, "repl.{}", d),
            Self::Block(func) => write!(f, "block.{}", func),
            Self::Log => write!(f, "console.log"),
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

    pub fn functions() -> Vec<String> {
        vec!["format".to_string(), "type".to_string()]
    }

    pub fn with_receiver(receiver: &Value, name: &str) -> Result<Self> {
        let method = match (receiver, name) {
            (Value::Addr(addr), "balance") => Self::Balance(*addr),

            (v, "format") => Self::Format(Box::new(v.clone())),

            (v @ (Value::Str(_) | Value::Array(_)), "concat") => Self::Concat(Box::new(v.clone())),

            (v @ Value::Uint(_) | v @ Value::Int(_), "mul") => Self::Mul(Box::new(v.clone())),
            (v @ Value::Uint(_) | v @ Value::Int(_), "div") => Self::Div(Box::new(v.clone())),

            (Value::Tuple(values), "map") => Self::Map(
                values.clone(),
                Type::Tuple(values.iter().map(Value::get_type).collect()),
            ),
            (Value::Array(values), "map") => {
                let arr_type = values.first().map_or(Type::Uint(256), Value::get_type);
                Self::Map(values.clone(), Type::Array(Box::new(arr_type)))
            }

            (Value::TypeObject(t @ Type::Int(_)) | Value::TypeObject(t @ Type::Uint(_)), "max") => {
                Self::Max(t.clone())
            }
            (Value::TypeObject(t @ Type::Int(_)) | Value::TypeObject(t @ Type::Uint(_)), "min") => {
                Self::Min(t.clone())
            }

            (Value::TypeObject(Type::Contract(name, abi)), "decode") => {
                Self::Decode(name.clone(), abi.clone())
            }
            (Value::TypeObject(Type::Repl), _) => {
                Directive::from_name(name).map(Self::Directive)?
            }
            (Value::TypeObject(Type::Block), _) => {
                BlockFunction::from_name(name).map(Self::Block)?
            }
            (Value::TypeObject(Type::Console), "log") => Self::Log,

            _ => bail!("no method {} for type {}", name, receiver.get_type()),
        };
        Ok(method)
    }

    pub fn is_property(&self) -> bool {
        match self {
            Self::Balance(_) | Self::Block(_) | Self::Min(_) | Self::Max(_) => true,
            Self::Directive(d) => d.is_property(),
            _ => false,
        }
    }

    pub async fn execute(&self, args: &[Value], env: &mut Env) -> Result<Value> {
        match self {
            Self::Balance(addr) => Ok(Value::Uint(env.get_provider().get_balance(*addr).await?)),
            Self::FormatFunc => format_func(args).map(Value::Str),
            Self::Format(v) => format(v, args).map(Value::Str),
            Self::Concat(v) => concat(v, args),

            Self::Mul(v) => {
                let (value, decimals) = mul_div_args(args)?;
                (v.as_ref().clone() * value.clone())? / Value::decimal_multiplier(decimals as u8)
            }
            Self::Div(v) => {
                let (value, decimals) = mul_div_args(args)?;
                (v.as_ref().clone() * Value::decimal_multiplier(decimals as u8))? / value.clone()
            }

            Self::Max(t) => t.max(),
            Self::Min(t) => t.min(),

            Self::Decode(name, abi) => decode_calldata(name, abi, args),
            Self::Map(values, type_) => {
                let result = map(values, type_.clone(), args, env).await?;
                Ok(result)
            }
            Self::GetType => get_type(args).map(Value::TypeObject),
            Self::Directive(d) => d.execute(args, env).await,
            Self::Block(f) => f.execute(args, env).await,
            Self::Log => {
                args.iter().for_each(|arg| println!("{}", arg));
                Ok(Value::Null)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uint_to_decimals() -> Result<()> {
        let value = U256::from(10).pow(U256::from(18));
        assert_eq!(uint_to_decimals(value, None, None)?, "1.00");

        assert_eq!(
            uint_to_decimals(U256::from(12348000), Some(6), None)?,
            "12.35"
        );
        assert_eq!(
            uint_to_decimals(U256::from(12348000), Some(6), Some(3))?,
            "12.348"
        );

        Ok(())
    }
}
