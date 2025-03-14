use alloy::{json_abi::Function, primitives::FixedBytes, transports::http::reqwest};
use anyhow::{anyhow, bail, Result};
use itertools::Itertools;
use serde_json::Value;

const FOUR_BYTES_API_URL: &str = "https://www.4byte.directory/api/v1/signatures/";

async fn get_results(selector_str: &str) -> Result<Vec<Value>> {
    let url = format!("{}?hex_signature={}", FOUR_BYTES_API_URL, selector_str);
    let response = reqwest::get(url).await?;
    let body = response.json::<Value>().await?;
    body["results"]
        .as_array()
        .cloned()
        .ok_or(anyhow!("No results found for selector {}", selector_str))
}

pub async fn find_function(selector: FixedBytes<4>) -> Result<Function> {
    // NOTE: 4byte.directory API seems to be senstitive to 0x prefix and is not consistent across functions
    let mut results = get_results(&selector.to_string()).await?;
    if results.is_empty() {
        results = get_results(&selector.to_string()[2..]).await?;
    }
    if results.is_empty() {
        bail!("No results found for selector {}", selector);
    }
    let desired_result = results
        .iter()
        .sorted_by_key(|r| r["id"].as_u64()) // get first registered signature
        .next()
        .unwrap();
    let signature = desired_result["text_signature"]
        .as_str()
        .ok_or(anyhow!("No text signature found for selector {}", selector))?;
    Function::parse(signature).map_err(|e| anyhow!(e))
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[tokio::test]
    async fn test_find_function() {
        let selector = FixedBytes::from_str("0x1bcf634e").unwrap();
        let function = find_function(selector).await.unwrap();
        assert_eq!(function.name, "executeL2Proposal");
    }
}
