use crate::interpreter::{
    builtins::{FunctionDefinition, FunctionParam},
    ContractInfo, Env, Type, Value,
};
use alloy::dyn_abi::{DynSolType, DynSolValue, JsonAbiExt};
use anyhow::{bail, Result};
use futures::{future::BoxFuture, FutureExt};
use lazy_static::lazy_static;

fn abi_decode<'a>(_env: &'a mut Env, args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move {
        let (data, sol_type) = match args {
            [_, Value::Bytes(data_), Value::Tuple(values)] => {
                let types = values
                    .iter()
                    .map(|v| match v {
                        Value::TypeObject(ty) => ty.clone().try_into(),
                        _ => bail!("abi.decode function expects tuple of types as argument"),
                    })
                    .collect::<Result<Vec<_>>>()?;
                (data_, DynSolType::Tuple(types))
            }
            [_, Value::Bytes(data_), Value::TypeObject(ty)] => {
                (data_, DynSolType::Tuple(vec![ty.clone().try_into()?]))
            }
            _ => bail!("abi.decode function expects bytes and tuple of types as argument"),
        };
        let decoded = sol_type.abi_decode(data)?;
        decoded.try_into()
    }
    .boxed()
}

fn abi_decode_calldata<'a>(_env: &'a mut Env, args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move {
        let (name, abi) = match args.first() {
            Some(Value::TypeObject(Type::Contract(ContractInfo(name, abi)))) => (name, abi),
            _ => bail!("decode function expects contract type as first argument"),
        };
        let data = match args.get(1) {
            Some(Value::Bytes(bytes)) => bytes,
            _ => bail!("decode function expects bytes as argument"),
        };
        let selector = alloy::primitives::FixedBytes::<4>::from_slice(&data[..4]);
        let function =
            abi.functions()
                .find(|f| f.selector() == selector)
                .ok_or(anyhow::anyhow!(
                    "function with selector {} not found for {}",
                    selector,
                    name
                ))?;
        let decoded = function.abi_decode_input(&data[4..], true)?;
        let values = decoded
            .into_iter()
            .map(Value::try_from)
            .collect::<Result<Vec<_>>>()?;
        Ok(Value::Tuple(vec![
            Value::Str(function.signature()),
            Value::Tuple(values),
        ]))
    }
    .boxed()
}

fn abi_encode<'a>(_env: &'a mut Env, args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move {
        let arr = Value::Tuple(args[1..].to_vec());
        let dyn_sol = DynSolValue::try_from(&arr)?;
        let abi_encoded = dyn_sol.abi_encode();
        Ok(Value::Bytes(abi_encoded))
    }
    .boxed()
}

fn abi_encode_packed<'a>(_env: &'a mut Env, args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move {
        let arr = Value::Tuple(args[1..].to_vec());
        let dyn_sol = DynSolValue::try_from(&arr)?;
        let abi_encoded = dyn_sol.abi_encode_packed();
        Ok(Value::Bytes(abi_encoded))
    }
    .boxed()
}

lazy_static! {
    pub static ref ABI_ENCODE: FunctionDefinition = FunctionDefinition {
        name_: "encode".to_string(),
        property: false,
        valid_args: vec![],
        execute_fn: abi_encode,
    };
    pub static ref ABI_ENCODE_PACKED: FunctionDefinition = FunctionDefinition {
        name_: "encodePacked".to_string(),
        property: false,
        valid_args: vec![],
        execute_fn: abi_encode_packed,
    };
    pub static ref ABI_DECODE: FunctionDefinition = FunctionDefinition {
        name_: "decode".to_string(),
        property: false,
        valid_args: vec![],
        execute_fn: abi_decode,
    };
    pub static ref ABI_DECODE_CALLDATA: FunctionDefinition = FunctionDefinition {
        name_: "decode".to_string(),
        property: false,
        valid_args: vec![vec![FunctionParam::new("calldata", Type::Bytes)]],
        execute_fn: abi_decode_calldata,
    };
}
