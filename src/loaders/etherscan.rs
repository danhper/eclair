use alloy::{json_abi::JsonAbi, transports::http::reqwest};
use anyhow::{anyhow, bail, Result};
use serde_json::Value;

pub async fn load_abi(chain_id: u64, address: &str) -> Result<JsonAbi> {
    let base_url = get_base_url(chain_id)?;
    let api_key = api_key_from_env(chain_id)?;
    let url = format!(
        "{}?module=contract&action=getabi&address={}&apikey={}",
        base_url, address, api_key
    );
    let value = reqwest::get(&url).await?.json::<Value>().await?;
    let abi_str = value["result"]
        .as_str()
        .ok_or(anyhow!("failed to fetch ABI"))?;
    JsonAbi::from_json_str(abi_str).map_err(|e| anyhow!(e))
}

fn get_base_url(chain_id: u64) -> Result<&'static str> {
    let url = match chain_id {
        1 => "https://api.etherscan.io/api",                // Ethereum
        10 => "https://api-optimistic.etherscan.io/api",    // Optimism
        100 => "https://api.gnosisscan.io/api",             // Gnosis Chain
        137 => "https://api.polygonscan.com/api",           // Polygon
        1101 => "https://api-zkevm.polygonscan.com/api",    // Polygon zkEVM
        8453 => "https://api.basescan.org/api",             // Base
        42161 => "https://api.arbiscan.io/api",             // Arbitrum
        11155111 => "https://api-sepolia.etherscan.io/api", // Sepolia
        _ => bail!("chain id {} not supported", chain_id),
    };
    Ok(url)
}

fn api_key_from_env(chain_id: u64) -> Result<String> {
    let mut key = match chain_id {
        1 => std::env::var("ETHERSCAN_API_KEY"),
        10 => std::env::var("OP_ETHERSCAN_API_KEY"),
        100 => std::env::var("GNOSISSCAN_API_KEY"),
        137 => std::env::var("POLYGON_API_KEY"),
        1101 => std::env::var("POLYGON_ZKEVM_API_KEY"),
        8453 => std::env::var("BASESCAN_API_KEY"),
        42161 => std::env::var("ARBISCAN_API_KEY"),
        11155111 => std::env::var("SEPOLIA_ETHERSCAN_API_KEY"),
        _ => bail!("chain id {} not supported", chain_id),
    };
    if key.is_err() {
        key = std::env::var("ETHERSCAN_API_KEY");
    }
    key.map_err(|_| anyhow!("missing API key for chain id {}", chain_id))
}
