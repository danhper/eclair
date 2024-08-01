use alloy::{
    dyn_abi::DynSolValue,
    eips::{BlockId, BlockNumberOrTag},
    hex,
    primitives::{Address, B256, I256, U256},
};
use anyhow::{anyhow, bail, Result};
use indexmap::IndexMap;
use itertools::Itertools;
use std::{
    fmt::{self, Display, Formatter},
    ops::{Add, Div, Mul, Rem, Sub},
    str::FromStr,
};

use super::{
    builtins::{INSTANCE_METHODS, STATIC_METHODS, TYPE_METHODS},
    functions::Function,
    types::{ContractInfo, HashableIndexMap, Receipt, Type},
};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
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
    NamedTuple(String, HashableIndexMap<String, Value>),
    Array(Vec<Value>, Box<Type>),
    Mapping(HashableIndexMap<Value, Value>, Box<Type>, Box<Type>),
    TypeObject(Type),
    Transaction(B256),
    TransactionReceipt(Receipt),
    Func(Box<Function>),
}

fn _values_to_string(values: &[Value]) -> String {
    values.iter().map(|v| format!("{}", v)).join(", ")
}

fn _format_struct_fields(fields: &IndexMap<String, Value>) -> String {
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
            Value::NamedTuple(name, v) => {
                write!(f, "{} {{ {} }}", name, _format_struct_fields(&v.0))
            }
            Value::Tuple(v) => write!(f, "({})", _values_to_string(v)),
            Value::Array(v, _) => write!(f, "[{}]", _values_to_string(v)),
            Value::Mapping(v, kt, vt) => {
                let values = v.0.iter().map(|(k, v)| format!("{}: {}", k, v)).join(", ");
                write!(f, "mapping({} => {}) {{ {} }}", kt, vt, values)
            }
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
                let prop_names = vs.0.iter().map(|(k, _)| k.clone()).collect();
                let tuple =
                    vs.0.iter()
                        .map(|(_, v)| DynSolValue::try_from(v))
                        .collect::<Result<Vec<_>>>()?;

                DynSolValue::CustomStruct {
                    name: name.clone(),
                    prop_names,
                    tuple,
                }
            }
            Value::Tuple(vs) => DynSolValue::Tuple(_values_to_dyn_sol_values(vs)?),
            Value::Array(vs, _) => DynSolValue::Array(_values_to_dyn_sol_values(vs)?),
            Value::Mapping(_, _, _) => bail!("cannot convert mapping to Solidity type"),
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
        let mut fields = IndexMap::new();
        fields.insert("address".to_string(), Value::Addr(log.address()));
        fields.insert(
            "topics".to_string(),
            Value::Array(
                log.topics()
                    .iter()
                    .map(|t| Value::FixBytes(*t, 32))
                    .collect(),
                Box::new(Type::FixBytes(32)),
            ),
        );
        fields.insert("data".to_string(), Value::Bytes(log.data().data.to_vec()));

        Value::NamedTuple("Log".to_string(), HashableIndexMap(fields))
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
            DynSolValue::Array(vs) => _dyn_sol_values_to_values(vs).map(|xs| {
                let type_ = xs.first().map(Value::get_type).unwrap_or(Type::Any);
                Value::Array(xs, Box::new(type_))
            }),
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
                Ok(Value::NamedTuple(
                    name,
                    HashableIndexMap(IndexMap::from_iter(vs)),
                ))
            }
            v => Err(anyhow!("{:?} not supported", v)),
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

impl From<usize> for Value {
    fn from(n: usize) -> Self {
        Value::Uint(U256::from(n), 256)
    }
}

impl From<u128> for Value {
    fn from(n: u128) -> Self {
        Value::Uint(U256::from(n), 256)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::Str(s.to_string())
    }
}

impl From<Function> for Value {
    fn from(f: Function) -> Self {
        Value::Func(Box::new(f))
    }
}

