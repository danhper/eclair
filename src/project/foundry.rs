use super::loader::ProjectLoader;
use anyhow::{anyhow, bail, Result};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub struct FoundryProjectLoader;

impl FoundryProjectLoader {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Box<dyn ProjectLoader> {
        Box::new(FoundryProjectLoader {})
    }
}

impl ProjectLoader for FoundryProjectLoader {
    fn name(&self) -> &'static str {
        "foundry"
    }

    fn abi_dirs(&self) -> Vec<PathBuf> {
        vec![Path::new("out").to_path_buf()]
    }

    fn get_contract_name(&self, json: &Value) -> Result<String> {
        let targets = json["metadata"]["settings"]["compilationTarget"]
            .as_object()
            .ok_or(anyhow!("invalid compilation target"))?;
        if targets.len() != 1 {
            bail!("invalid compilation target");
        }
        let target = targets.values().next().unwrap();
        target
            .as_str()
            .ok_or(anyhow!("invalid compilation target"))
            .map(|s| s.to_string())
    }

    fn should_exclude_file(&self, path: &Path) -> bool {
        path.to_str()
            .map_or(true, |f| f.contains(".s.sol") || f.contains(".t.sol"))
    }

    fn is_valid(&self, directory: &Path) -> bool {
        directory.join("foundry.toml").is_file()
    }
}
