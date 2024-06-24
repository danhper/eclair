use std::fmt::Display;

use alloy::json_abi::JsonAbi;
use itertools::Itertools;

#[derive(Debug, Clone)]
pub enum Type {
    Address,
    Bool,
    Int(usize),
    Uint(usize),
    FixBytes(usize),
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
