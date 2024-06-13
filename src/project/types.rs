use anyhow::Result;

pub trait Project {
    fn load(directory: &str) -> Result<Self>
    where
        Self: std::marker::Sized;
    fn is_valid_project(directory: &str) -> bool;
    fn get_contract(&self, name: &str) -> ethers::abi::Contract;
    fn contract_names(&self) -> Vec<String>;
}
