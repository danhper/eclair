use alloy::{
    dyn_abi::DynSolValue,
    hex,
    primitives::{Address, B256, I256, U256},
};
use anyhow::{bail, Result};
use itertools::Itertools;
use std::{
    collections::BTreeMap,
    fmt::{self, Display, Formatter},
    ops::{Add, Div, Mul, Rem, Sub},
};

use super::{
    functions::Function,
    types::{ContractInfo, Receipt, Type},
};

#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Int(I256, usize),
    Uint(U256, usize),
    Str(String),
    FixBytes(B256, usize),
    Bytes(Vec<u8>),
    Addr(Address),
    Contract(ContractInfo, Address),
    Tuple(Vec<Value>),
    NamedTuple(String, BTreeMap<String, Value>),
    Array(Vec<Value>),
    TypeObject(Type),
    Transaction(B256),
    TransactionReceipt(Receipt),
    Func(Function),
}

fn _values_to_string(values: &[Value]) -> String {
    values.iter().map(|v| format!("{}", v)).join(", ")
}

fn _format_struct_fields(fields: &BTreeMap<String, Value>) -> String {
    fields
        .iter()
        .map(|(k, v)| format!("{}: {}", k, v))
        .join(", ")
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
            Value::Int(n, _) => write!(f, "{}", n),
            Value::Uint(n, _) => write!(f, "{}", n),
            Value::Addr(a) => write!(f, "{}", a.to_checksum(None)),
            Value::Str(s) => write!(f, "\"{}\"", s),
            Value::FixBytes(w, s) => {
                let bytes = w[..*s].to_vec();
                write!(f, "0x{}", hex::encode(bytes))
            }
            Value::Bytes(bytes) => write!(f, "0x{}", hex::encode(bytes)),
            Value::NamedTuple(name, v) => write!(f, "{} {{ {} }}", name, _format_struct_fields(v)),
            Value::Tuple(v) => write!(f, "({})", _values_to_string(v)),
            Value::Array(v) => write!(f, "[{}]", _values_to_string(v)),
            Value::TypeObject(t) => write!(f, "{}", t),
            Value::Transaction(t) => write!(f, "Transaction({})", t),
            Value::TransactionReceipt(r) => write!(f, "TransactionReceipt {{ {} }}", r),
            Value::Contract(ContractInfo(name, _), addr) => {
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
            Value::Int(n, s) => DynSolValue::Int(*n, *s),
            Value::Uint(n, s) => DynSolValue::Uint(*n, *s),
            Value::Str(s) => DynSolValue::String(s.clone()),
            Value::Addr(a) => DynSolValue::Address(*a),
            Value::FixBytes(w, s) => DynSolValue::FixedBytes(*w, *s),
            Value::Transaction(t) => DynSolValue::FixedBytes(*t, 32),
            Value::Bytes(b) => DynSolValue::Bytes(b.clone()),
            Value::Contract(_, addr) => DynSolValue::Address(*addr),
            Value::NamedTuple(name, vs) => {
                let prop_names = vs.iter().map(|(k, _)| k.clone()).collect();
                let tuple = vs
                    .iter()
                    .map(|(_, v)| DynSolValue::try_from(v))
                    .collect::<Result<Vec<_>>>()?;

                DynSolValue::CustomStruct {
                    name: name.clone(),
                    prop_names,
                    tuple,
                }
            }
            Value::Tuple(vs) => DynSolValue::Tuple(_values_to_dyn_sol_values(vs)?),
            Value::Array(vs) => DynSolValue::Array(_values_to_dyn_sol_values(vs)?),
            Value::Null => bail!("cannot convert null to Solidity type"),
            Value::TransactionReceipt(_) => bail!("cannot convert receipt to Solidity type"),
            Value::TypeObject(_) => bail!("cannot convert type objects to Solidity type"),
            Value::Func(_) => bail!("cannot convert function to Solidity type"),
        };
        Ok(v)
    }
}

