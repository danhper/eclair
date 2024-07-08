use std::{fs, path::Path};

use alloy::json_abi::JsonAbi;
use anyhow::Result;
use serde_json::Value;

pub fn load_abi<P>(filepath: P, key: Option<&str>) -> Result<JsonAbi>
where
    P: AsRef<Path>,
{
    let expanded_path = shellexpand::path::full(filepath.as_ref())?;
    let file_content = fs::read_to_string(expanded_path)?;
    if let Some(key) = key {
        let json: Value = serde_json::from_str(&file_content)?;
        JsonAbi::from_json_str(&json[key].to_string()).map_err(Into::into)
    } else {
        JsonAbi::from_json_str(&file_content).map_err(Into::into)
    }
}
