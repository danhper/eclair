use anyhow::{anyhow, bail, Result};
use indexmap::IndexMap;
use itertools::{Either, Itertools};
use std::str::FromStr;

use alloy::{
    dyn_abi::{EventExt, JsonAbiExt},
    json_abi::Event,
    primitives::{FixedBytes, B256, U256},
    rpc::types::{Log, TransactionReceipt},
};

use super::{types::HashableIndexMap, Env, Type, Value};

pub fn join_with_final<T>(separator: &str, final_separator: &str, strings: Vec<T>) -> String
where
    T: std::string::ToString,
{
    if strings.is_empty() {
        return "".to_string();
    }
    if strings.len() == 1 {
        return strings[0].to_string();
    }
    let mut result = strings[0].to_string();
    for s in strings[1..strings.len() - 1].iter() {
        result.push_str(separator);
        result.push_str(&s.to_string());
    }
    result.push_str(final_separator);
    result.push_str(&strings[strings.len() - 1].to_string());
    result
}

pub fn parse_rational_literal(whole: &str, raw_fraction: &str, raw_exponent: &str) -> Result<U256> {
    let mut n = if whole.is_empty() {
        U256::from(0)
    } else {
        U256::from_str(whole)?
    };
    let exponent = if raw_exponent.is_empty() {
        U256::from(0)
    } else {
        U256::from_str(raw_exponent)?
    };
    n *= U256::from(10).pow(exponent);

    if !raw_fraction.is_empty() {
        let removed_zeros = raw_fraction.trim_end_matches('0');
        let decimals_count = U256::from(removed_zeros.len());
        let fraction = U256::from_str(removed_zeros)?;
        if decimals_count > exponent {
            bail!("fraction has more digits than decimals");
        }
        let adjusted_fraction = fraction * U256::from(10).pow(exponent - decimals_count);
        n += adjusted_fraction;
    };

    Ok(n)
}

pub fn decode_log_args(log: &Log, event: &Event) -> Result<Value> {
    let decoded = event.decode_log(log.data(), true)?;
    let mut fully_decoded = IndexMap::new();
    let (indexed_names, body_names): (Vec<_>, Vec<_>) = event.inputs.iter().partition_map(|v| {
        if v.indexed {
            Either::Left(v.name.clone())
        } else {
            Either::Right(v.name.clone())
        }
    });
    for (value, name) in decoded.indexed.iter().zip(indexed_names.iter()) {
        fully_decoded.insert(name.clone(), value.clone().try_into()?);
    }
    for (value, name) in decoded.body.iter().zip(body_names.iter()) {
        fully_decoded.insert(name.clone(), value.clone().try_into()?);
    }

    Ok(Value::NamedTuple(
        event.name.clone(),
        HashableIndexMap(fully_decoded),
    ))
}

