use std::sync::Arc;

use crate::interpreter::{
    functions::{FunctionDef, FunctionParam, SyncMethod},
    ContractInfo, Env, Type, Value,
};
use alloy::dyn_abi::{DynSolType, DynSolValue, JsonAbiExt};
use anyhow::{bail, Result};
use lazy_static::lazy_static;

fn abi_decode(_env: &mut Env, _receiver: &Value, args: &[Value]) -> Result<Value> {
    let (data, sol_type) = match args {
        [Value::Bytes(data_), Value::Tuple(values)] => {
            let types = values
                .iter()
                .map(|v| match v {
                    Value::TypeObject(ty) => ty.clone().try_into(),
                    _ => bail!("abi.decode function expects tuple of types as argument"),
                })
                .collect::<Result<Vec<_>>>()?;
            (data_, DynSolType::Tuple(types))
        }
        [Value::Bytes(data_), Value::TypeObject(ty)] => {
            (data_, DynSolType::Tuple(vec![ty.clone().try_into()?]))
        }
        _ => bail!("abi.decode function expects bytes and tuple of types as argument"),
    };
    let decoded = sol_type.abi_decode(data)?;
    decoded.try_into()
}

fn abi_decode_calldata(_env: &mut Env, receiver: &Value, args: &[Value]) -> Result<Value> {
    let (name, abi) = match receiver {
        Value::TypeObject(Type::Contract(ContractInfo(name, abi))) => (name, abi),
        _ => bail!("decode function expects contract type as first argument"),
    };
    let data = match args.first() {
        Some(Value::Bytes(bytes)) => bytes,
        _ => bail!("decode function expects bytes as argument"),
    };
    let selector = alloy::primitives::FixedBytes::<4>::from_slice(&data[..4]);
    let function = abi
        .functions()
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

fn abi_encode(_env: &mut Env, _receiver: &Value, args: &[Value]) -> Result<Value> {
    let arr = Value::Tuple(args.to_vec());
    let dyn_sol = DynSolValue::try_from(&arr)?;
    let abi_encoded = dyn_sol.abi_encode();
    Ok(Value::Bytes(abi_encoded))
}

fn abi_encode_packed(_env: &mut Env, _receiver: &Value, args: &[Value]) -> Result<Value> {
    let arr = Value::Tuple(args.to_vec());
    let dyn_sol = DynSolValue::try_from(&arr)?;
    let abi_encoded = dyn_sol.abi_encode_packed();
    Ok(Value::Bytes(abi_encoded))
}

lazy_static! {
    pub static ref ABI_ENCODE: Arc<dyn FunctionDef> = SyncMethod::arc("encode", abi_encode, vec![]);
    pub static ref ABI_ENCODE_PACKED: Arc<dyn FunctionDef> =
        SyncMethod::arc("encodePacked", abi_encode_packed, vec![]);
    pub static ref ABI_DECODE: Arc<dyn FunctionDef> = SyncMethod::arc("decode", abi_decode, vec![]);
    pub static ref ABI_DECODE_CALLDATA: Arc<dyn FunctionDef> = SyncMethod::arc(
        "decode",
        abi_decode_calldata,
        vec![vec![FunctionParam::new("calldata", Type::Bytes)]]
    );
}
