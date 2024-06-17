use ethers::abi::Token;
use ethers::{
    abi::{Abi, Tokenizable},
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

impl Tokenizable for Value {
    fn into_token(self) -> ethers::abi::Token {
        match self {
            Value::Uint(n) => Token::Uint(n.clone()),
            Value::Str(s) => Token::String(s.clone()),
            Value::Addr(a) => Token::Address(a.clone()),
            Value::Contract(name, addr, _) => panic!("Cannot serialize contract value"),
        }
    }

    fn from_token(token: ethers::abi::Token) -> Result<Self, ethers::abi::InvalidOutputType>
    where
        Self: Sized,
    {
        match token {
            Token::Uint(n) => Ok(Value::Uint(n)),
            Token::String(s) => Ok(Value::Str(s)),
            Token::Address(a) => Ok(Value::Addr(a)),
            _ => Err(ethers::abi::InvalidOutputType("not valid".to_string())),
        }
    }
}
