use ethers::{
    abi::Abi,
    types::{Address, U256},
    utils::to_checksum,
};
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone)]
pub enum Value {
    Uint(U256),
    Str(String),
    Addr(Address),
    Contract(String, Address, Abi),
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Value::Uint(n) => write!(f, "{}", n),
            Value::Addr(a) => write!(f, "{}", to_checksum(a, None)),
            Value::Str(s) => write!(f, "\"{}\"", s),
            Value::Contract(name, addr, _) => write!(f, "{}({})", name, to_checksum(addr, None)),
        }
    }
}
