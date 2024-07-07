use std::{collections::BTreeMap, fmt::Display};

use alloy::{
    dyn_abi::DynSolType,
    json_abi::JsonAbi,
    primitives::{Address, B256, I256, U160, U256},
    rpc::types::TransactionReceipt,
};
use anyhow::{bail, Result};
use itertools::Itertools;

use super::{
    block_functions::BlockFunction,
    functions::{ContractCallMode, Function},
    Directive, Value,
};

#[derive(Debug, Clone, PartialEq)]
pub struct ContractInfo(pub String, pub JsonAbi);

impl ContractInfo {
    pub fn create_call(&self, name: &str, addr: Address) -> Result<Function> {
        let _func = self
            .1
            .function(name)
            .ok_or_else(|| anyhow::anyhow!("function {} not found in contract {}", name, self.0))?;
        Ok(Function::ContractCall(
            self.clone(),
            addr,
            name.to_string(),
            ContractCallMode::Default,
        ))
    }
}

#[derive(Debug, Clone)]
pub struct Receipt {
    tx_hash: B256,
    block_hash: B256,
    block_number: u64,
    status: bool,
    gas_used: u128,
    effective_gas_price: u128,
}

impl Receipt {
    pub fn get(&self, field: &str) -> Result<Value> {
        let result = match field {
            "tx_hash" => Value::FixBytes(self.tx_hash, 32),
            "block_hash" => Value::FixBytes(self.block_hash, 32),
            "block_number" => Value::Uint(U256::from(self.block_number)),
            "status" => Value::Bool(self.status),
            "gas_used" => Value::Uint(U256::from(self.gas_used)),
            "effective_gas_price" => Value::Uint(U256::from(self.effective_gas_price)),
            _ => bail!("receipt has no field {}", field),
        };
        Ok(result)
    }

    pub fn keys() -> Vec<String> {
        vec![
            "tx_hash".to_string(),
            "block_hash".to_string(),
            "block_number".to_string(),
            "status".to_string(),
            "gas_used".to_string(),
            "effective_gas_price".to_string(),
        ]
    }
}

impl From<TransactionReceipt> for Receipt {
    fn from(receipt: TransactionReceipt) -> Self {
        Receipt {
            tx_hash: receipt.transaction_hash,
            block_hash: receipt.block_hash.unwrap_or_default(),
            block_number: receipt.block_number.unwrap_or(0),
            status: receipt.status(),
            gas_used: receipt.gas_used,
            effective_gas_price: receipt.effective_gas_price,
        }
    }
}

impl Display for Receipt {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "tx_hash: {}, block_hash: {}, block_number: {}, status: {}, gas_used: {}, gas_price: {}",
            self.tx_hash, self.block_hash, self.block_number, self.status, self.gas_used, self.effective_gas_price
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Null,
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
    Contract(ContractInfo),
    Transaction,
    TransactionReceipt,
    Function,
    Repl,
    Block,
    Console,
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Null => write!(f, "null"),
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
            Type::Contract(ContractInfo(name, _)) => write!(f, "{}", name),
            Type::Function => write!(f, "function"),
            Type::Transaction => write!(f, "transaction"),
            Type::TransactionReceipt => write!(f, "transactionReceipt"),

            Type::Repl => write!(f, "repl"),
            Type::Block => write!(f, "block"),
            Type::Console => write!(f, "console"),
        }
    }
}

