use std::sync::Arc;

use crate::interpreter::{
    functions::{FunctionDef, FunctionParam, SyncMethod},
    ContractInfo, Env, Type, Value,
};
use alloy::{
    dyn_abi::{DynSolType, DynSolValue, JsonAbiExt},
    json_abi::{self, JsonAbi},
    primitives::FixedBytes,
};
use anyhow::{anyhow, bail, Result};
use lazy_static::lazy_static;

trait Decodable: JsonAbiExt {
    fn signature(&self) -> String;
    fn selector(&self) -> FixedBytes<4>;
}

impl Decodable for json_abi::Function {
    fn signature(&self) -> String {
        json_abi::Function::signature(self)
    }

    fn selector(&self) -> FixedBytes<4> {
        json_abi::Function::selector(self)
    }
}
impl Decodable for json_abi::Error {
    fn signature(&self) -> String {
        json_abi::Error::signature(self)
    }

    fn selector(&self) -> FixedBytes<4> {
        json_abi::Error::selector(self)
    }
}

fn _generic_abi_decode<D: Decodable, F>(
    receiver: &Value,
    args: &[Value],
    type_: &str,
    get_options: F,
) -> Result<Value>
where
    F: Fn(&JsonAbi) -> Vec<&D>,
{
    let (name, abi) = match receiver {
        Value::TypeObject(Type::Contract(ContractInfo(name, abi))) => (name, abi),
        _ => bail!("decode {} expects contract type as first argument", type_),
    };
    let data = match args.first() {
        Some(Value::Bytes(bytes)) => bytes,
        _ => bail!("decode {} expects bytes as argument", type_),
    };
    let selector = alloy::primitives::FixedBytes::<4>::from_slice(&data[..4]);
    let options = get_options(abi);
    let error = options
        .iter()
        .find(|f| f.selector() == selector)
        .ok_or(anyhow!(
            "{} with selector {} not found for {}",
            type_,
            selector,
            name
        ))?;
    let decoded = error.abi_decode_input(&data[4..], true)?;
    let values = decoded
        .into_iter()
        .map(Value::try_from)
        .collect::<Result<Vec<_>>>()?;
    Ok(Value::Tuple(vec![
        Value::Str(error.signature()),
        Value::Tuple(values),
    ]))
}

fn abi_decode_calldata(_env: &mut Env, receiver: &Value, args: &[Value]) -> Result<Value> {
    _generic_abi_decode(receiver, args, "function", |abi| abi.functions().collect())
}

fn abi_decode_error(_env: &mut Env, receiver: &Value, args: &[Value]) -> Result<Value> {
    _generic_abi_decode(receiver, args, "error", |abi| abi.errors().collect())
}

fn value_to_soltype(value: &Value) -> Result<DynSolType> {
    match value {
        Value::TypeObject(ty) => Ok(DynSolType::try_from(ty.clone())?),
        Value::Tuple(values) => values
            .iter()
            .map(value_to_soltype)
            .collect::<Result<Vec<_>>>()
            .map(DynSolType::Tuple),
        _ => bail!("abi.decode expects tuple of types as second argument"),
    }
}

fn abi_decode(args: &[Value]) -> Result<Value> {
    let decoded = match args {
        [Value::Bytes(data_), value] => {
            let ty = value_to_soltype(value)?;
            ty.abi_decode_params(data_)?
        }
        _ => bail!("abi.decode expects bytes and tuple of types as argument"),
    };
    decoded.try_into()
}

fn abi_decode_(_env: &mut Env, _receiver: &Value, args: &[Value]) -> Result<Value> {
    abi_decode(args)
}

fn abi_encode(args: &[Value]) -> Result<Value> {
    let abi_encoded = if args.len() == 1 {
        DynSolValue::try_from(&args[0])?.abi_encode()
    } else {
        let arr = Value::Tuple(args.to_vec());
        DynSolValue::try_from(&arr)?.abi_encode_params()
    };
    Ok(Value::Bytes(abi_encoded))
}

fn abi_encode_(_env: &mut Env, _receiver: &Value, args: &[Value]) -> Result<Value> {
    abi_encode(args)
}

fn abi_encode_packed(args: &[Value]) -> Result<Value> {
    let arr = Value::Tuple(args.to_vec());
    let dyn_sol = DynSolValue::try_from(&arr)?;
    let abi_encoded = dyn_sol.abi_encode_packed();
    Ok(Value::Bytes(abi_encoded))
}

fn abi_encode_packed_(_env: &mut Env, _receiver: &Value, args: &[Value]) -> Result<Value> {
    abi_encode_packed(args)
}

