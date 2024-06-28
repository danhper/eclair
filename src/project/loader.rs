use alloy::json_abi::JsonAbi;
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

pub trait ProjectLoader {
    fn name(&self) -> &'static str;

    fn get_contract_name(&self, value: &serde_json::Value) -> Result<String>;
    fn is_valid(&self, directory: &Path) -> bool;
    fn should_exclude_file(&self, path: &Path) -> bool;
    fn abi_dirs(&self) -> Vec<PathBuf>;

    fn load_abi_from_file(&self, filepath: &Path) -> Result<(String, JsonAbi)> {
        let file = File::open(filepath)?;
        let reader = BufReader::new(file);
        let json: Value = serde_json::from_reader(reader)?;
        let contract_name = self.get_contract_name(&json)?;
        Ok((
            contract_name.to_string(),
            JsonAbi::from_json_str(&json["abi"].to_string())?,
            // serde_json::from_value(json["abi"].clone())?, // TODO: figure out why this doesn't work
        ))
    }

    fn get_abi_files(&self, directory: &Path) -> Result<Vec<String>> {
        let files = glob::glob(Path::new(directory).join("**/*.json").to_str().unwrap())?;
        let mut result = vec![];

        for file in files {
            let file = file?;
            if !self.should_exclude_file(&file) {
                let filepath = file.to_str().ok_or(anyhow!("Invalid file path"))?;
                result.push(filepath.to_string());
            }
        }
        Ok(result)
    }

    fn load(&self, directory: &Path) -> Result<HashMap<String, JsonAbi>> {
        if !self.is_valid(directory) {
            return Err(anyhow::anyhow!("Invalid project"));
        }
        let mut abis = HashMap::new();
        for abi_dir in self.abi_dirs() {
            for filepath in self.get_abi_files(&directory.join(abi_dir))? {
                if let Ok((contract_name, abi)) = self.load_abi_from_file(Path::new(&filepath)) {
                    abis.insert(contract_name, abi);
                }
            }
        }
        Ok(abis)
    }
}