impl From<BlockId> for Value {
    fn from(block_id: BlockId) -> Self {
        match block_id {
            BlockId::Hash(hash) => Value::FixBytes(hash.block_hash, 32),
            BlockId::Number(n) => match n {
                BlockNumberOrTag::Earliest => Value::Str("earliest".to_string()),
                BlockNumberOrTag::Latest => Value::Str("latest".to_string()),
                BlockNumberOrTag::Pending => Value::Str("pending".to_string()),
                BlockNumberOrTag::Number(n) => Value::Uint(U256::from(n), 256),
                BlockNumberOrTag::Finalized => Value::Str("finalized".to_string()),
                BlockNumberOrTag::Safe => Value::Str("safe".to_string()),
            },
        }
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

impl<T: Into<Value>> From<(T,)> for Value
where
    T: Into<Value>,
{
    fn from(t: (T,)) -> Self {
        Value::Tuple(vec![t.0.into()])
    }
}

impl<T: Into<Value>, U: Into<Value>> From<(T, U)> for Value
where
    T: Into<Value>,
{
    fn from(t: (T, U)) -> Self {
        Value::Tuple(vec![t.0.into(), t.1.into()])
    }
}

impl<T: Into<Value>, U: Into<Value>, V: Into<Value>> From<(T, U, V)> for Value
where
    T: Into<Value>,
{
    fn from(t: (T, U, V)) -> Self {
        Value::Tuple(vec![t.0.into(), t.1.into(), t.2.into()])
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
            (Value::Array(a, _), Value::Array(b, _)) => a.partial_cmp(b),
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
                vs.0.iter()
                    .map(|(k, v)| (k.clone(), v.get_type()))
                    .collect(),
            ),
            Value::Tuple(vs) => Type::Tuple(vs.iter().map(Value::get_type).collect()),
            Value::Array(_, t) => Type::Array(t.clone()),
            Value::Mapping(_, kt, vt) => Type::Mapping(kt.clone(), vt.clone()),
            Value::Contract(c, _) => Type::Contract(c.clone()),
            Value::Null => Type::Null,
            Value::Func(_) => Type::Function,
            Value::TypeObject(type_ @ Type::Type(_)) => type_.clone(),
            Value::TypeObject(type_) => Type::Type(Box::new(type_.clone())),
            Value::Transaction(_) => Type::Transaction,
            Value::TransactionReceipt(_) => Type::TransactionReceipt,
        }
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> Result<usize> {
        let len = match self {
            Value::Array(items, _) => items.len(),
            Value::Tuple(items) => items.len(),
            Value::Bytes(b) => b.len(),
            Value::Str(s) => s.len(),
            v => bail!("{} is not iterable", v.get_type()),
        };
        Ok(len)
    }

    pub fn set_index(&mut self, index: &Value, value: Value) -> Result<()> {
        match self {
            Value::Array(items, t) => {
                let int_index = index.as_usize()?;
                if int_index >= items.len() {
                    bail!("index out of bounds")
                }
                items[int_index] = t.cast(&value)?;
                Ok(())
            }
            Value::Mapping(v, kt, vt) => {
                v.0.insert(kt.cast(index)?, vt.cast(&value)?);
                Ok(())
            }
            _ => bail!("{} is not an array", self.get_type()),
        }
    }

    pub fn is_number(&self) -> bool {
        matches!(self, Value::Uint(..) | Value::Int(..))
    }

    pub fn is_builtin(&self) -> bool {
        matches!(
            self,
            Value::TypeObject(Type::Console) | Value::TypeObject(Type::Repl)
        )
    }

    pub fn as_address(&self) -> Result<Address> {
        match self {
            Value::Addr(addr) => Ok(*addr),
            _ => bail!("cannot convert {} to address", self.get_type()),
        }
    }

    pub fn as_contract(&self) -> Result<(ContractInfo, Address)> {
        match self {
            Value::Contract(info, addr) => Ok((info.clone(), *addr)),
            _ => bail!("cannot convert {} to contract", self.get_type()),
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

    pub fn as_u128(&self) -> Result<u128> {
        match self {
            Value::Uint(n, _) => Ok(n.to()),
            _ => bail!("cannot convert {} to u128", self.get_type()),
        }
    }

    pub fn as_u256(&self) -> Result<U256> {
        match self {
            Value::Uint(n, _) => Ok(n.to()),
            _ => bail!("cannot convert {} to u256", self.get_type()),
        }
    }

    pub fn as_block_id(&self) -> Result<BlockId> {
        match self {
            Value::FixBytes(hash, 32) => Ok(BlockId::Hash((*hash).into())),
            Value::Str(s) => BlockId::from_str(s).map_err(Into::into),
            n if n.is_number() => Ok(BlockId::number(n.as_u64()?)),
            _ => bail!("cannot convert {} to block id", self.get_type()),
        }
    }

    pub fn as_record(&self) -> Result<&HashableIndexMap<String, Value>> {
        match self {
            Value::NamedTuple(_, map) => Ok(map),
            _ => bail!("cannot convert {} to map", self.get_type()),
        }
    }

    pub fn get_field(&self, field: &str) -> Result<Value> {
        match self {
            Value::NamedTuple(_, fields) => fields
                .0
                .get(field)
                .cloned()
                .ok_or(anyhow!("field {} not found", field)),
            _ => bail!("{} is not a struct", self.get_type()),
        }
    }

    pub fn get_items(&self) -> Result<Vec<Value>> {
        match self {
            Value::Array(items, _) => Ok(items.clone()),
            Value::Tuple(items) => Ok(items.clone()),
            Value::NamedTuple(_, items) => Ok(items.0.values().cloned().collect()),
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

    pub fn member_access(&self, member: &str) -> Result<Value> {
        match self {
            Value::NamedTuple(_, kv) if kv.0.contains_key(member) => {
                Ok(kv.0.get(member).unwrap().clone())
            }
            Value::TransactionReceipt(r) if r.contains_key(member) => r.get(member),
            Value::Contract(c, addr) => c.make_function(member, *addr).map(Into::into),
            Value::Func(f) => f.member_access(member),
            _ => {
                let (type_, methods) = match self {
                    Value::TypeObject(Type::Type(type_)) => {
                        (type_.as_ref().clone(), TYPE_METHODS.get(&type_.into()))
                    }
                    Value::TypeObject(type_) => (type_.clone(), STATIC_METHODS.get(&type_.into())),
                    _ => (
                        self.get_type(),
                        INSTANCE_METHODS.get(&(&self.get_type()).into()),
                    ),
                };
                let func = methods.and_then(|m| m.get(member)).ok_or(anyhow!(
                    "{} does not have member {}",
                    type_,
                    member
                ))?;
                Ok(Function::method(func.clone(), self).into())
            }
        }
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
            (Value::Str(a), Value::Str(b)) => return Ok(Value::Str(a + &b)),
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
