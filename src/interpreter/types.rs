use std::fmt::Display;

use alloy::json_abi::JsonAbi;

#[derive(Debug, Clone)]
pub enum Type {
    Address,
    Bool,
    Int(u16),
    Uint(u16),
    Bytes,
    String,
    Array(Box<Type>),
    Tuple(Vec<Type>),
    Contract(String, JsonAbi),
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Address => write!(f, "address"),
            Type::Bool => write!(f, "bool"),
            Type::Int(size) => write!(f, "int{}", size),
            Type::Uint(size) => write!(f, "uint{}", size),
            Type::Bytes => write!(f, "bytes"),
            Type::String => write!(f, "string"),
            Type::Array(t) => write!(f, "{}[]", t),
            Type::Tuple(t) => {
                write!(f, "(")?;
                for (i, t) in t.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", t)?;
                }
                write!(f, ")")
            }
            Type::Contract(name, _) => write!(f, "{}", name),
        }
    }
}
