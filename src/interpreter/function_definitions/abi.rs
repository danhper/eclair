use crate::interpreter::{function_definitions::FunctionDefinition, Env, Value};
use alloy::dyn_abi::{DynSolType, DynSolValue};
use anyhow::{bail, Result};
use futures::{future::BoxFuture, FutureExt};
use lazy_static::lazy_static;

fn abi_decode<'a>(_env: &'a mut Env, args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move {
        let (data, sol_type) = match args {
            [Value::Bytes(data_), type_ @ Value::Tuple(_)] => {
                (data_, DynSolType::try_from(type_.get_type())?)
            }
            [Value::Bytes(data_), Value::TypeObject(ty)] => {
                (data_, DynSolType::Tuple(vec![ty.clone().try_into()?]))
            }
            _ => bail!("abi.decode function expects bytes and tuple of types as argument"),
        };
        let decoded = sol_type.abi_decode(data)?;
        decoded.try_into()
    }
    .boxed()
}

fn abi_encode<'a>(_env: &'a mut Env, args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move {
        let arr = Value::Tuple(args.to_vec());
        let dyn_sol = DynSolValue::try_from(&arr)?;
        let abi_encoded = dyn_sol.abi_encode();
        Ok(Value::Bytes(abi_encoded))
    }
    .boxed()
}

fn abi_encode_packed<'a>(_env: &'a mut Env, args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
    async move {
        let arr = Value::Tuple(args.to_vec());
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
}
