pub trait Project {
    fn get_contract(&self, name: &str) -> ethers::abi::Contract;
    fn contract_names(&self) -> Vec<String>;
}
