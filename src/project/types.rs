pub trait Project {
    fn get_contract(&self, name: &str) -> alloy::json_abi::JsonAbi;
    fn contract_names(&self) -> Vec<String>;
}
