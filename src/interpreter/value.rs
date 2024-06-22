// use alloy::abi::Token;
use alloy::{
    json_abi::JsonAbi,
    primitives::{Address, U256},
};
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone)]
pub enum Value {
    Uint(U256),
    Str(String),
    Addr(Address),
    Contract(String, Address, JsonAbi),
}

unsafe impl std::marker::Send for Value {}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Value::Uint(n) => write!(f, "{}", n),
            Value::Addr(a) => write!(f, "{}", a.to_checksum(None)),
            Value::Str(s) => write!(f, "\"{}\"", s),
            Value::Contract(name, addr, _) => write!(f, "{}({})", name, addr.to_checksum(None)),
        }
    }
}

impl TryFrom<&Value> for alloy::dyn_abi::DynSolValue {
    type Error = anyhow::Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let v = match value {
            Value::Uint(n) => alloy::dyn_abi::DynSolValue::Uint(*n, 256),
            Value::Str(s) => alloy::dyn_abi::DynSolValue::String(s.clone()),
            Value::Addr(a) => alloy::dyn_abi::DynSolValue::Address(*a),
            Value::Contract(_, addr, _) => alloy::dyn_abi::DynSolValue::Address(*addr),
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