impl From<alloy::rpc::types::Log> for Value {
    fn from(log: alloy::rpc::types::Log) -> Self {
        let mut fields = BTreeMap::new();
        fields.insert("address".to_string(), Value::Addr(log.address()));
        fields.insert(
            "topics".to_string(),
            Value::Array(
                log.topics()
                    .iter()
                    .map(|t| Value::FixBytes(*t, 32))
                    .collect(),
            ),
        );
        fields.insert("data".to_string(), Value::Bytes(log.data().data.to_vec()));

        Value::NamedTuple("Log".to_string(), fields)
    }
}

impl TryFrom<alloy::dyn_abi::DynSolValue> for Value {
    type Error = anyhow::Error;

    fn try_from(value: alloy::dyn_abi::DynSolValue) -> Result<Self, Self::Error> {
        match value {
            DynSolValue::Bool(b) => Ok(Value::Bool(b)),
            DynSolValue::Int(n, s) => Ok(Value::Int(n, s)),
            DynSolValue::Uint(n, s) => Ok(Value::Uint(n, s)),
            DynSolValue::String(s) => Ok(Value::Str(s)),
            DynSolValue::Address(a) => Ok(Value::Addr(a)),
            DynSolValue::FixedBytes(w, s) => Ok(Value::FixBytes(w, s)),
            DynSolValue::Bytes(v) => Ok(Value::Bytes(v)),
            DynSolValue::Tuple(vs) => _dyn_sol_values_to_values(vs).map(Value::Tuple),
            DynSolValue::Array(vs) => _dyn_sol_values_to_values(vs).map(Value::Array),
            DynSolValue::CustomStruct {
                name,
                prop_names,
                tuple,
            } => {
                let vs = prop_names
                    .into_iter()
                    .zip(tuple)
                    .map(|(k, dv)| Value::try_from(dv).map(|v| (k.clone(), v)))
                    .collect::<Result<Vec<_>>>()?;
                Ok(Value::NamedTuple(name, BTreeMap::from_iter(vs)))
            }
            v => Err(anyhow::anyhow!("{:?} not supported", v)),
        }
    }
}

impl From<i32> for Value {
    fn from(n: i32) -> Self {
        Value::Int(n.try_into().unwrap(), 256)
    }
}

impl From<u64> for Value {
    fn from(n: u64) -> Self {
        Value::Uint(U256::from(n), 256)
    }
}

impl From<u128> for Value {
    fn from(n: u128) -> Self {
        Value::Uint(U256::from(n), 256)
    }
}

impl<const N: usize> From<alloy::primitives::FixedBytes<N>> for Value {
    fn from(bytes: alloy::primitives::FixedBytes<N>) -> Self {
        Value::FixBytes(B256::from_slice(&bytes[..]), N)
    }
}

impl<T> From<&T> for Value
where
    T: Into<Value> + Clone,
{
    fn from(t: &T) -> Self {
        t.clone().into()
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a, _), Value::Int(b, _)) => a == b,
            (Value::Uint(a, _), Value::Uint(b, _)) => a == b,
            (Value::Int(a, _), Value::Uint(b, _)) => *a == I256::from_raw(*b),
            (Value::Uint(a, _), Value::Int(b, _)) => I256::from_raw(*a) == *b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Addr(a), Value::Addr(b)) => a == b,
            (Value::FixBytes(a, _), Value::FixBytes(b, _)) => a == b,
            (Value::Bytes(a), Value::Bytes(b)) => a == b,
            (Value::Tuple(a), Value::Tuple(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => a == b,
            (Value::TypeObject(a), Value::TypeObject(b)) => a == b,
            (Value::Contract(_, a), Value::Contract(_, b)) => a == b,
            _ => false,
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Value::Bool(a), Value::Bool(b)) => a.partial_cmp(b),
            (Value::Int(a, _), Value::Int(b, _)) => a.partial_cmp(b),
            (Value::Uint(a, _), Value::Uint(b, _)) => a.partial_cmp(b),
            (Value::Int(a, _), Value::Uint(b, _)) => a.partial_cmp(&I256::from_raw(*b)),
            (Value::Uint(a, _), Value::Int(b, _)) => I256::from_raw(*a).partial_cmp(b),
            (Value::Str(a), Value::Str(b)) => a.partial_cmp(b),
            (Value::Addr(a), Value::Addr(b)) => a.partial_cmp(b),
            (Value::FixBytes(a, _), Value::FixBytes(b, _)) => a.partial_cmp(b),
            (Value::Tuple(a), Value::Tuple(b)) => a.partial_cmp(b),
            (Value::Array(a), Value::Array(b)) => a.partial_cmp(b),
            (Value::Contract(_, a), Value::Contract(_, b)) => a.partial_cmp(b),
            _ => None,
        }
    }
}

