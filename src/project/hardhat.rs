use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

use super::loader::ProjectLoader;

pub struct HardhatProjectLoader;

impl HardhatProjectLoader {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Box<impl ProjectLoader> {
        Box::new(HardhatProjectLoader {})
    }
}

impl ProjectLoader for HardhatProjectLoader {
    fn name(&self) -> &'static str {
        "hardhat"
    }

    fn abi_dirs(&self) -> Vec<PathBuf> {
        vec![Path::new("artifacts").to_path_buf()]
    }

    fn get_contract_name(&self, json: &serde_json::Value) -> Result<String> {
        json["contractName"]
            .as_str()
            .ok_or(anyhow!("invalid contract name"))
            .map(|s| s.to_string())
    }

    fn should_exclude_file(&self, path: &Path) -> bool {
        path.to_str().map_or(true, |f| f.contains(".dbg.json"))
    }

    fn is_valid(&self, directory: &Path) -> bool {
        let files = ["hardhat.config.ts", "hardhat.config.js"];
        files.iter().any(|file| directory.join(file).is_file())
    }
}
