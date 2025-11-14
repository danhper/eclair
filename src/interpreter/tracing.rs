use alloy::{
    dyn_abi::{FunctionExt, JsonAbiExt},
    json_abi::Function,
    primitives::{Bytes, FixedBytes},
    rpc::types::trace::geth::CallFrame,
};
use anyhow::Result;
use itertools::Itertools;

use crate::interpreter::utils::decode_error;

use super::{Env, Value};

fn try_format_func(
    env: &Env,
    func: &Function,
    input: &[u8],
    output: &Option<Bytes>,
    is_error: bool,
) -> Result<String> {
    let decoded = func.abi_decode_input(input)?;
    let values = Value::try_from(decoded)?;
    let result = format!("{}{}", func.name, values);
    if let Some(output) = output {
        let value_output = if is_error {
            decode_error(env, output)?
        } else {
            let decoded = func.abi_decode_output(output)?;
            if decoded.len() == 1 {
                Value::try_from(decoded[0].clone())?
            } else {
                Value::try_from(decoded)?
            }
        };
        Ok(format!("{} -> {}", result, value_output))
    } else {
        Ok(result)
    }
}

fn get_formatted_function(
    env: &Env,
    input: &Bytes,
    output: &Option<Bytes>,
    is_error: bool,
) -> String {
    if input.len() >= 4 {
        let selector = FixedBytes::<4>::from_slice(&input[..4]);
        if let Some(func) = env.get_function(&selector) {
            if let Ok(result) = try_format_func(env, func, &input[4..], output, is_error) {
                return result;
            }
        }
    }
    if let Some(output) = output {
        format!("{} -> {}", input, output)
    } else {
        format!("{}", input)
    }
}

fn get_formatted_call(env: &Env, frame: &CallFrame) -> String {
    let mut formatted = "".to_string();
    if let Some(addr) = frame.to {
        if let Some(contract) = env.get_contract_name(&addr) {
            formatted.push_str(&format!("{}({})", &contract, addr));
        } else {
            formatted.push_str(&format!("{}", addr));
        }
    }
    formatted.push_str("::");
    formatted.push_str(&get_formatted_function(
        env,
        &frame.input,
        &frame.output,
        frame.error.is_some(),
    ));

    formatted
}

fn format_call(
    env: &Env,
    frame: &CallFrame,
    depth: usize,
    wrap_opts: &textwrap::Options,
) -> String {
    let indent = format!("{:indent$}", "", indent = depth * 4);
    let subsequent_indent = format!("{:indent$}", "", indent = depth * 4 + 2);
    let opts = wrap_opts
        .clone()
        .initial_indent(&indent)
        .subsequent_indent(&subsequent_indent);
    let call_str = get_formatted_call(env, frame);
    let rows = textwrap::wrap(&call_str, opts);
    let mut result = rows.iter().join("\n");

    for call in &frame.calls {
        result.push('\n');
        result.push_str(&format_call(env, call, depth + 1, wrap_opts));
    }

    result
}

pub fn format_call_frame(env: &Env, frame: &CallFrame) -> String {
    let wrap_opts = textwrap::Options::new(textwrap::termwidth() - 16).break_words(true);
    format_call(env, frame, 0, &wrap_opts)
}
