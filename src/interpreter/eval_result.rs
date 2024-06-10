use ethers::{
    types::{Address, U256},
    utils::{hex, keccak256},
};
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone)]
pub enum EvalResult {
    Empty,
    Uint(U256),
    Str(String),
    Addr(Address),
}

impl Display for EvalResult {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            EvalResult::Empty => write!(f, ""),
            EvalResult::Uint(n) => write!(f, "{}", n),
            EvalResult::Addr(a) => write!(f, "{}", to_checksum(a)),
            EvalResult::Str(s) => write!(f, "\"{}\"", s),
        }
    }
}

// https://github.com/gakonst/ethers-rs/blob/a41ae901e58fbe45e5ec037d57f6e75a85ef1cc9/ethers-core/src/utils/mod.rs#L267
fn to_checksum(addr: &Address) -> String {
    let hash = hex::encode(keccak256(addr));
    let addr_hex = hex::encode(addr.as_bytes());
    let addr_hex = addr_hex.as_bytes();

    addr_hex
        .iter()
        .zip(hash.as_bytes())
        .fold("0x".to_owned(), |mut encoded, (addr, c)| {
            encoded.push(if *c >= 56 {
                addr.to_ascii_uppercase() as char
            } else {
                addr.to_ascii_lowercase() as char
            });
            encoded
        })
}
