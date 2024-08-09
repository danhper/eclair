use std::sync::Arc;

use alloy::{primitives::B256, rpc::types::Filter};
use anyhow::{bail, Result};
use futures::{future::BoxFuture, FutureExt};
use lazy_static::lazy_static;

use crate::interpreter::{functions::FunctionDef, types::LOG_TYPE, utils, Env, Type, Value};

#[derive(Debug)]
struct EventOptions {
    topic0: Option<B256>,
    topic1: Option<B256>,
    topic2: Option<B256>,
    topic3: Option<B256>,
    from_block: Option<u64>,
    to_block: Option<u64>,
}

impl TryFrom<&crate::interpreter::types::HashableIndexMap<String, Value>> for EventOptions {
    type Error = anyhow::Error;

    fn try_from(map: &crate::interpreter::types::HashableIndexMap<String, Value>) -> Result<Self> {
        let topic0 = map.0.get("topic0").map(|v| v.as_b256()).transpose()?;
        let topic1 = map.0.get("topic1").map(|v| v.as_b256()).transpose()?;
        let topic2 = map.0.get("topic2").map(|v| v.as_b256()).transpose()?;
        let topic3 = map.0.get("topic3").map(|v| v.as_b256()).transpose()?;
        let from_block = map.0.get("fromBlock").map(|v| v.as_u64()).transpose()?;
        let to_block = map.0.get("toBlock").map(|v| v.as_u64()).transpose()?;

        Ok(EventOptions {
            topic0,
            topic1,
            topic2,
            topic3,
            from_block,
            to_block,
        })
    }
}

fn fetch_events<'a>(
    env: &'a mut Env,
    args: &'a [Value],
    options: EventOptions,
) -> BoxFuture<'a, Result<Value>> {
    async move {
        let mut filter = Filter::new();
        if let Some(topic0) = options.topic0 {
            filter = filter.event_signature(topic0);
        }
        if let Some(topic1) = options.topic1 {
            filter = filter.topic1(topic1);
        }
        if let Some(topic2) = options.topic2 {
            filter = filter.topic2(topic2);
        }
        if let Some(topic3) = options.topic3 {
            filter = filter.topic3(topic3);
        }
        if let Some(from_block) = options.from_block {
            filter = filter.from_block(from_block);
        } else {
            filter = filter.from_block(0);
        }
        if let Some(to_block) = options.to_block {
            filter = filter.to_block(to_block);
        }

        match args {
            [Value::Addr(addr)] => filter = filter.address(*addr),
            [Value::Array(addrs, ty_)] if ty_.as_ref() == &Type::Address => {
                let addresses = addrs
                    .iter()
                    .map(|a| a.as_address())
                    .collect::<Result<Vec<_>>>()?;
                filter = filter.address(addresses)
            }
            _ => bail!("events.fetch: invalid arguments"),
        }

        let logs = env.get_provider().get_logs(&filter).await?;
        let parsed_logs = logs
            .into_iter()
            .map(|log| utils::log_to_value(env, log))
            .collect::<Result<Vec<Value>>>()?;
        Ok(Value::Array(parsed_logs, Box::new(LOG_TYPE.clone())))
    }
    .boxed()
}

#[derive(Debug)]
struct FetchEvents;

impl FunctionDef for FetchEvents {
    fn name(&self) -> String {
        "fetch".to_string()
    }

    fn get_valid_args(
        &self,
        _receiver: &Option<Value>,
    ) -> Vec<Vec<crate::interpreter::functions::FunctionParam>> {
        vec![
            vec![crate::interpreter::functions::FunctionParam::new(
                "address",
                Type::Address,
            )],
            vec![crate::interpreter::functions::FunctionParam::new(
                "addresses",
                Type::Array(Box::new(Type::Address)),
            )],
        ]
    }

    fn is_property(&self) -> bool {
        false
    }

    fn execute<'a>(
        &'a self,
        env: &'a mut Env,
        values: &'a [Value],
        options: &'a crate::interpreter::types::HashableIndexMap<String, Value>,
    ) -> BoxFuture<'a, Result<Value>> {
        async move {
            let parsed_opts = options.try_into()?;
            fetch_events(env, &values[1..], parsed_opts).await
        }
        .boxed()
    }
}

lazy_static! {
    pub static ref FETCH_EVENTS: Arc<dyn FunctionDef> = Arc::new(FetchEvents);
}
