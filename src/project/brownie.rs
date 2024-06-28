use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

use super::loader::ProjectLoader;

pub struct BrownieProjectLoader;

impl BrownieProjectLoader {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Box<impl ProjectLoader> {
        Box::new(BrownieProjectLoader {})
    }
}

impl ProjectLoader for BrownieProjectLoader {
    fn name(&self) -> &'static str {
        "brownie"
    }

    fn abi_dirs(&self) -> Vec<PathBuf> {
        ["contracts", "interfaces", "libraries"]
            .iter()
            .map(|d| Path::new("build").join(d))
            .collect()
    }

    fn get_contract_name(&self, json: &serde_json::Value) -> Result<String> {
        json["contractName"]
            .as_str()
            .ok_or(anyhow!("invalid contract name"))
            .map(|s| s.to_string())
    }

    fn should_exclude_file(&self, _path: &Path) -> bool {
        false
    }

    fn is_valid(&self, directory: &Path) -> bool {
        directory.join("brownie-config.yaml").is_file()
    }
}
