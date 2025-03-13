use std::sync::Arc;

use crate::{
    interpreter::{
        functions::{AsyncMethod, FunctionDef, FunctionParam, SyncMethod},
        ContractInfo, Env, Type, Value,
    },
    loaders,
};
use alloy::{
    dyn_abi::{DynSolType, DynSolValue, JsonAbiExt},
    json_abi::{self, JsonAbi},
    primitives::FixedBytes,
};
use anyhow::{anyhow, bail, Result};
use futures::{future::BoxFuture, FutureExt};
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

fn _run_decode(signature: String, decoded: Vec<DynSolValue>) -> Result<Value> {
    let values = decoded
        .into_iter()
        .map(Value::try_from)
        .collect::<Result<Vec<_>>>()?;
    Ok(Value::Tuple(vec![
        Value::Str(signature),
        Value::Tuple(values),
    ]))
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
    let decodable = options
        .iter()
        .find(|f| f.selector() == selector)
        .ok_or(anyhow!(
            "{} with selector {} not found for {}",
            type_,
            selector,
            name
        ))?;
    let decoded = decodable.abi_decode_input(&data[4..], true)?;
    _run_decode(decodable.signature(), decoded)
}

fn abi_decode_data(env: &mut Env, _receiver: &Value, args: &[Value]) -> Result<Value> {
    let data = match args.first() {
        Some(Value::Bytes(bytes)) => bytes,
        _ => bail!("abi.decodeData expects bytes as argument"),
    };
    if data.len() < 4 {
        bail!("abi.decodeData expects at least 4 bytes");
    }
    let selector = alloy::primitives::FixedBytes::<4>::from_slice(&data[..4]);
    let (signature, decoded) = if let Some(func) = env.get_function(&selector) {
        (func.signature(), func.abi_decode_input(&data[4..], true)?)
    } else if let Some(error) = env.get_error(&selector) {
        (error.signature(), error.abi_decode_input(&data[4..], true)?)
    } else {
        bail!("function or error with selector {} not found", selector);
    };
    _run_decode(signature, decoded)
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
    let data = match args.first().map(|a| Type::Bytes.cast(a)) {
        Some(Ok(Value::Bytes(bytes))) => bytes,
        _ => bail!("abi.decode expects bytes as first argument"),
    };
    let type_ = args
        .get(1)
        .ok_or(anyhow!("abi.decode expects type as second argument"))?;

    let ty = value_to_soltype(type_)?;
    let decoded = ty.abi_decode_params(&data)?;
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

fn fetch_abi<'a>(
    env: &'a mut Env,
    _receiver: &'a Value,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        match args {
            [Value::Str(name), Value::Addr(address)] => {
                let chain_id = env.get_chain_id().await?;
                let etherscan_config = env.config.get_etherscan_config(chain_id)?;
                let abi =
                    loaders::etherscan::load_abi(etherscan_config, &address.to_string()).await?;
                let contract_info = env.add_contract(name, abi);
                Ok(Value::Contract(contract_info, *address))
            }
            _ => bail!("fetchAbi: invalid arguments"),
        }
    }
    .boxed()
}

fn load_abi(env: &mut Env, _receiver: &Value, args: &[Value]) -> Result<Value> {
    let (name, filepath, key) = match args {
        [Value::Str(name), Value::Str(filepath)] => (name, filepath, None),
        [Value::Str(name), Value::Str(filepath), Value::Str(key)] => {
            (name, filepath, Some(key.as_str()))
        }
        _ => bail!("loadAbi: invalid arguments"),
    };
    let abi = loaders::file::load_abi(filepath, key)?;
    env.add_contract(name, abi);
    Ok(Value::Null)
}

lazy_static! {
    pub static ref ABI_ENCODE: Arc<dyn FunctionDef> =
        SyncMethod::arc("encode", abi_encode_, vec![]);
    pub static ref ABI_ENCODE_PACKED: Arc<dyn FunctionDef> =
        SyncMethod::arc("encodePacked", abi_encode_packed_, vec![]);
    pub static ref ABI_DECODE: Arc<dyn FunctionDef> =
        SyncMethod::arc("decode", abi_decode_, vec![]);
    pub static ref ABI_DECODE_DATA: Arc<dyn FunctionDef> = SyncMethod::arc(
        "decodeData",
        abi_decode_data,
        vec![vec![FunctionParam::new("data", Type::Bytes)]]
    );
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
    pub static ref ABI_FETCH: Arc<dyn FunctionDef> = AsyncMethod::arc(
        "fetch",
        fetch_abi,
        vec![vec![
            FunctionParam::new("name", Type::String),
            FunctionParam::new("address", Type::Address)
        ]]
    );
    pub static ref ABI_LOAD: Arc<dyn FunctionDef> = SyncMethod::arc(
        "load",
        load_abi,
        vec![
            vec![
                FunctionParam::new("name", Type::String),
                FunctionParam::new("filepath", Type::String)
            ],
            vec![
                FunctionParam::new("name", Type::String),
                FunctionParam::new("filepath", Type::String),
                FunctionParam::new("jsonKey", Type::String)
            ]
        ]
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

    #[test]
    fn test_abi_decode_fixed_bytes() {
        let bytes = hex::decode("0x00000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000003000000000000000000000000e07f9d810a48ab5c3c914ba3ca53af14e4491e8a40c10f1900000000000000000000000000000000000000000000000000000000000000000000000000000000ea8106503a136eaad94bf9fcf1de485459fd538e000000000000000000000000a1886c8d748deb3774225593a70c79454b1da8a6e182dd8700000000000000000000000000000000000000000000000000000000000000000000000000000000fe41992176ad0fa41c4a2ed70f3c36273027c27c000000000000000000000000a1886c8d748deb3774225593a70c79454b1da8a6401030ce00000000000000000000000000000000000000000000000000000000000000000000000000000000fe41992176ad0fa41c4a2ed70f3c36273027c27c").unwrap();
        let decoded = abi_decode(&[
            Value::Bytes(bytes.clone()),
            Value::Tuple(vec![Value::TypeObject(Type::Array(Box::new(Type::Tuple(
                vec![Type::Address, Type::FixBytes(4), Type::Address],
            ))))]),
        ])
        .unwrap();

        let first_elem = decoded.at(&0.into()).unwrap().at(&0.into()).unwrap();
        let expected_address = "0xe07F9D810a48ab5c3c914BA3cA53AF14E4491e8A";
        let actual_address = first_elem.at(&0.into()).unwrap().to_string();
        assert_eq!(expected_address, actual_address);
        let expected_selector = "0x40c10f19";
        let actual_selector = first_elem.at(&1.into()).unwrap().to_string();
        assert_eq!(expected_selector, actual_selector);
    }
}
