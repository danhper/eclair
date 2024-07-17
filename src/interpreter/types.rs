use std::fmt::Display;

use alloy::{
    dyn_abi::DynSolType,
    json_abi::JsonAbi,
    primitives::{Address, B256, I256, U160, U256},
    rpc::types::{Log, TransactionReceipt},
};
use anyhow::{bail, Result};
use indexmap::IndexMap;
use itertools::Itertools;
use solang_parser::pt as parser;

use super::{
    block_functions::BlockFunction,
    function_definitions::{
        abi::{ABI_DECODE, ABI_DECODE_CALLDATA, ABI_ENCODE, ABI_ENCODE_PACKED},
        console::CONSOLE_LOG,
        numeric::{TYPE_MAX, TYPE_MIN},
    },
    functions::{ContractCall, Function, FunctionCall},
    Directive, Value,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashableIndexMap<K, V>(pub IndexMap<K, V>)
where
    K: Eq + std::hash::Hash,
    V: Eq;

impl<K, V> std::hash::Hash for HashableIndexMap<K, V>
where
    K: std::hash::Hash + Eq,
    V: std::hash::Hash + Eq,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.iter().for_each(|v| v.hash(state));
    }
}

impl<K, V> FromIterator<(K, V)> for HashableIndexMap<K, V>
where
    K: std::hash::Hash + Eq,
    V: Eq,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iterable: I) -> Self {
        Self(IndexMap::from_iter(iterable))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContractInfo(pub String, pub JsonAbi);

impl ContractInfo {
    pub fn create_call(&self, name: &str, addr: Address) -> Result<Function> {
        let _func = self
            .1
            .function(name)
            .ok_or_else(|| anyhow::anyhow!("function {} not found in contract {}", name, self.0))?;
        Ok(Function::ContractCall(ContractCall::new(
            self.clone(),
            addr,
            name.to_string(),
        )))
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Receipt {
    tx_hash: B256,
    block_hash: B256,
    block_number: u64,
    status: bool,
    gas_used: u128,
    effective_gas_price: u128,
    logs: Vec<Log>,
}

impl Receipt {
    pub fn get(&self, field: &str) -> Result<Value> {
        let result = match field {
            "tx_hash" => Value::FixBytes(self.tx_hash, 32),
            "block_hash" => Value::FixBytes(self.block_hash, 32),
            "block_number" => Value::Uint(U256::from(self.block_number), 256),
            "status" => Value::Bool(self.status),
            "gas_used" => Value::Uint(U256::from(self.gas_used), 256),
            "effective_gas_price" => Value::Uint(U256::from(self.effective_gas_price), 256),
            "logs" => Value::Array(
                self.logs.iter().map(|log| log.into()).collect(),
                Box::new(Type::FixBytes(32)),
            ),
            _ => bail!("receipt has no field {}", field),
        };
        Ok(result)
    }

    pub fn keys() -> Vec<String> {
        let keys = [
            "tx_hash",
            "block_hash",
            "block_number",
            "status",
            "gas_used",
            "effective_gas_price",
            "logs",
        ];
        keys.map(|s| s.to_string()).to_vec()
    }
}

impl From<TransactionReceipt> for Receipt {
    fn from(receipt: TransactionReceipt) -> Self {
        let logs = receipt.inner.logs().to_vec();
        Receipt {
            tx_hash: receipt.transaction_hash,
            block_hash: receipt.block_hash.unwrap_or_default(),
            block_number: receipt.block_number.unwrap_or(0),
            status: receipt.status(),
            gas_used: receipt.gas_used,
            effective_gas_price: receipt.effective_gas_price,
            logs,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Any,
    Null,
    Address,
    Bool,
    Int(usize),
    Uint(usize),
    FixBytes(usize),
    Bytes,
    String,
    Array(Box<Type>),
    FixedArray(Box<Type>, usize),
    NamedTuple(String, HashableIndexMap<String, Type>),
    Tuple(Vec<Type>),
    Mapping(Box<Type>, Box<Type>),
    Contract(ContractInfo),
    Transaction,
    TransactionReceipt,
    Function,
    Repl,
    Block,
    Console,
    Abi,
    Type(Box<Type>),
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Any => write!(f, "any"),
            Type::Null => write!(f, "null"),
            Type::Address => write!(f, "address"),
            Type::Bool => write!(f, "bool"),
            Type::Int(size) => write!(f, "int{}", size),
            Type::Uint(size) => write!(f, "uint{}", size),
            Type::FixBytes(size) => write!(f, "bytes{}", size),
            Type::Bytes => write!(f, "bytes"),
            Type::String => write!(f, "string"),
            Type::Array(t) => write!(f, "{}[]", t),
            Type::FixedArray(t, s) => write!(f, "{}[{}]", t, s),
            Type::NamedTuple(name, t) => {
                let items = t.0.iter().map(|(k, v)| format!("{}: {}", k, v)).join(", ");
                write!(f, "{} {{{}}}", name, items)
            }
            Type::Tuple(t) => {
                let items = t.iter().map(|v| format!("{}", v)).join(", ");
                write!(f, "({})", items)
            }
            Type::Mapping(k, v) => write!(f, "mapping({} => {})", k, v),
            Type::Contract(ContractInfo(name, _)) => write!(f, "{}", name),
            Type::Function => write!(f, "function"),

            Type::Transaction => write!(f, "transaction"),
            Type::TransactionReceipt => write!(f, "transactionReceipt"),

            Type::Repl => write!(f, "repl"),
            Type::Block => write!(f, "block"),
            Type::Console => write!(f, "console"),
            Type::Abi => write!(f, "abi"),
            Type::Type(t) => write!(f, "type({})", t),
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
            DynSolType::FixedArray(t, s) => {
                Type::FixedArray(Box::new(t.as_ref().clone().into()), s)
            }
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

impl TryFrom<parser::Type> for Type {
    type Error = anyhow::Error;

    fn try_from(type_: parser::Type) -> std::result::Result<Self, Self::Error> {
        match type_ {
            parser::Type::Address => Ok(Type::Address),
            parser::Type::Bool => Ok(Type::Bool),
            parser::Type::Int(size) => Ok(Type::Int(size as usize)),
            parser::Type::Uint(size) => Ok(Type::Uint(size as usize)),
            parser::Type::Bytes(size) => Ok(Type::FixBytes(size as usize)),
            parser::Type::DynamicBytes => Ok(Type::Bytes),
            parser::Type::String => Ok(Type::String),
            parser::Type::Function { .. } => Ok(Type::Function),
            parser::Type::Rational => Ok(Type::Uint(256)),
            parser::Type::AddressPayable => Ok(Type::Address),
            parser::Type::Mapping { key, value, .. } => match (key.as_ref(), value.as_ref()) {
                (parser::Expression::Type(_, k), parser::Expression::Type(_, v)) => {
                    Ok(Type::Mapping(
                        Box::new(k.clone().try_into()?),
                        Box::new(v.clone().try_into()?),
                    ))
                }
                (key, value) => {
                    bail!("unsupported mapping key {} and value {}", key, value)
                }
            },
            parser::Type::Payable { .. } => bail!("payable type is not supported yet"),
        }
    }
}

impl TryFrom<Type> for DynSolType {
    type Error = anyhow::Error;

    fn try_from(type_: Type) -> std::result::Result<Self, Self::Error> {
        match type_ {
            Type::Address => Ok(DynSolType::Address),
            Type::Bool => Ok(DynSolType::Bool),
            Type::Int(size) => Ok(DynSolType::Int(size)),
            Type::Uint(size) => Ok(DynSolType::Uint(size)),
            Type::FixBytes(size) => Ok(DynSolType::FixedBytes(size)),
            Type::Bytes => Ok(DynSolType::Bytes),
            Type::String => Ok(DynSolType::String),
            Type::Function => Ok(DynSolType::Function),
            Type::Array(t) => Ok(DynSolType::Array(Box::new((*t).try_into()?))),
            Type::FixedArray(t, s) => Ok(DynSolType::FixedArray(Box::new((*t).try_into()?), s)),
            Type::NamedTuple(name, fields) => Ok(DynSolType::CustomStruct {
                name,
                prop_names: fields.0.keys().cloned().collect(),
                tuple: fields
                    .0
                    .values()
                    .map(|t| t.clone().try_into())
                    .collect::<Result<Vec<_>>>()?,
            }),
            Type::Tuple(types) => Ok(DynSolType::Tuple(
                types
                    .into_iter()
                    .map(|t| t.try_into())
                    .collect::<Result<Vec<_>>>()?,
            )),
            _ => bail!("type {} is not supported", type_),
        }
    }
}

impl Type {
    pub fn default_value(&self) -> Result<Value> {
        let value = match self {
            Type::Null => Value::Null,
            Type::Address => Value::Addr(Address::ZERO),
            Type::Bool => Value::Bool(false),
            Type::Int(size) => Value::Int(I256::ZERO, *size),
            Type::Uint(size) => Value::Uint(U256::ZERO, *size),
            Type::FixBytes(size) => Value::FixBytes(B256::default(), *size),
            Type::Bytes => Value::Bytes(vec![]),
            Type::String => Value::Str("".to_string()),
            Type::Array(t) => Value::Array(vec![], t.clone()),
            Type::FixedArray(t, size) => Value::Array(vec![t.default_value()?; *size], t.clone()),
            Type::NamedTuple(_, fields) => Value::NamedTuple(
                "".to_string(),
                fields
                    .0
                    .iter()
                    .map(|(k, v)| v.default_value().map(|v_| (k.clone(), v_)))
                    .collect::<Result<HashableIndexMap<_, _>>>()?,
            ),
            Type::Tuple(types) => Value::Tuple(
                types
                    .iter()
                    .map(|t| t.default_value())
                    .collect::<Result<Vec<_>>>()?,
            ),
            Type::Mapping(kt, vt) => {
                Value::Mapping(HashableIndexMap(IndexMap::new()), kt.clone(), vt.clone())
            }
            _ => bail!("cannot get default value for type {}", self),
        };
        Ok(value)
    }

    pub fn is_int(&self) -> bool {
        matches!(self, Type::Int(_) | Type::Uint(_))
    }

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
        ]
    }

    pub fn cast(&self, value: &Value) -> Result<Value> {
        match (self, value) {
            (Type::Any, value) => Ok(value.clone()),
            (type_, value) if type_ == &value.get_type() => Ok(value.clone()),
            (Type::Contract(info), Value::Addr(addr)) => Ok(Value::Contract(info.clone(), *addr)),
            (Type::Address, Value::Contract(_, addr)) => Ok(Value::Addr(*addr)),
            (Type::Address, Value::Uint(v, _)) => Ok(Value::Addr(Address::from(v.to::<U160>()))),
            (Type::Address, Value::Int(v, _)) if v.is_zero() => Ok(Value::Addr(Address::ZERO)),
            (Type::Address, Value::Int(_, _)) => {
                bail!("cannot only cast cast zero address from int")
            }
            (Type::String, Value::Bytes(v)) => {
                Ok(Value::Str(String::from_utf8_lossy(v).to_string()))
            }
            (Type::Uint(size), Value::Int(v, _)) => {
                if v.is_negative() {
                    bail!("cannot cast negative int to uint")
                }
                Value::Uint((*v).try_into()?, *size).validate_int()
            }
            (Type::Uint(size), Value::Uint(v, _)) => Value::Uint(*v, *size).validate_int(),
            (Type::Int(size), Value::Int(v, _)) => Value::Int(*v, *size).validate_int(),
            (Type::Int(size), Value::Uint(v, _)) => {
                Value::Int((*v).try_into()?, *size).validate_int()
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
                let mut new_values = IndexMap::new();
                for (key, value) in types_.0.iter().zip(values.iter()) {
                    new_values.insert(key.0.clone(), key.1.cast(value)?);
                }
                Ok(Value::NamedTuple(
                    name.to_string(),
                    HashableIndexMap(new_values),
                ))
            }
            (Type::Array(t), Value::Array(v, _)) => v
                .iter()
                .map(|value| t.cast(value))
                .collect::<Result<Vec<_>>>()
                .map(|items| Value::Array(items, t.clone())),
            (Type::Tuple(types), Value::Tuple(v)) => v
                .iter()
                .zip(types.iter())
                .map(|(value, t)| t.cast(value))
                .collect::<Result<Vec<_>>>()
                .map(Value::Tuple),
            _ => bail!("cannot cast {} to {}", value.get_type(), self),
        }
    }

    pub fn functions(&self) -> Vec<String> {
        match self {
            Type::Contract(ContractInfo(_, abi)) => {
                abi.functions.keys().map(|s| s.to_string()).collect()
            }
            Type::NamedTuple(_, fields) => fields.0.keys().map(|s| s.to_string()).collect(),
            Type::Uint(_) | Type::Int(_) => {
                vec!["format".to_string(), "mul".to_string(), "div".to_string()]
            }
            Type::Transaction => vec!["getReceipt".to_string()],
            Type::TransactionReceipt => Receipt::keys(),
            Type::Array(_) => vec![
                "concat".to_string(),
                "length".to_string(),
                "map".to_string(),
            ],
            Type::String => vec!["concat".to_string(), "length".to_string()],

            Type::Bytes => vec!["concat".to_string(), "length".to_string()],

            Type::Mapping(_, _) => vec!["keys".to_string()],

            Type::Type(t) => match *t.clone() {
                Type::Contract(_) => vec!["decode".to_string()],
                Type::Abi => vec![
                    "encode".to_string(),
                    "decode".to_string(),
                    "encodePacked".to_string(),
                ],
                Type::Console => vec!["log".to_string()],
                Type::Block => BlockFunction::all(),
                Type::Repl => Directive::all()
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
                Type::Uint(_) | Type::Int(_) => vec!["max".to_string(), "min".to_string()],
                _ => vec![],
            },
            _ => vec![],
        }
    }

    pub fn max(&self) -> Result<Value> {
        let inner_type = match self {
            Type::Type(t) => t.as_ref(),
            _ => self,
        };
        let res = match inner_type {
            Type::Uint(256) => Value::Uint(U256::MAX, 256),
            Type::Uint(size) => {
                Value::Uint((U256::from(1) << U256::from(*size)) - U256::from(1), 256)
            }
            Type::Int(size) => Value::Int(
                I256::from_raw((U256::from(1) << U256::from(*size - 1)) - U256::from(1)),
                256,
            ),
            _ => bail!("cannot get max value for type {}", self),
        };
        Ok(res)
    }

    pub fn min(&self) -> Result<Value> {
        let inner_type = match self {
            Type::Type(t) => t.as_ref(),
            _ => self,
        };
        match inner_type {
            Type::Uint(_) => Ok(0u64.into()),
            Type::Int(256) => Ok(Value::Int(I256::MIN, 256)),
            Type::Int(size) => {
                let minus_one = -I256::from_raw(U256::from(1));
                Ok(Value::Int(
                    minus_one.asl(*size - 1).expect("min computation failed"),
                    256,
                ))
            }
            _ => bail!("cannot get min value for type {}", self),
        }
    }

    pub fn member_access(&self, member: &str) -> Result<Value> {
        let type_value = Value::TypeObject(self.clone());
        let f = match self {
            Type::Int(_) | Type::Uint(_) => match member {
                "max" => FunctionCall::method(&TYPE_MAX, &type_value),
                "min" => FunctionCall::method(&TYPE_MIN, &type_value),
                _ => bail!("{} does not have member {}", self, member),
            },

            Type::Contract(_) => match member {
                "decode" => FunctionCall::method(&ABI_DECODE_CALLDATA, &type_value),
                _ => bail!("{} does not have member {}", self, member),
            },

            Type::Abi => match member {
                "encode" => FunctionCall::function(&ABI_ENCODE),
                "encodePacked" => FunctionCall::function(&ABI_ENCODE_PACKED),
                "decode" => FunctionCall::function(&ABI_DECODE),
                _ => bail!("{} does not have member {}", self, member),
            },

            Type::Console => match member {
                "log" => FunctionCall::function(&CONSOLE_LOG),
                _ => bail!("{} does not have member {}", self, member),
            },

            _ => bail!("{} does not have member {}", self, member),
        };
        Ok(Value::Func(Function::Call(Box::new(f))))
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
            assert_eq!(t.max().unwrap(), Value::Uint(expected, 256));
        }
    }

    #[test]
    fn uint_min() {
        let cases = vec![
            (Type::Uint(256), 0u64),
            (Type::Uint(128), 0u64),
            (Type::Uint(64), 0u64),
            (Type::Uint(32), 0u64),
            (Type::Uint(8), 0u64),
        ];
        for (t, expected) in cases {
            assert_eq!(t.min().unwrap(), Value::Uint(U256::from(expected), 256));
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
            assert_eq!(t.max().unwrap(), Value::Int(expected, 256));
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
            assert_eq!(t.min().unwrap(), Value::Int(expected, 256));
        }
    }
}