impl Value {
    pub fn get_type(&self) -> Type {
        match self {
            Value::Bool(_) => Type::Bool,
            Value::Int(_, s) => Type::Int(*s),
            Value::Uint(_, s) => Type::Uint(*s),
            Value::Str(_) => Type::String,
            Value::Addr(_) => Type::Address,
            Value::FixBytes(_, s) => Type::FixBytes(*s),
            Value::Bytes(_) => Type::Bytes,
            Value::NamedTuple(name, vs) => Type::NamedTuple(
                name.clone(),
                vs.iter().map(|(k, v)| (k.clone(), v.get_type())).collect(),
            ),
            Value::Tuple(vs) => Type::Tuple(vs.iter().map(Value::get_type).collect()),
            Value::Array(vs) => {
                let t = vs.iter().map(Value::get_type).next().unwrap_or(Type::Bool);
                Type::Array(Box::new(t))
            }
            Value::Contract(c, _) => Type::Contract(c.clone()),
            Value::Null => Type::Null,
            Value::Func(_) => Type::Function,
            Value::TypeObject(type_ @ Type::Type(_)) => type_.clone(),
            Value::TypeObject(type_) => Type::Type(Box::new(type_.clone())),
            Value::Transaction(_) => Type::Transaction,
            Value::TransactionReceipt(_) => Type::TransactionReceipt,
        }
    }

    pub fn is_builtin(&self) -> bool {
        matches!(
            self,
            Value::TypeObject(Type::Console)
                | Value::TypeObject(Type::Repl)
                | Value::Func(Function::Builtin(_))
        )
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
            Value::Int(n, _) => Ok(n.as_i32()),
            Value::Uint(n, _) => Ok(n.to()),
            _ => bail!("cannot convert {} to i32", self.get_type()),
        }
    }

    pub fn as_usize(&self) -> Result<usize> {
        match self {
            Value::Int(n, _) => Ok(n.as_usize()),
            Value::Uint(n, _) => Ok(n.to()),
            _ => bail!("cannot convert {} to usize", self.get_type()),
        }
    }

    pub fn as_u64(&self) -> Result<u64> {
        match self {
            Value::Int(n, _) => Ok(n.as_u64()),
            Value::Uint(n, _) => Ok(n.to()),
            _ => bail!("cannot convert {} to u64", self.get_type()),
        }
    }

    pub fn get_field(&self, field: &str) -> Result<Value> {
        match self {
            Value::NamedTuple(_, fields) => fields
                .get(field)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("field {} not found", field)),
            _ => bail!("{} is not a struct", self.get_type()),
        }
    }

    pub fn get_items(&self) -> Result<Vec<Value>> {
        match self {
            Value::Array(items) => Ok(items.clone()),
            Value::Tuple(items) => Ok(items.clone()),
            Value::NamedTuple(_, items) => Ok(items.values().cloned().collect()),
            _ => bail!("{} is not iterable", self.get_type()),
        }
    }

    pub fn decimal_multiplier(decimals: u8) -> Value {
        Value::Uint(U256::from(10).pow(U256::from(decimals)), 256)
    }

    pub fn validate_int(self) -> Result<Self> {
        let type_ = self.get_type();
        let min_value = type_.min()?;
        let max_value = type_.max()?;
        if self < min_value || self > max_value {
            bail!("{} is out of range for {}", self, type_)
        }
        Ok(self)
    }
}

impl Add for Value {
    type Output = Result<Value>;

