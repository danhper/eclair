// use alloy::abi::Token;
use alloy::{
    dyn_abi::DynSolValue,
    hex,
    json_abi::JsonAbi,
    primitives::{Address, B256, I256, U256},
};
use anyhow::{bail, Result};
use itertools::Itertools;
use std::fmt::{self, Display, Formatter};

use super::{functions::Function, types::Type};

#[derive(Debug, Clone)]
pub struct ContractInfo(pub String, pub Address, pub JsonAbi);

#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Int(I256),
    Uint(U256),
    Str(String),
    FixBytes(B256, usize),
    Bytes(Vec<u8>),
    Addr(Address),
    Contract(ContractInfo),
    Tuple(Vec<Value>),
    Array(Vec<Value>),
    TypeObject(Type),
    Func(Function),
}

fn _values_to_string(values: &[Value]) -> String {
    values.iter().map(|v| format!("{}", v)).join(", ")
}

fn _values_to_dyn_sol_values(values: &[Value]) -> Result<Vec<DynSolValue>> {
    values.iter().map(DynSolValue::try_from).collect()
}

fn _dyn_sol_values_to_values(values: Vec<DynSolValue>) -> Result<Vec<Value>> {
    values.into_iter().map(Value::try_from).collect()
}

unsafe impl std::marker::Send for Value {}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Int(n) => write!(f, "{}", n),
            Value::Uint(n) => write!(f, "{}", n),
            Value::Addr(a) => write!(f, "{}", a.to_checksum(None)),
            Value::Str(s) => write!(f, "\"{}\"", s),
            Value::FixBytes(w, s) => {
                let bytes = w[..*s].to_vec();
                write!(f, "0x{}", hex::encode(bytes))
            }
            Value::Bytes(bytes) => write!(f, "0x{}", hex::encode(bytes)),
            Value::Tuple(v) => write!(f, "({})", _values_to_string(v)),
            Value::Array(v) => write!(f, "[{}]", _values_to_string(v)),
            Value::TypeObject(t) => write!(f, "{}", t),
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
            Value::Bool(b) => DynSolValue::Bool(*b),
            Value::Int(n) => DynSolValue::Int(*n, 256),
            Value::Uint(n) => DynSolValue::Uint(*n, 256),
            Value::Str(s) => DynSolValue::String(s.clone()),
            Value::Addr(a) => DynSolValue::Address(*a),
            Value::FixBytes(w, s) => DynSolValue::FixedBytes(*w, *s),
            Value::Bytes(b) => DynSolValue::Bytes(b.clone()),
            Value::Contract(ContractInfo(_, addr, _)) => DynSolValue::Address(*addr),
            Value::Tuple(vs) => DynSolValue::Tuple(_values_to_dyn_sol_values(vs)?),
            Value::Array(vs) => DynSolValue::Array(_values_to_dyn_sol_values(vs)?),
            Value::Null => bail!("cannot convert null to Solidity type"),
            Value::TypeObject(_) => bail!("cannot convert type objects to Solidity type"),
            Value::Func(_) => bail!("cannot convert function to Solidity type"),
        };
        Ok(v)
    }
}

impl TryFrom<alloy::dyn_abi::DynSolValue> for Value {
    type Error = anyhow::Error;

    fn try_from(value: alloy::dyn_abi::DynSolValue) -> Result<Self, Self::Error> {
        match value {
            DynSolValue::Bool(b) => Ok(Value::Bool(b)),
            DynSolValue::Uint(n, _) => Ok(Value::Uint(n)),
            DynSolValue::String(s) => Ok(Value::Str(s)),
            DynSolValue::Address(a) => Ok(Value::Addr(a)),
            DynSolValue::FixedBytes(w, s) => Ok(Value::FixBytes(w, s)),
            DynSolValue::Bytes(v) => Ok(Value::Bytes(v)),
            DynSolValue::Tuple(vs) => _dyn_sol_values_to_values(vs).map(Value::Tuple),
            DynSolValue::Array(vs) => _dyn_sol_values_to_values(vs).map(Value::Array),
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
            (Value::Bytes(a), Value::Bytes(b)) => a == b,
            (Value::Tuple(a), Value::Tuple(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => a == b,
            (Value::TypeObject(a), Value::TypeObject(b)) => a == b,
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
            (Value::Tuple(a), Value::Tuple(b)) => a.partial_cmp(b),
            (Value::Array(a), Value::Array(b)) => a.partial_cmp(b),
            (Value::Contract(ContractInfo(_, a, _)), Value::Contract(ContractInfo(_, b, _))) => {
                a.partial_cmp(b)
            }
            _ => None,
        }
    }
}

impl Value {
    pub fn get_type(&self) -> Type {
        match self {
            Value::Bool(_) => Type::Bool,
            Value::Int(_) => Type::Int(256),
            Value::Uint(_) => Type::Uint(256),
            Value::Str(_) => Type::String,
            Value::Addr(_) => Type::Address,
            Value::FixBytes(_, s) => Type::FixBytes(*s),
            Value::Bytes(_) => Type::Bytes,
            Value::Tuple(vs) => Type::Tuple(vs.iter().map(Value::get_type).collect()),
            Value::Array(vs) => {
                let t = vs.iter().map(Value::get_type).next().unwrap_or(Type::Bool);
                Type::Array(Box::new(t))
            }
            Value::Contract(ContractInfo(name, _, abi)) => {
                Type::Contract(name.clone(), abi.clone())
            }
            Value::Null => Type::Null,
            Value::Func(_) => Type::Function,
            Value::TypeObject(type_) => type_.clone(),
        }
    }

    pub fn as_address(&self) -> Result<Address> {
        match self {
            Value::Addr(addr) => Ok(*addr),
            _ => bail!("cannot convert {} to address", self.get_type()),
        }
    }

    pub fn as_string(&self) -> Result<String> {
        match self {
            Value::Str(str) => Ok(str.clone()),
            _ => bail!("cannot convert {} to string", self.get_type()),
        }
    }

    pub fn as_i32(&self) -> Result<i32> {
        match self {
            Value::Int(n) => Ok(n.as_i32()),
            Value::Uint(n) => Ok(n.to()),
            _ => bail!("cannot convert {} to i32", self.get_type()),
        }
    }

    pub fn as_usize(&self) -> Result<usize> {
        match self {
            Value::Int(n) => Ok(n.as_usize()),
            Value::Uint(n) => Ok(n.to()),
            _ => bail!("cannot convert {} to usize", self.get_type()),
        }
    }
}
