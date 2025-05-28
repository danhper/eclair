use std::collections::{BTreeMap, HashMap};

use foundry_config::Chain;

use crate::loaders::EtherscanConfig;
use anyhow::{anyhow, Result};

const DEFAULT_RPC_URL: &str = "http://localhost:8545";

#[derive(Debug, Clone)]
pub struct Config {
    pub rpc_url: String,
    pub debug: bool,
    pub rpc_endpoints: BTreeMap<String, String>,
    pub etherscan: HashMap<Chain, EtherscanConfig>,
}

impl Config {
    pub fn new(rpc_url: Option<String>, debug: bool, config: foundry_config::Config) -> Self {
        let rpc_endpoints: BTreeMap<_, _> = config
            .rpc_endpoints
            .resolved()
            .iter()
            .filter_map(|(k, v)| v.clone().ok().map(|v_| (k.clone(), v_)))
            .collect();
        let etherscan = config
            .etherscan
            .resolved()
            .iter()
            .filter_map(|(_k, v)| v.clone().ok().and_then(|c| c.chain.map(|cc| (cc, c))))
            .map(|(k, v)| (k, EtherscanConfig::new(v.key, v.api_url)))
            .collect();
        let rpc_url = rpc_url
            .or(rpc_endpoints.get("mainnet").cloned())
            .unwrap_or(DEFAULT_RPC_URL.to_string());
        Self {
            rpc_url,
            debug,
            rpc_endpoints,
            etherscan,
        }
    }

    pub fn get_etherscan_config(&self, chain_id: u64) -> Result<EtherscanConfig> {
        self.etherscan
            .get(&Chain::from_id(chain_id))
            .cloned()
            .ok_or(anyhow!("missing etherscan config"))
            .or_else(|_| EtherscanConfig::default_for_chain(chain_id))
            .or_else(|_| {
                let mainnet_key = self
                    .etherscan
                    .get(&Chain::mainnet())
                    .map(|c| c.api_key.clone())
                    .ok_or(anyhow!("missing mainnet etherscan key"))?;
                Ok(EtherscanConfig::with_key(chain_id, mainnet_key))
            })
    }
}
