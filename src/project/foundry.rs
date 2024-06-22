use alloy::json_abi::JsonAbi;
use anyhow::{anyhow, bail, Result};
use serde_json::Value;
use std::{collections::HashMap, fs::File, io::BufReader, path::Path};

pub struct FoundryProject {
    abis: HashMap<String, JsonAbi>,
}

impl FoundryProject {
    pub fn load<P: AsRef<Path>>(directory: P) -> Result<Self> {
        if !Self::is_valid(&directory) {
            return Err(anyhow::anyhow!("Invalid project"));
        }
        let mut project = FoundryProject::new();
        project._load_abis_from_directory(&directory)?;
        Ok(project)
    }

    fn new() -> Self {
        FoundryProject {
            abis: HashMap::new(),
        }
    }

    pub fn is_valid<P: AsRef<Path>>(directory: P) -> bool {
        Path::new(directory.as_ref()).join("foundry.toml").is_file()
    }

    fn _load_abis_from_directory<P: AsRef<Path>>(&mut self, directory: P) -> Result<()> {
        let files = glob::glob(
            Path::new(directory.as_ref())
                .join("out")
                .join("**/*.json")
                .to_str()
                .unwrap(),
        )?;
        for file in files {
            let file = file?;
            let filepath = file.to_str().unwrap();
            if filepath.contains(".t.sol/") || filepath.contains(".s.sol/") {
                continue;
            }
            self._load_abi_from_file(filepath)?;
        }
        Ok(())
    }

    fn _load_abi_from_file(&mut self, filepath: &str) -> Result<()> {
        let file = File::open(filepath)?;
        let reader = BufReader::new(file);

        let json: Value = serde_json::from_reader(reader)?;
        let targets = json["metadata"]["settings"]["compilationTarget"]
            .as_object()
            .ok_or(anyhow!("invalid compilation target {}", filepath))?;
        if targets.len() != 1 {
            bail!("invalid compilation target");
        }
        let contract_name = targets.values().next().unwrap().as_str().unwrap();
        self.abis.insert(
            contract_name.to_string(),
            JsonAbi::from_json_str(&json["abi"].to_string())?,
            // serde_json::from_value(json["abi"].clone())?, // TODO: figure out why this doesn't work
        );
        Ok(())
    }
}

impl Default for FoundryProject {
    fn default() -> Self {
        Self::new()
    }
}

impl super::types::Project for FoundryProject {
    fn get_contract(&self, name: &str) -> JsonAbi {
        self.abis.get(name).expect("Contract not found").clone()
    }

    fn contract_names(&self) -> Vec<String> {
        self.abis.keys().cloned().collect()
    }
}
