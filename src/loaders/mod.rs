mod brownie;
pub mod etherscan;
pub mod file;
mod foundry;
pub mod four_bytes;
mod hardhat;
mod loader;
pub mod types;

use brownie::BrownieProjectLoader;
pub use etherscan::EtherscanConfig;
use foundry::FoundryProjectLoader;
use hardhat::HardhatProjectLoader;

pub fn load<P: AsRef<std::path::Path>>(directory: P) -> Vec<types::Project> {
    let loaders = [
        FoundryProjectLoader::new(),
        HardhatProjectLoader::new(),
        BrownieProjectLoader::new(),
    ];

    let mut projects = vec![];
    for loader in loaders.iter() {
        if loader.is_valid(directory.as_ref()) {
            match loader.load(directory.as_ref()) {
                Ok(abis) => projects.push(types::Project::new(abis)),
                Err(e) => eprintln!("Error loading {} project: {:?}", loader.name(), e),
            }
        }
    }

    projects
}
