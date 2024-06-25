use std::fmt::Display;

use alloy::json_abi::JsonAbi;
use anyhow::{bail, Result};
use itertools::Itertools;

use super::{ContractInfo, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Address,
    Bool,
    Int(usize),
    Uint(usize),
    FixBytes(usize),
    Bytes,
    String,
    Array(Box<Type>),
    Tuple(Vec<Type>),
    Contract(String, JsonAbi),
    Function,
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Address => write!(f, "address"),
            Type::Bool => write!(f, "bool"),
            Type::Int(size) => write!(f, "int{}", size),
            Type::Uint(size) => write!(f, "uint{}", size),
            Type::FixBytes(size) => write!(f, "bytes{}", size),
            Type::Bytes => write!(f, "bytes"),
            Type::String => write!(f, "string"),
            Type::Array(t) => write!(f, "{}[]", t),
            Type::Tuple(t) => {
                let items = t.iter().map(|v| format!("{}", v)).join(", ");
                write!(f, "({})", items)
            }
            Type::Contract(name, _) => write!(f, "{}", name),
            Type::Function => write!(f, "function"),
        }
    }
}

impl TryFrom<solang_parser::pt::Type> for Type {
    type Error = anyhow::Error;

    fn try_from(type_: solang_parser::pt::Type) -> std::result::Result<Self, Self::Error> {
        match type_ {
            solang_parser::pt::Type::Address => Ok(Type::Address),
            solang_parser::pt::Type::Bool => Ok(Type::Bool),
            solang_parser::pt::Type::Int(size) => Ok(Type::Int(size as usize)),
            solang_parser::pt::Type::Uint(size) => Ok(Type::Uint(size as usize)),
            solang_parser::pt::Type::Bytes(size) => Ok(Type::FixBytes(size as usize)),
            solang_parser::pt::Type::DynamicBytes => Ok(Type::Bytes),
            solang_parser::pt::Type::String => Ok(Type::String),
            solang_parser::pt::Type::Function { .. } => Ok(Type::Function),
            solang_parser::pt::Type::Rational => Ok(Type::Uint(256)),
            solang_parser::pt::Type::AddressPayable => Ok(Type::Address),
            solang_parser::pt::Type::Mapping { .. } => bail!("mapping type is not supported yet"),
            solang_parser::pt::Type::Payable { .. } => bail!("payable type is not supported yet"),
        }
    }
}

impl Type {
    pub fn cast(&self, value: &Value) -> Result<Value> {
        match (self, value) {
            (Type::Contract(name, abi), Value::Addr(addr)) => Ok(Value::Contract(ContractInfo(
                name.clone(),
                *addr,
                abi.clone(),
            ))),
            (type_, value) if type_ == &value.get_type() => Ok(value.clone()),
            _ => bail!("cannot cast {} to {} (yet?)", value.get_type(), self),
        }
    }
}
