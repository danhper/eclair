use lazy_static::lazy_static;
use std::fmt::Display;

use alloy::{
    dyn_abi::DynSolType,
    json_abi::JsonAbi,
    primitives::{Address, B256, I256, U160, U256},
};
use anyhow::{anyhow, bail, Result};
use indexmap::IndexMap;
use itertools::Itertools;
use solang_parser::pt as parser;

use super::{
    builtins::{INSTANCE_METHODS, STATIC_METHODS},
    functions::{ContractFunction, Function},
    utils::to_fixed_bytes,
    Value,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashableIndexMap<K, V>(pub IndexMap<K, V>)
where
    K: Eq + std::hash::Hash,
    V: Eq;

impl<K, V> std::default::Default for HashableIndexMap<K, V>
where
    K: Eq + std::hash::Hash,
    V: Eq,
{
    fn default() -> Self {
        Self(IndexMap::default())
    }
}

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

pub struct ArrayIndex(pub i64);
impl ArrayIndex {
    pub fn get_index(&self, array_size: usize) -> Result<usize> {
        let index = if self.0 < 0 {
            array_size as i64 + self.0
        } else {
            self.0
        };
        if index < 0 || index as usize >= array_size {
            bail!(
                "index out of bounds: {} for array of size {}",
                self.0,
                array_size
            )
        }
        Ok(index as usize)
    }
}
impl TryFrom<Value> for ArrayIndex {
    type Error = anyhow::Error;

    fn try_from(value: Value) -> Result<Self> {
        match value {
            Value::Int(i, _) => Ok(ArrayIndex(i.try_into()?)),
            Value::Uint(i, _) => Ok(ArrayIndex(i.try_into()?)),
            _ => bail!("cannot convert {} to array index", value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContractInfo(pub String, pub JsonAbi);

impl ContractInfo {
    pub fn make_function(&self, name: &str, addr: Address) -> Result<Function> {
        let _func = self
            .1
            .function(name)
            .ok_or_else(|| anyhow!("function {} not found in contract {}", name, self.0))?;
        Ok(Function::new(
            ContractFunction::arc(name),
            Some(&Value::Contract(self.clone(), addr)),
        ))
    }

    pub fn member_access(&self, name: &str) -> Result<Value> {
        if let Some(event) = self.1.events.get(name).and_then(|v| v.first()) {
            return Ok(Value::TypeObject(Type::Event(event.clone())));
        }
        let func = STATIC_METHODS
            .get(&NonParametricType::Contract)
            .unwrap()
            .get(name)
            .ok_or(anyhow!("{} not found in contract {}", name, self.0))?;
        Ok(Function::method(
            func.clone(),
            &Value::TypeObject(Type::Contract(self.clone())),
        )
        .into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NonParametricType {
    Any,
    Null,
    Address,
    Bool,
    Int,
    Uint,
    FixBytes,
    Bytes,
    String,
    Array,
    FixedArray,
    NamedTuple,
    Tuple,
    Mapping,
    Contract,
    Event,
    Transaction,
    Function,
    Repl,
    Accounts,
    Vm,
    Block,
    Console,
    Fs,
    Json,
    Events,
    Abi,
    Type,
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
    Event(alloy::json_abi::Event),
    Transaction,
    Function,
    Accounts,
    Repl,
    Vm,
    Block,
    Console,
    Fs,
    Json,
    Events,
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
            Type::Event(event) => write!(f, "{}", event.full_signature()),
            Type::Function => write!(f, "function"),

            Type::Transaction => write!(f, "Transaction"),

            Type::Accounts => write!(f, "accounts"),
            Type::Repl => write!(f, "repl"),
            Type::Vm => write!(f, "vm"),
            Type::Block => write!(f, "block"),
            Type::Events => write!(f, "events"),
            Type::Console => write!(f, "console"),
            Type::Json => write!(f, "json"),
            Type::Fs => write!(f, "fs"),
            Type::Abi => write!(f, "abi"),
            Type::Type(t) => write!(f, "type({})", t),
        }
    }
}

impl<T: AsRef<Type>> From<T> for NonParametricType {
    fn from(value: T) -> Self {
        match value.as_ref() {
            Type::Any => NonParametricType::Any,
            Type::Null => NonParametricType::Null,
            Type::Address => NonParametricType::Address,
            Type::Bool => NonParametricType::Bool,
            Type::Int(_) => NonParametricType::Int,
            Type::Uint(_) => NonParametricType::Uint,
            Type::FixBytes(_) => NonParametricType::FixBytes,
            Type::Bytes => NonParametricType::Bytes,
            Type::String => NonParametricType::String,
            Type::Array(_) => NonParametricType::Array,
            Type::FixedArray(..) => NonParametricType::FixedArray,
            Type::NamedTuple(..) => NonParametricType::NamedTuple,
            Type::Tuple(_) => NonParametricType::Tuple,
            Type::Mapping(..) => NonParametricType::Mapping,
            Type::Contract(..) => NonParametricType::Contract,
            Type::Event(..) => NonParametricType::Event,
            Type::Function => NonParametricType::Function,
            Type::Accounts => NonParametricType::Accounts,
            Type::Transaction => NonParametricType::Transaction,
            Type::Repl => NonParametricType::Repl,
            Type::Vm => NonParametricType::Vm,
            Type::Block => NonParametricType::Block,
            Type::Console => NonParametricType::Console,
            Type::Fs => NonParametricType::Fs,
            Type::Json => NonParametricType::Json,
            Type::Events => NonParametricType::Events,
            Type::Abi => NonParametricType::Abi,
            Type::Type(_) => NonParametricType::Type,
        }
    }
}

impl AsRef<Type> for Type {
    fn as_ref(&self) -> &Type {
        self
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

fn canonical_string_for_tuple(types: &[Type]) -> Result<String> {
    let items = types
        .iter()
        .map(|t| t.canonical_string())
        .collect::<Result<Vec<_>>>()?
        .join(",");
    Ok(format!("({})", items))
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

    pub fn canonical_string(&self) -> Result<String> {
        let result = match self {
            Type::Address => "address".to_string(),
            Type::Bool => "bool".to_string(),
            Type::Int(size) => format!("int{}", size),
            Type::Uint(size) => format!("uint{}", size),
            Type::FixBytes(size) => format!("bytes{}", size),
            Type::Bytes => "bytes".to_string(),
            Type::String => "string".to_string(),
            Type::Array(t) => format!("{}[]", t.canonical_string()?),
            Type::FixedArray(t, size) => format!("{}[{}]", t.canonical_string()?, size),
            Type::NamedTuple(_, fields) => {
                let types = fields.0.values().cloned().collect_vec();
                canonical_string_for_tuple(&types)?
            }
            Type::Tuple(types) => canonical_string_for_tuple(types.as_slice())?,
            _ => bail!("cannot get canonical string for type {}", self),
        };
        Ok(result)
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
            "mapping".to_string(),
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
            (Type::Address, Value::FixBytes(v, _)) => {
                Ok(Value::Addr(Address::from_slice(&v.0[12..])))
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
            (t @ Type::FixBytes(bytes_num), Value::Uint(v, bits_num))
                if *bytes_num * 8 == *bits_num =>
            {
                let bytes = to_fixed_bytes(&v.to_be_bytes_vec(), *bytes_num, true)?;
                t.cast(&Value::FixBytes(bytes, *bytes_num))
            }
            (Type::Uint(bits_num), Value::FixBytes(v, bytes_num))
                if *bytes_num * 8 == *bits_num =>
            {
                let num = U256::from_be_slice(&v[..*bytes_num]);
                Ok(Value::Uint(num, *bits_num))
            }
            (Type::FixBytes(size), Value::FixBytes(bytes, previous_size)) => Ok(Value::FixBytes(
                to_fixed_bytes(&bytes.0[..*previous_size], *size, false)?,
                *size,
            )),
            (Type::FixBytes(size), Value::Addr(addr)) if *size == 20 => {
                let bytes = to_fixed_bytes(&addr.0 .0, 20, false)?;
                Ok(Value::FixBytes(bytes, 20))
            }
            (Type::Transaction, Value::FixBytes(v, 32)) => Ok(Value::Transaction(*v)),
            (Type::Bytes, Value::Str(v)) => Ok(Value::Bytes(v.as_bytes().to_vec())),
            (type_ @ Type::FixBytes(_), Value::Str(_)) => type_.cast(&Type::Bytes.cast(value)?),
            (Type::Bytes, Value::FixBytes(v, s)) => {
                Ok(Value::Bytes(v.0[v.0.len() - *s..].to_vec()))
            }
            (Type::FixBytes(size), Value::Bytes(bytes)) => {
                Ok(Value::FixBytes(to_fixed_bytes(bytes, *size, false)?, *size))
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
            (Type::NamedTuple(name, types_), Value::NamedTuple(_, kvs)) => {
                if kvs.0.keys().ne(types_.0.keys()) {
                    bail!("named tuple keys do not match")
                }
                let mut new_values = IndexMap::new();
                for (key, type_) in types_.0.iter() {
                    new_values.insert(key.clone(), type_.cast(kvs.0.get(key).unwrap())?);
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
            Type::Type(type_) => {
                let mut static_methods = STATIC_METHODS
                    .get(&type_.into())
                    .map_or(vec![], |m| m.keys().cloned().collect());

                if let Type::Contract(ContractInfo(_, abi)) = type_.as_ref() {
                    static_methods.extend(abi.events.keys().map(|s| s.to_string()));
                }

                static_methods
            }
            _ => INSTANCE_METHODS
                .get(&self.into())
                .map_or(vec![], |m| m.keys().cloned().collect()),
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
}

lazy_static! {
    pub static ref LOG_TYPE: Type = Type::NamedTuple(
        "Log".to_string(),
        HashableIndexMap::from_iter([
            ("address".to_string(), Type::Address),
            ("topics".to_string(), Type::Array(Box::new(Type::Uint(256)))),
            ("data".to_string(), Type::Bytes),
            ("args".to_string(), Type::Any),
        ]),
    );
}

#[cfg(test)]
mod tests {
    use crate::interpreter::Value;
    use alloy::{
        hex::FromHex,
        primitives::{B256, I128, I256, I32, I64, I8, U128, U256, U32, U64, U8},
    };

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

    #[test]
    fn cast_bytes() {
        let b256_value =
            Value::from_hex("0x00000000000000000000000000000000000000000000000000000000281dd5af")
                .unwrap();

        assert_eq!(
            Type::FixBytes(4).cast(&b256_value).unwrap(),
            Value::from_hex("0x00000000").unwrap()
        );
        let b4 =
            B256::from_hex("0x281dd5af00000000000000000000000000000000000000000000000000000000")
                .unwrap();
        let b4_value = Value::FixBytes(b4, 4);

        let padded_b256_value =
            Value::from_hex("0x281dd5af00000000000000000000000000000000000000000000000000000000")
                .unwrap();
        assert_eq!(
            Type::FixBytes(32).cast(&b4_value).unwrap(),
            padded_b256_value
        );
        assert_eq!(
            Type::FixBytes(4).cast(&padded_b256_value).unwrap(),
            b4_value
        );

        let n = Value::Uint(U256::from(1), 8);
        let bytes1 = Type::FixBytes(1).cast(&n).unwrap();
        assert_eq!(bytes1, Value::from_hex("0x01").unwrap());
        assert_eq!(Type::Uint(8).cast(&bytes1).unwrap(), n);
    }

    #[test]
    fn array_index() {
        let size = 10;
        let cases = vec![(0, 0), (1, 1), (-1, 9), (5, 5), (-5, 5)];
        for (index, expected) in cases {
            assert_eq!(super::ArrayIndex(index).get_index(size).unwrap(), expected);
        }
    }
}