impl From<DynSolType> for Type {
    fn from(type_: DynSolType) -> Self {
        match type_ {
            DynSolType::Address => Type::Address,
            DynSolType::Bool => Type::Bool,
            DynSolType::Int(size) => Type::Int(size),
            DynSolType::Uint(size) => Type::Uint(size),
            DynSolType::Bytes => Type::Bytes,
            DynSolType::FixedBytes(s) => Type::FixBytes(s),
            DynSolType::String => Type::String,
            DynSolType::Function => Type::Function,
            DynSolType::Array(t) => Type::Array(Box::new(t.as_ref().clone().into())),
            DynSolType::FixedArray(t, _) => Type::Array(Box::new(t.as_ref().clone().into())),
            DynSolType::Tuple(types) => {
                Type::Tuple(types.iter().map(|t| Type::from(t.clone())).collect())
            }
            DynSolType::CustomStruct {
                name,
                prop_names,
                tuple,
            } => Type::NamedTuple(
                name,
                prop_names
                    .into_iter()
                    .zip(
                        tuple
                            .iter()
                            .map(|t| Type::from(t.clone()))
                            .collect::<Vec<_>>(),
                    )
                    .collect(),
            ),
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
    pub fn builtins() -> Vec<String> {
        vec![
            "address".to_string(),
            "bool".to_string(),
            "int8".to_string(),
            "int16".to_string(),
            "int32".to_string(),
            "int64".to_string(),
            "int128".to_string(),
            "int256".to_string(),
            "uint8".to_string(),
            "uint16".to_string(),
            "uint32".to_string(),
            "uint64".to_string(),
            "uint128".to_string(),
            "uint256".to_string(),
            "bytes".to_string(),
            "string".to_string(),
            "Transaction".to_string(),
        ]
    }

    pub fn cast(&self, value: &Value) -> Result<Value> {
        match (self, value) {
            (type_, value) if type_ == &value.get_type() => Ok(value.clone()),
            (Type::Contract(info), Value::Addr(addr)) => Ok(Value::Contract(info.clone(), *addr)),
            (Type::Address, Value::Contract(_, addr)) => Ok(Value::Addr(*addr)),
            (Type::Address, Value::Uint(v)) => Ok(Value::Addr(Address::from(v.to::<U160>()))),
            (Type::Address, Value::Int(v)) if v.is_zero() => Ok(Value::Addr(Address::ZERO)),
            (Type::Address, Value::Int(_)) => {
                bail!("cannot only cast cast zero address from int")
            }
            (Type::String, Value::Bytes(v)) => {
                Ok(Value::Str(String::from_utf8_lossy(v).to_string()))
            }
            (Type::Transaction, Value::FixBytes(v, 32)) => Ok(Value::Transaction(*v)),
            (Type::Bytes, Value::Str(v)) => Ok(Value::Bytes(v.as_bytes().to_vec())),
            (type_ @ Type::FixBytes(_), Value::Str(_)) => type_.cast(&Type::Bytes.cast(value)?),
            (Type::FixBytes(size), Value::Bytes(v)) => {
                let mut new_vector = v.clone();
                new_vector.resize(*size, 0);
                Ok(Value::FixBytes(B256::from_slice(&new_vector), *size))
            }
            (Type::NamedTuple(name, types_), Value::Tuple(values)) => {
                let mut new_values = BTreeMap::new();
                for (key, value) in types_.iter().zip(values.iter()) {
                    new_values.insert(key.0.clone(), key.1.cast(value)?);
                }
                Ok(Value::NamedTuple(name.to_string(), new_values))
            }
            (Type::Array(t), Value::Array(v)) => v
                .iter()
                .map(|value| t.cast(value))
                .collect::<Result<Vec<_>>>()
                .map(Value::Array),
            (Type::Tuple(types), Value::Tuple(v)) => v
                .iter()
                .zip(types.iter())
                .map(|(value, t)| t.cast(value))
                .collect::<Result<Vec<_>>>()
                .map(Value::Array),
            _ => bail!("cannot cast {} to {}", value.get_type(), self),
        }
    }

    pub fn functions(&self) -> Vec<String> {
        match self {
            Type::Contract(ContractInfo(_, abi)) => {
                abi.functions.keys().map(|s| s.to_string()).collect()
            }
            Type::Console => vec!["log".to_string()],
            Type::NamedTuple(_, fields) => fields.keys().map(|s| s.to_string()).collect(),
            Type::Repl => Directive::all()
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            Type::Block => BlockFunction::all(),
            Type::Uint(_) | Type::Int(_) => {
                vec!["format".to_string()]
            }
            Type::Transaction => vec!["getReceipt".to_string()],
            Type::TransactionReceipt => Receipt::keys(),
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