pub fn decode_error(env: &Env, data: &[u8]) -> Result<Value> {
    if data.len() < 4 {
        bail!("error data is too short");
    }
    let selector = FixedBytes::from_slice(&data[..4]);
    let error = env
        .get_error(&selector)
        .ok_or(anyhow!("error with selector {} not found", selector))?;
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

pub fn log_to_value(env: &Env, log: Log) -> Result<Value> {
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

    if let Some(evt) = log.topic0().and_then(|t| env.get_event(t)) {
        let decoded_args = decode_log_args(&log, evt)?;
        fields.insert("args".to_string(), decoded_args);
    } else {
        fields.insert("args".to_string(), Value::Null);
    }

    Ok(Value::NamedTuple(
        "Log".to_string(),
        HashableIndexMap(fields),
    ))
}

pub fn receipt_to_value(env: &Env, receipt: TransactionReceipt) -> Result<Value> {
    let logs = receipt.inner.logs().to_vec();
    let transformed_logs = logs
        .into_iter()
        .map(|log| log_to_value(env, log))
        .collect::<Result<Vec<Value>>>()?;
    Ok(Value::from_receipt(receipt, transformed_logs))
}

pub fn to_fixed_bytes(bytes: &[u8], size: usize, use_trailing_bytes: bool) -> Result<B256> {
    let mut new_bytes = vec![0; 32];
    let new_size = bytes.len().min(size);
    if use_trailing_bytes {
        let to_copy = &bytes[bytes.len() - new_size..];
        new_bytes[..new_size].copy_from_slice(to_copy);
    } else {
        let to_copy = &bytes[..new_size];
        new_bytes[..new_size].copy_from_slice(to_copy);
    }
    Ok(B256::from_slice(&new_bytes))
}

#[cfg(test)]
mod tests {
    use alloy::{
        json_abi::EventParam,
        primitives::{address, b256, bytes, LogData},
    };

    use super::*;

    #[test]
    fn test_join_with_final() {
        assert_eq!(
            join_with_final(", ", " and ", vec!["a", "b", "c"]),
            "a, b and c"
        );
        assert_eq!(join_with_final(", ", " and ", vec!["a", "b"]), "a and b");
        assert_eq!(join_with_final(", ", " and ", vec!["a"]), "a");
    }

    #[test]
    fn test_parse_rational_literal() {
        // 1e3
        assert_eq!(
            parse_rational_literal("1", "", "3").unwrap(),
            U256::from(1000)
        );
        // 123
        assert_eq!(
            parse_rational_literal("123", "", "").unwrap(),
            U256::from(123)
        );
        // 1.2e3
        assert_eq!(
            parse_rational_literal("1", "2", "3").unwrap(),
            U256::from(1200)
        );
        // 1.0
        assert_eq!(parse_rational_literal("1", "0", "").unwrap(), U256::from(1));
        // 1.01e3
        assert_eq!(
            parse_rational_literal("1", "01", "3").unwrap(),
            U256::from(1010)
        );
        // 1.1234e4
        assert_eq!(
            parse_rational_literal("1", "1234", "4").unwrap(),
            U256::from(11234)
        );
        // 1.12340e4
        assert_eq!(
            parse_rational_literal("1", "12340", "4").unwrap(),
            U256::from(11234)
        );
        // 1.1234e5
        assert_eq!(
            parse_rational_literal("1", "1234", "5").unwrap(),
            U256::from(112340)
        );
        // 1.01234e5
        assert_eq!(
            parse_rational_literal("1", "01234", "5").unwrap(),
            U256::from(101234)
        );
        // .1e3
        assert_eq!(
            parse_rational_literal("", "1", "3").unwrap(),
            U256::from(100)
        );
    }

    #[test]
    fn test_to_fixed_bytes() {
        assert_eq!(
            to_fixed_bytes(&[18, 52], 2, true).unwrap(),
            B256::from(U256::from(4660).checked_shl(240).unwrap().to_be_bytes())
        );
        assert_eq!(
            to_fixed_bytes(&[18, 52], 4, true).unwrap(),
            B256::from(U256::from(4660).checked_shl(240).unwrap().to_be_bytes())
        );
        assert_eq!(
            to_fixed_bytes(&[18, 52], 1, true).unwrap(),
            B256::from(U256::from(52).checked_shl(248).unwrap().to_be_bytes())
        );
        assert_eq!(
            to_fixed_bytes(&[102, 111, 111], 8, false).unwrap(),
            B256::from(
                U256::from(7381240360074215424u64)
                    .checked_shl(192)
                    .unwrap()
                    .to_be_bytes()
            ) // 0x666f6f0000000000
        );
    }

    #[test]
    fn test_decode_event() {
        let event = Event {
            name: "Transfer".to_string(),
            inputs: vec![
                EventParam {
                    ty: "address".to_string(),
                    name: "from".to_string(),
                    indexed: true,
                    components: vec![],
                    internal_type: None,
                },
                EventParam {
                    ty: "address".to_string(),
                    name: "to".to_string(),
                    indexed: true,
                    components: vec![],
                    internal_type: None,
                },
                EventParam {
                    ty: "uint256".to_string(),
                    name: "value".to_string(),
                    indexed: false,
                    components: vec![],
                    internal_type: None,
                },
            ],
            anonymous: false,
        };
        let log = _get_log();
        let decoded = decode_log_args(&log, &event).unwrap();
        assert_eq!(
            decoded,
            Value::NamedTuple(
                "Transfer".to_string(),
                HashableIndexMap(
                    vec![
                        (
                            "from".to_string(),
                            Value::Addr(address!("35641673a0ce64f644ad2f395be19668a06a5616"))
                        ),
                        (
                            "to".to_string(),
                            Value::Addr(address!("9748a9de5a2d81e96c2070f7f0d1d128bbb4d3c4"))
                        ),
                        (
                            "value".to_string(),
                            Value::Uint(U256::from_str("2270550663541970860349").unwrap(), 256)
                        ),
                    ]
                    .into_iter()
                    .collect()
                )
            )
        );
    }

    fn _get_log() -> Log {
        Log {
            inner: alloy::primitives::Log {
                address: address!("e07f9d810a48ab5c3c914ba3ca53af14e4491e8a"),
                data: LogData::new_unchecked(
                    vec![
                        b256!("ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"),
                        b256!("00000000000000000000000035641673a0ce64f644ad2f395be19668a06a5616"),
                        b256!("0000000000000000000000009748a9de5a2d81e96c2070f7f0d1d128bbb4d3c4"),
                    ],
                    bytes!("00000000000000000000000000000000000000000000007b1638669932a6793d"),
                ),
            },
            block_hash: Some(b256!(
                "d82cbdd9aba2827815d8db2e0665b1f54e6decc4f59042e53344f6562301e55b"
            )),
            block_number: Some(18735365),
            block_timestamp: None,
            transaction_hash: Some(b256!(
                "fb89e2333b81f2751eedaf2aeffb787917d42ea6ea7c5afd4d45371f3f1b8079"
            )),
            transaction_index: Some(134),
            log_index: Some(207),
            removed: false,
        }
    }
}
