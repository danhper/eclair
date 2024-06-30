use std::{collections::BTreeMap, fmt::Display};

use alloy::{
    json_abi::JsonAbi,
    primitives::{I256, U256},
};
use anyhow::{bail, Result};
use itertools::Itertools;

use super::{block_functions::BlockFunction, ContractInfo, Directive, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Null,
    This,
    Address,
    Bool,
    Int(usize),
    Uint(usize),
    FixBytes(usize),
    Bytes,
    String,
    Array(Box<Type>),
    NamedTuple(String, BTreeMap<String, Type>),
    Tuple(Vec<Type>),
    Contract(String, JsonAbi),
    Function,
    Repl,
    Block,
    Console,
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Null => write!(f, "null"),
            Type::This => write!(f, "this"),
            Type::Address => write!(f, "address"),
            Type::Bool => write!(f, "bool"),
            Type::Int(size) => write!(f, "int{}", size),
            Type::Uint(size) => write!(f, "uint{}", size),
            Type::FixBytes(size) => write!(f, "bytes{}", size),
            Type::Bytes => write!(f, "bytes"),
            Type::String => write!(f, "string"),
            Type::Array(t) => write!(f, "{}[]", t),
            Type::NamedTuple(name, t) => {
                let items = t.iter().map(|(k, v)| format!("{}: {}", k, v)).join(", ");
                write!(f, "{} {{{}}}", name, items)
            }
            Type::Tuple(t) => {
                let items = t.iter().map(|v| format!("{}", v)).join(", ");
                write!(f, "({})", items)
            }
            Type::Contract(name, _) => write!(f, "{}", name),
            Type::Function => write!(f, "function"),

            Type::Repl => write!(f, "repl"),
            Type::Block => write!(f, "block"),
            Type::Console => write!(f, "console"),
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

    pub fn functions(&self) -> Vec<String> {
        match self {
            Type::Contract(_, abi) => abi.functions.keys().map(|s| s.to_string()).collect(),
            Type::Console => vec!["log".to_string()],
            Type::NamedTuple(_, fields) => fields.keys().map(|s| s.to_string()).collect(),
            Type::Repl => Directive::all(),
            Type::Block => BlockFunction::all(),
            Type::Uint(_) | Type::Int(_) => {
                vec!["format".to_string()]
            }
            Type::Array(_) => vec!["concat".to_string()],
            Type::String => vec!["concat".to_string()],
            _ => vec![],
        }
    }

    pub fn max(&self) -> Result<Value> {
        let res = match self {
            Type::Uint(256) => U256::MAX,
            Type::Uint(size) => (U256::from(1) << U256::from(*size)) - U256::from(1),
            Type::Int(size) => (U256::from(1) << U256::from(*size - 1)) - U256::from(1),
            _ => bail!("cannot get max value for type {}", self),
        };
        Ok(Value::Uint(res))
    }

    pub fn min(&self) -> Result<Value> {
        match self {
            Type::Uint(_) => Ok(0.into()),
            Type::Int(256) => Ok(Value::Int(I256::MIN)),
            Type::Int(size) => {
                let minus_one = -I256::from_raw(U256::from(1));
                Ok(Value::Int(
                    minus_one.asl(*size - 1).expect("min computation failed"),
                ))
            }
            _ => bail!("cannot get min value for type {}", self),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::interpreter::Value;
    use alloy::primitives::{I128, I256, I32, I64, I8, U128, U256, U32, U64, U8};

    use super::Type;

    #[test]
    fn uint_max() {
        let cases = vec![
            (Type::Uint(256), U256::MAX),
            (Type::Uint(128), U256::from(U128::MAX)),
            (Type::Uint(64), U256::from(U64::MAX)),
            (Type::Uint(32), U256::from(U32::MAX)),
            (Type::Uint(8), U256::from(U8::MAX)),
        ];
        for (t, expected) in cases {
            assert_eq!(t.max().unwrap(), Value::Uint(expected));
        }
    }

    #[test]
    fn uint_min() {
        let cases = vec![
            (Type::Uint(256), 0),
            (Type::Uint(128), 0),
            (Type::Uint(64), 0),
            (Type::Uint(32), 0),
            (Type::Uint(8), 0),
        ];
        for (t, expected) in cases {
            assert_eq!(t.min().unwrap(), Value::Uint(U256::from(expected)));
        }
    }

    #[test]
    fn int_max() {
        let cases = vec![
            (Type::Int(256), I256::MAX.to_string()),
            (Type::Int(128), I128::MAX.to_string()),
            (Type::Int(64), I64::MAX.to_string()),
            (Type::Int(32), I32::MAX.to_string()),
            (Type::Int(8), I8::MAX.to_string()),
        ];
        for (t, expected_str) in cases {
            let expected = I256::from_dec_str(&expected_str).unwrap();
            assert_eq!(t.max().unwrap(), Value::Int(expected));
        }
    }

    #[test]
    fn int_min() {
        let cases = vec![
            (Type::Int(256), I256::MIN.to_string()),
            (Type::Int(128), I128::MIN.to_string()),
            (Type::Int(64), I64::MIN.to_string()),
            (Type::Int(32), I32::MIN.to_string()),
            (Type::Int(8), I8::MIN.to_string()),
        ];
        for (t, expected_str) in cases {
            let expected = I256::from_dec_str(&expected_str).unwrap();
            assert_eq!(t.min().unwrap(), Value::Int(expected));
        }
    }
}
