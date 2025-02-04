use alloy::{
    dyn_abi::DynSolValue,
    eips::{BlockId, BlockNumberOrTag},
    hex::{self, FromHex},
    primitives::{Address, B256, I256, U256},
    rpc::types::TransactionReceipt,
};
use anyhow::{anyhow, bail, Result};
use indexmap::IndexMap;
use itertools::Itertools;
use serde::{ser::SerializeStruct, Serialize};
use std::{
    fmt::{self, Display, Formatter},
    ops::{Add, BitAnd, BitOr, BitXor, Div, Mul, Rem, Shl, Shr, Sub},
    str::FromStr,
    u64,
};

use super::{
    builtins::{INSTANCE_METHODS, STATIC_METHODS, TYPE_METHODS},
    functions::Function,
    types::{ArrayIndex, ContractInfo, HashableIndexMap, Type, LOG_TYPE},
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
                let bytes = w[0..*s].to_vec();
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
            Value::TypeObject(_) => bail!("cannot convert type objects to Solidity type"),
            Value::Func(_) => bail!("cannot convert function to Solidity type"),
        };
        Ok(v)
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Value::Null => serializer.serialize_unit(),
            Value::Bool(b) => serializer.serialize_bool(*b),
            Value::Int(n, _) => {
                if n.ge(&I256::try_from(i64::MIN).unwrap())
                    && n.le(&I256::try_from(i64::MAX).unwrap())
                {
                    serializer.serialize_i64(n.as_i64())
                } else {
                    serializer.serialize_str(&n.to_string())
                }
            }
            Value::Uint(n, _) => {
                if n.le(&U256::from(u64::MAX)) {
                    serializer.serialize_u64(n.to())
                } else {
                    serializer.serialize_str(&n.to_string())
                }
            }
            Value::Str(s) => serializer.serialize_str(s),
            Value::FixBytes(w, s) => {
                let bytes = w[0..*s].to_vec();
                serializer.serialize_str(&format!("0x{}", hex::encode(bytes)))
            }
            Value::Bytes(bytes) => serializer.serialize_str(&format!("0x{}", hex::encode(bytes))),
            Value::Addr(a) => serializer.serialize_str(&a.to_checksum(None)),
            Value::Contract(ContractInfo(_name, _), addr) => {
                serializer.serialize_str(&addr.to_checksum(None))
            }
            Value::Tuple(v) => v.serialize(serializer),
            Value::NamedTuple(name, v) => {
                let mut state = serializer.serialize_struct(name.clone().leak(), v.0.len())?;
                for (k, v) in v.0.iter() {
                    state.serialize_field(k.clone().leak(), v)?;
                }
                state.end()
            }
            Value::Array(v, _) => v.serialize(serializer),
            Value::Mapping(v, _, _) => v.0.serialize(serializer),
            Value::TypeObject(t) => serializer.serialize_str(&format!("{}", t)),
            Value::Transaction(t) => serializer.serialize_str(&format!("0x{}", hex::encode(t))),
            Value::Func(func) => serializer.serialize_str(&format!("{}", func)),
        }
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

impl TryFrom<Vec<alloy::dyn_abi::DynSolValue>> for Value {
    type Error = anyhow::Error;