lazy_static! {
    pub static ref ABI_ENCODE: Arc<dyn FunctionDef> =
        SyncMethod::arc("encode", abi_encode_, vec![]);
    pub static ref ABI_ENCODE_PACKED: Arc<dyn FunctionDef> =
        SyncMethod::arc("encodePacked", abi_encode_packed_, vec![]);
    pub static ref ABI_DECODE: Arc<dyn FunctionDef> =
        SyncMethod::arc("decode", abi_decode_, vec![]);
    pub static ref ABI_DECODE_CALLDATA: Arc<dyn FunctionDef> = SyncMethod::arc(
        "decode",
        abi_decode_calldata,
        vec![vec![FunctionParam::new("calldata", Type::Bytes)]]
    );
    pub static ref ABI_DECODE_ERROR: Arc<dyn FunctionDef> = SyncMethod::arc(
        "decode_error",
        abi_decode_error,
        vec![vec![FunctionParam::new("data", Type::Bytes)]]
    );
}

#[cfg(test)]
mod tests {
    use alloy::hex;

    use super::*;

    #[test]
    fn test_abi_encode_single_string() {
        let args = vec![Value::from("foo")];
        let expected_bytes = hex::decode("0x00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000003666f6f0000000000000000000000000000000000000000000000000000000000").unwrap();
        let expected = Value::Bytes(expected_bytes);
        let actual = abi_encode(&args).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_abi_encode_single_uint256() {
        let args = vec![Value::from(1)];
        let expected_bytes =
            hex::decode("0x0000000000000000000000000000000000000000000000000000000000000001")
                .unwrap();
        let expected = Value::Bytes(expected_bytes);
        let actual = abi_encode(&args).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_abi_encode_multiple_strings() {
        let args = vec![Value::from("foo"), Value::from("bar")];
        let expected_bytes = hex::decode("0x000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000800000000000000000000000000000000000000000000000000000000000000003666f6f000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000036261720000000000000000000000000000000000000000000000000000000000").unwrap();
        let expected = Value::Bytes(expected_bytes);
        let actual = abi_encode(&args).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_abi_encode_multiple_uint256() {
        let args = vec![Value::from(1u64), Value::from(2u64)];
        let expected_bytes =
            hex::decode("0x00000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000002")
                .unwrap();
        let expected = Value::Bytes(expected_bytes);
        let actual = abi_encode(&args).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_abi_encode_multiple_types() {
        let args = vec![Value::from(1u64), Value::from("foo")];
        let expected_bytes =
            hex::decode("0x000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000000000000000003666f6f0000000000000000000000000000000000000000000000000000000000")
                .unwrap();
        let expected = Value::Bytes(expected_bytes);
        let actual = abi_encode(&args).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_abi_decode_single_string() {
        let value = Value::from("foo");
        let encoded = abi_encode(&[value.clone()]).unwrap();
        let decoded = abi_decode(&[encoded.clone(), Value::TypeObject(Type::String)]).unwrap();
        assert_eq!(value, decoded);
    }

    #[test]
    fn test_abi_decode_multiple_strings() {
        let args = vec![Value::from("foo"), Value::from("bar")];
        let encoded = abi_encode(&args).unwrap();
        let decoded = abi_decode(&[
            encoded.clone(),
            Value::Tuple(vec![
                Value::TypeObject(Type::String),
                Value::TypeObject(Type::String),
            ]),
        ])
        .unwrap();
        assert_eq!(Value::Tuple(args), decoded);
    }

    #[test]
    fn test_abi_decode_multiple_types() {
        let args = vec![Value::from("foo"), Value::from(2u64)];
        let encoded = abi_encode(&args).unwrap();
        let decoded = abi_decode(&[
            encoded.clone(),
            Value::Tuple(vec![
                Value::TypeObject(Type::String),
                Value::TypeObject(Type::Uint(256)),
            ]),
        ])
        .unwrap();
        assert_eq!(Value::Tuple(args), decoded);
    }

    #[test]
    fn test_abi_decode_nested_types() {
        let args = vec![
            Value::Bool(true),
            Value::Tuple(vec![Value::from("foo"), Value::from(2u64)]),
        ];
        let encoded = abi_encode(&args).unwrap();
        let decoded = abi_decode(&[
            encoded.clone(),
            Value::Tuple(vec![
                Value::TypeObject(Type::Bool),
                Value::Tuple(vec![
                    Value::TypeObject(Type::String),
                    Value::TypeObject(Type::Uint(256)),
                ]),
            ]),
        ])
        .unwrap();
        assert_eq!(Value::Tuple(args), decoded);
    }
}