    fn add(self, other: Self) -> Self::Output {
        let error_msg = format!("cannot add {} and {}", self.get_type(), other.get_type());
        match (self, other) {
            (Value::Int(a, s1), Value::Int(b, s2)) => Ok(Value::Int(a + b, s1.max(s2))),
            (Value::Uint(a, s1), Value::Uint(b, s2)) => Ok(Value::Uint(a + b, s1.max(s2))),
            (Value::Int(a, s1), Value::Uint(b, s2)) => {
                Ok(Value::Int(a + I256::from_raw(b), s1.max(s2)))
            }
            (Value::Uint(a, s1), Value::Int(b, s2)) => {
                Ok(Value::Int(I256::from_raw(a) + b, s1.max(s2)))
            }
            (Value::Str(a), Value::Str(b)) => Ok(Value::Str(a + &b)),
            _ => bail!(error_msg),
        }
        .and_then(Value::validate_int)
    }
}

impl Sub for Value {
    type Output = Result<Value>;

    fn sub(self, other: Self) -> Self::Output {
        let error_msg = format!("cannot sub {} and {}", self.get_type(), other.get_type());
        match (self, other) {
            (Value::Int(a, s1), Value::Int(b, s2)) => Ok(Value::Int(a - b, s1.max(s2))),
            (Value::Uint(a, s1), Value::Uint(b, s2)) => Ok(Value::Uint(a - b, s1.max(s2))),
            (Value::Int(a, s1), Value::Uint(b, s2)) => {
                Ok(Value::Int(a - I256::from_raw(b), s1.max(s2)))
            }
            (Value::Uint(a, s1), Value::Int(b, s2)) => {
                Ok(Value::Int(I256::from_raw(a) - b, s1.max(s2)))
            }
            _ => bail!(error_msg),
        }
        .and_then(Value::validate_int)
    }
}

impl Mul for Value {
    type Output = Result<Value>;

    fn mul(self, other: Self) -> Self::Output {
        let error_msg = format!("cannot mul {} and {}", self.get_type(), other.get_type());
        match (self, other) {
            (Value::Int(a, s1), Value::Int(b, s2)) => Ok(Value::Int(a * b, s1.max(s2))),
            (Value::Uint(a, s1), Value::Uint(b, s2)) => Ok(Value::Uint(a * b, s1.max(s2))),
            (Value::Int(a, s1), Value::Uint(b, s2)) => {
                Ok(Value::Int(a * I256::from_raw(b), s1.max(s2)))
            }
            (Value::Uint(a, s1), Value::Int(b, s2)) => {
                Ok(Value::Int(I256::from_raw(a) * b, s1.max(s2)))
            }
            _ => bail!(error_msg),
        }
        .and_then(Value::validate_int)
    }
}

impl Div for Value {
    type Output = Result<Value>;

    fn div(self, other: Self) -> Self::Output {
        let error_msg = format!("cannot div {} and {}", self.get_type(), other.get_type());
        match (self, other) {
            (Value::Int(a, s1), Value::Int(b, s2)) => Ok(Value::Int(a / b, s1.max(s2))),
            (Value::Uint(a, s1), Value::Uint(b, s2)) => Ok(Value::Uint(a / b, s1.max(s2))),
            (Value::Int(a, s1), Value::Uint(b, s2)) => {
                Ok(Value::Int(a / I256::from_raw(b), s1.max(s2)))
            }
            (Value::Uint(a, s1), Value::Int(b, s2)) => {
                Ok(Value::Int(I256::from_raw(a) / b, s1.max(s2)))
            }
            _ => bail!(error_msg),
        }
        .and_then(Value::validate_int)
    }
}

impl Rem for Value {
    type Output = Result<Value>;

    fn rem(self, other: Self) -> Self::Output {
        let error_msg = format!("cannot rem {} and {}", self.get_type(), other.get_type());
        match (self, other) {
            (Value::Int(a, s1), Value::Int(b, s2)) => Ok(Value::Int(a % b, s1.max(s2))),
            (Value::Uint(a, s1), Value::Uint(b, s2)) => Ok(Value::Uint(a % b, s1.max(s2))),
            (Value::Int(a, s1), Value::Uint(b, s2)) => {
                Ok(Value::Int(a % I256::from_raw(b), s1.max(s2)))
            }
            (Value::Uint(a, s1), Value::Int(b, s2)) => {
                Ok(Value::Int(I256::from_raw(a) % b, s1.max(s2)))
            }
            _ => bail!(error_msg),
        }
        .and_then(Value::validate_int)
    }
}
