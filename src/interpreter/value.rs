// use alloy::abi::Token;
use alloy::{
    json_abi::JsonAbi,
    primitives::{Address, I256, U256},
};
use anyhow::bail;
use std::fmt::{self, Display, Formatter};

use super::functions::Function;

#[derive(Debug, Clone)]
pub struct ContractInfo(pub String, pub Address, pub JsonAbi);

#[derive(Debug, Clone)]
pub enum Value {
    Int(I256),
    Uint(U256),
    Str(String),
    Addr(Address),
    Contract(ContractInfo),
    Func(Function),
}

unsafe impl std::marker::Send for Value {}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Uint(n) => write!(f, "{}", n),
            Value::Addr(a) => write!(f, "{}", a.to_checksum(None)),
            Value::Str(s) => write!(f, "\"{}\"", s),
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
            Value::Int(n) => alloy::dyn_abi::DynSolValue::Int(*n, 256),
            Value::Uint(n) => alloy::dyn_abi::DynSolValue::Uint(*n, 256),
            Value::Str(s) => alloy::dyn_abi::DynSolValue::String(s.clone()),
            Value::Addr(a) => alloy::dyn_abi::DynSolValue::Address(*a),
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
            alloy::dyn_abi::DynSolValue::Uint(n, _) => Ok(Value::Uint(n)),
            alloy::dyn_abi::DynSolValue::String(s) => Ok(Value::Str(s)),
            alloy::dyn_abi::DynSolValue::Address(a) => Ok(Value::Addr(a)),
            v => Err(anyhow::anyhow!("{:?} not supported", v)),
        }
    }
}