    fn try_from(value: Vec<alloy::dyn_abi::DynSolValue>) -> std::result::Result<Self, Self::Error> {
        let values = value
            .into_iter()
            .map(Value::try_from)
            .collect::<Result<Vec<_>>>()?;
        Ok(Value::Tuple(values))
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

impl FromHex for Value {
    type Error = anyhow::Error;

    fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self> {
        let result = if hex.as_ref().len() % 2 == 1 {
            bail!("hex number literal must have an even number of digits")
        } else if hex.as_ref().len() == 42 {
            Value::Addr(Address::from_hex(hex)?)
        } else if hex.as_ref().len() <= 66 {
            let data = Vec::from_hex(&hex.as_ref()[2..])?;
            let mut bytes = vec![0; 32];
            bytes[..data.len()].copy_from_slice(&data);
            Value::FixBytes(B256::from_slice(&bytes), (hex.as_ref().len() - 2) / 2)
        } else {
            Value::Bytes(Vec::from_hex(&hex.as_ref()[2..])?)
        };
        Ok(result)
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

    pub fn at(&self, index: &Value) -> Result<Value> {
        match self {
            Value::Array(items, _t) => {
                let int_index = index.as_usize()?;
                if int_index >= items.len() {
                    bail!("index out of bounds")
                }
                Ok(items[int_index].clone())
            }
            Value::Tuple(items) => {
                let int_index = index.as_usize()?;
                if int_index >= items.len() {
                    bail!("index out of bounds")
                }
                Ok(items[int_index].clone())
            }
            Value::Mapping(v, kt, _vt) => {
                v.0.get(&kt.cast(index)?)
                    .cloned()
                    .ok_or(anyhow!("key not found: {}", index))
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
            Value::TypeObject(Type::Console)
                | Value::TypeObject(Type::Repl)
                | Value::TypeObject(Type::Events)
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

    pub fn as_type(&self) -> Result<Type> {
        match self {
            Value::TypeObject(type_) => Ok(type_.clone()),
            _ => bail!("cannot convert {} to type", self.get_type()),
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
            Value::Int(n, _) => {
                if n.is_negative() {
                    bail!("negative number")
                } else {
                    Ok(n.as_usize())
                }
            }
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

    pub fn as_b256(&self) -> Result<B256> {
        match Type::FixBytes(32).cast(self) {
            Ok(Value::FixBytes(n, 32)) => Ok(n),
            _ => bail!("cannot convert {} to bytes32", self.get_type()),
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
            Value::Contract(c, addr) => c.make_function(member, *addr).map(Into::into),
            Value::Func(f) => f.member_access(member),
            Value::TypeObject(Type::Contract(c)) => c.member_access(member),
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

    pub fn slice(&self, start: Option<ArrayIndex>, end: Option<ArrayIndex>) -> Result<Value> {
        let length = self.len()?;
        let start = start.unwrap_or(ArrayIndex(0)).get_index(length)?;
        let end = match end {
            Some(end) => end.get_index(length)?,
            None => length,
        };

        match self {
            Value::Array(items, t) => {
                let items = items[start..end].to_vec();
                Ok(Value::Array(items, t.clone()))
            }
            Value::Bytes(bytes) => {
                let bytes = bytes[start..end].to_vec();
                Ok(Value::Bytes(bytes))
            }
            Value::Str(s) => {
                let s = s.chars().skip(start).take(end - start).collect();
                Ok(Value::Str(s))
            }
            _ => bail!("{} is not sliceable", self.get_type()),
        }
    }

    pub fn from_receipt(receipt: TransactionReceipt, parsed_logs: Vec<Value>) -> Self {
        let fields = IndexMap::from([
            (
                "txHash".to_string(),
                Value::FixBytes(receipt.transaction_hash, 32),
            ),
            (
                "blockHash".to_string(),
                Value::FixBytes(receipt.block_hash.unwrap_or_default(), 32),
            ),
            (
                "blockNumber".to_string(),
                receipt.block_number.unwrap_or(0u64).into(),
            ),
            ("status".to_string(), Value::Bool(receipt.status())),
            ("gasUsed".to_string(), receipt.gas_used.into()),
            ("gasPrice".to_string(), receipt.effective_gas_price.into()),
            (
                "logs".to_string(),
                Value::Array(parsed_logs, Box::new(LOG_TYPE.clone())),
            ),
        ]);

        Value::NamedTuple("Receipt".to_string(), HashableIndexMap(fields))
    }

    fn apply_operation<F1, F2>(self, other: Self, iop: F1, uop: F2, op_name: &str) -> Result<Value>
    where
        F1: Fn(I256, I256) -> I256,
        F2: Fn(U256, U256) -> U256,
    {
        let error_msg = format!(
            "cannot {} {} and {}",
            op_name,
            self.get_type(),
            other.get_type()
        );
        match (self, other) {
            (Value::Int(a, s1), Value::Int(b, s2)) => Ok(Value::Int(iop(a, b), s1.max(s2))),
            (Value::Uint(a, s1), Value::Uint(b, s2)) => Ok(Value::Uint(uop(a, b), s1.max(s2))),
            (Value::Int(a, s1), Value::Uint(b, s2)) => {
                Ok(Value::Int(iop(a, I256::from_raw(b)), s1.max(s2)))
            }
            (Value::Uint(a, s1), Value::Int(b, s2)) => {
                Ok(Value::Int(iop(I256::from_raw(a), b), s1.max(s2)))
            }
            _ => bail!(error_msg),
        }
        .and_then(Value::validate_int)
    }
}

impl Add for Value {
    type Output = Result<Value>;

    fn add(self, other: Self) -> Self::Output {
        match (self, other) {
            (Value::Str(a), Value::Str(b)) => Ok(Value::Str(a + &b)),
            (Value::Array(a, t1), Value::Array(b, t2)) if t1 == t2 => {
                let mut new_arr = a.clone();
                new_arr.extend(b);
                Ok(Value::Array(new_arr, t1))
            }
            (s, o) => s.apply_operation(o, |a, b| a + b, |a, b| a + b, "add"),
        }
    }
}

impl Sub for Value {
    type Output = Result<Value>;

    fn sub(self, other: Self) -> Self::Output {
        self.apply_operation(other, |a, b| a - b, |a, b| a - b, "sub")
    }
}

impl Mul for Value {
    type Output = Result<Value>;

    fn mul(self, other: Self) -> Self::Output {
        self.apply_operation(other, |a, b| a * b, |a, b| a * b, "mul")
    }
}

impl Div for Value {
    type Output = Result<Value>;

    fn div(self, other: Self) -> Self::Output {
        self.apply_operation(other, |a, b| a / b, |a, b| a / b, "div")
    }
}

impl Rem for Value {
    type Output = Result<Value>;

    fn rem(self, other: Self) -> Self::Output {
        self.apply_operation(other, |a, b| a % b, |a, b| a % b, "rem")
    }
}

impl BitAnd for Value {
    type Output = Result<Value>;

    fn bitand(self, other: Self) -> Self::Output {
        self.apply_operation(other, |a, b| a & b, |a, b| a & b, "bitand")
    }
}

impl BitOr for Value {
    type Output = Result<Value>;

    fn bitor(self, other: Self) -> Self::Output {
        self.apply_operation(other, |a, b| a | b, |a, b| a | b, "bitor")
    }
}

impl BitXor for Value {
    type Output = Result<Value>;

    fn bitxor(self, other: Self) -> Self::Output {
        self.apply_operation(other, |a, b| a ^ b, |a, b| a ^ b, "bitxor")
    }
}

impl Shl for Value {
    type Output = Result<Value>;

    fn shl(self, other: Self) -> Self::Output {
        match (self, other) {
            (Value::Uint(a, s1), Value::Uint(b, s2)) => Ok(Value::Uint(a << b, s1.max(s2))),
            (s, o) => bail!("cannot shl {} and {}", s.get_type(), o.get_type()),
        }
    }
}

impl Shr for Value {
    type Output = Result<Value>;

    fn shr(self, other: Self) -> Self::Output {
        match (self, other) {
            (Value::Uint(a, s1), Value::Uint(b, s2)) => Ok(Value::Uint(a >> b, s1.max(s2))),
            (s, o) => bail!("cannot shl {} and {}", s.get_type(), o.get_type()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(
            (Value::from(1u64) + Value::from(2u64)).unwrap(),
            Value::from(3u64)
        );

        assert_eq!(
            (Value::from("foo") + Value::from("bar")).unwrap(),
            Value::from("foobar")
        );

        assert_eq!(
            (Value::Array(vec![Value::from(1u64)], Box::new(Type::Uint(256)))
                + Value::Array(vec![Value::from(2u64)], Box::new(Type::Uint(256))))
            .unwrap(),
            Value::Array(
                vec![Value::from(1u64), Value::from(2u64)],
                Box::new(Type::Uint(256))
            )
        );
    }

    #[test]
    fn test_value_from_hex() {
        let addr = Address::from_hex("0x7a250d5630b4cf539739df2c5dacb4c659f2488d").unwrap();
        let value = Value::from_hex("0x7a250d5630b4cf539739df2c5dacb4c659f2488d").unwrap();
        assert_eq!(value, Value::Addr(addr));

        let value = Value::from_hex("0xdeadbeef").unwrap();
        assert_eq!(value.to_string(), "0xdeadbeef");
    }

    #[test]
    fn test_slice() {
        let array = Value::Array(
            vec![Value::from(1u64), Value::from(2u64), Value::from(3u64)],
            Box::new(Type::Int(256)),
        );
        let slice = array
            .slice(Some(ArrayIndex(1)), Some(ArrayIndex(2)))
            .unwrap();
        assert_eq!(
            slice,
            Value::Array(vec![Value::from(2u64)], Box::new(Type::Int(256)))
        );

        let slice = array
            .slice(Some(ArrayIndex(1)), Some(ArrayIndex(-1)))
            .unwrap();
        assert_eq!(
            slice,
            Value::Array(vec![Value::from(2u64)], Box::new(Type::Int(256)))
        );

        let bytes = Value::Bytes(vec![1, 2, 3]);
        let slice = bytes
            .slice(Some(ArrayIndex(1)), Some(ArrayIndex(2)))
            .unwrap();
        assert_eq!(slice, Value::Bytes(vec![2]));

        let bytes = Value::Bytes(vec![1, 2, 3]);
        let slice = bytes.slice(Some(ArrayIndex(1)), None).unwrap();
        assert_eq!(slice, Value::Bytes(vec![2, 3]));

        let str = Value::Str("hello".to_string());
        let slice = str.slice(Some(ArrayIndex(1)), Some(ArrayIndex(3))).unwrap();
        assert_eq!(slice, Value::Str("el".to_string()));
    }
}
