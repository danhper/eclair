use alloy::{json_abi::JsonAbi, transports::http::reqwest};
use anyhow::{anyhow, Result};
use serde_json::Value;

const API_URL: &str = "https://api.etherscan.io/v2/api";

#[derive(Debug, Clone)]
pub struct EtherscanConfig {
    pub api_key: String,
    pub base_url: String,
}

impl EtherscanConfig {
    fn new(api_key: String, base_url: String) -> Self {
        Self { api_key, base_url }
    }

    pub fn default_for_chain(chain_id: u64) -> Result<Self> {
        let base_url = get_base_url(chain_id);
        let api_key = api_key_from_env(chain_id)?;
        Ok(Self::new(api_key, base_url))
    }

    pub fn with_key(chain_id: u64, api_key: String) -> Self {
        Self {
            api_key,
            base_url: get_base_url(chain_id),
        }
    }
}

pub async fn load_abi(config: EtherscanConfig, address: &str) -> Result<JsonAbi> {
    let separator = if config.base_url.contains("?") {
        "&"
    } else {
        "?"
    };
    let url = format!(
        "{}{}module=contract&action=getabi&address={}&apikey={}",
        config.base_url, separator, address, config.api_key
    );
    let value = reqwest::get(&url).await?.json::<Value>().await?;
    let abi_str = value["result"]
        .as_str()
        .ok_or(anyhow!("failed to fetch ABI"))?;
    JsonAbi::from_json_str(abi_str).map_err(|e| anyhow!(e))
}

fn get_base_url(chain_id: u64) -> String {
    format!("{}?chainid={}", API_URL, chain_id)
}

fn api_key_from_env(chain_id: u64) -> Result<String> {
    if let Ok(key) = std::env::var("ETHERSCAN_API_KEY") {
        return Ok(key);
    }

    // Etherscan API v2 only needs one API key
    // This is left for backwards compatibility
    let key_name = match chain_id {
        10 => "OP_ETHERSCAN_API_KEY",
        100 => "GNOSISSCAN_API_KEY",
        137 => "POLYGONSCAN_API_KEY",
        1329 => "SEITRACE_API_KEY",
        1101 => "POLYGONSCAN_ZKEVM_API_KEY",
        8453 => "BASESCAN_API_KEY",
        42161 => "ARBISCAN_API_KEY",
        11155111 => "SEPOLIA_ETHERSCAN_API_KEY",
        _ => "ETHERSCAN_API_KEY",
    };
    std::env::var(key_name).map_err(|_| anyhow!("missing API key for chain id {}", chain_id))
}
