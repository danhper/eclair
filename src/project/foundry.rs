use anyhow::{anyhow, bail, Result};
use ethers::abi::Contract;
use serde_json::Value;
use std::{collections::HashMap, fs::File, io::BufReader, path::Path, time::Instant};

pub struct FoundryProject {
    abis: HashMap<String, Contract>,
}

impl FoundryProject {
    fn new() -> Self {
        FoundryProject {
            abis: HashMap::new(),
        }
    }

    fn _load_abis_from_directory(&mut self, directory: &str) -> Result<()> {
        let files = glob::glob(
            Path::new(directory)
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
            serde_json::from_value(json["abi"].clone())?,
        );
        Ok(())
    }
}

impl super::types::Project for FoundryProject {
    fn load(directory: &str) -> Result<Self> {
        if !Self::is_valid_project(directory) {
            return Err(anyhow::anyhow!("Invalid project"));
        }
        let mut project = FoundryProject::new();
        project._load_abis_from_directory(directory)?;
        Ok(project)
    }

    fn is_valid_project(directory: &str) -> bool {
        Path::new(directory).join("foundry.toml").is_file()
    }

    fn get_contract(&self, name: &str) -> ethers::abi::Contract {
        self.abis.get(name).expect("Contract not found").clone()
    }

    fn contract_names(&self) -> Vec<String> {
        self.abis.keys().cloned().collect()
    }
}
