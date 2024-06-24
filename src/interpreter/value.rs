// use alloy::abi::Token;
use alloy::{
    hex,
    json_abi::JsonAbi,
    primitives::{Address, B256, I256, U256},
};
use anyhow::bail;
use std::fmt::{self, Display, Formatter};

use super::functions::Function;

#[derive(Debug, Clone)]
pub struct ContractInfo(pub String, pub Address, pub JsonAbi);

#[derive(Debug, Clone)]
pub enum Value {
    Bool(bool),
    Int(I256),
    Uint(U256),
    Str(String),
    FixBytes(B256, usize),
    Addr(Address),
    Contract(ContractInfo),
    Func(Function),
}

unsafe impl std::marker::Send for Value {}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Value::Bool(b) => write!(f, "{}", b),
            Value::Int(n) => write!(f, "{}", n),
            Value::Uint(n) => write!(f, "{}", n),
            Value::Addr(a) => write!(f, "{}", a.to_checksum(None)),
            Value::Str(s) => write!(f, "\"{}\"", s),
            Value::FixBytes(w, s) => {
                let bytes = w[..*s].to_vec();
                write!(f, "0x{}", hex::encode(bytes))
            }
            Value::Contract(ContractInfo(name, addr, _)) => {
                write!(f, "{}({})", name, addr.to_checksum(None))
            }
            Value::Func(func) => write!(f, "{}", func),
        }
    }
}

impl TryFrom<&Value> for alloy::dyn_abi::DynSolValue {
    type Error = anyhow::Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let v = match value {
            Value::Bool(b) => alloy::dyn_abi::DynSolValue::Bool(*b),
            Value::Int(n) => alloy::dyn_abi::DynSolValue::Int(*n, 256),
            Value::Uint(n) => alloy::dyn_abi::DynSolValue::Uint(*n, 256),
            Value::Str(s) => alloy::dyn_abi::DynSolValue::String(s.clone()),
            Value::Addr(a) => alloy::dyn_abi::DynSolValue::Address(*a),
            Value::FixBytes(w, s) => alloy::dyn_abi::DynSolValue::FixedBytes(*w, *s),
            Value::Contract(ContractInfo(_, addr, _)) => {
                alloy::dyn_abi::DynSolValue::Address(*addr)
            }
            Value::Func(_) => bail!("cannot convert function to Solidity type"),
        };
        Ok(v)
    }
}

impl TryFrom<alloy::dyn_abi::DynSolValue> for Value {
    type Error = anyhow::Error;

    fn try_from(value: alloy::dyn_abi::DynSolValue) -> Result<Self, Self::Error> {
        match value {
            alloy::dyn_abi::DynSolValue::Bool(b) => Ok(Value::Bool(b)),
            alloy::dyn_abi::DynSolValue::Uint(n, _) => Ok(Value::Uint(n)),
            alloy::dyn_abi::DynSolValue::String(s) => Ok(Value::Str(s)),
            alloy::dyn_abi::DynSolValue::Address(a) => Ok(Value::Addr(a)),
            alloy::dyn_abi::DynSolValue::FixedBytes(w, s) => Ok(Value::FixBytes(w, s)),
            v => Err(anyhow::anyhow!("{:?} not supported", v)),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Uint(a), Value::Uint(b)) => a == b,
            (Value::Int(a), Value::Uint(b)) => *a == I256::from_raw(*b),
            (Value::Uint(a), Value::Int(b)) => I256::from_raw(*a) == *b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Addr(a), Value::Addr(b)) => a == b,
            (Value::FixBytes(a, _), Value::FixBytes(b, _)) => a == b,
            (Value::Contract(ContractInfo(_, a, _)), Value::Contract(ContractInfo(_, b, _))) => {
                a == b
            }
            _ => false,
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Value::Bool(a), Value::Bool(b)) => a.partial_cmp(b),
            (Value::Int(a), Value::Int(b)) => a.partial_cmp(b),
            (Value::Uint(a), Value::Uint(b)) => a.partial_cmp(b),
            (Value::Int(a), Value::Uint(b)) => a.partial_cmp(&I256::from_raw(*b)),
            (Value::Uint(a), Value::Int(b)) => I256::from_raw(*a).partial_cmp(b),
            (Value::Str(a), Value::Str(b)) => a.partial_cmp(b),
            (Value::Addr(a), Value::Addr(b)) => a.partial_cmp(b),
            (Value::FixBytes(a, _), Value::FixBytes(b, _)) => a.partial_cmp(b),
            (Value::Contract(ContractInfo(_, a, _)), Value::Contract(ContractInfo(_, b, _))) => {
                a.partial_cmp(b)
            }
            _ => None,
        }
    }
}
