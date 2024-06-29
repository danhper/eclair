use std::path::PathBuf;

const SOREPL_HISTORY_FILE_NAME: &str = ".sorepl_history.txt";
const DEFAULT_INIT_FILE: &str = ".sorepl_init.sol";

pub fn history_file() -> Option<PathBuf> {
    foundry_config::Config::foundry_dir().map(|p| p.join(SOREPL_HISTORY_FILE_NAME))
}

pub fn get_init_files(init_file_name: &Option<PathBuf>) -> Vec<PathBuf> {
    let init_file = init_file_name
        .clone()
        .unwrap_or(PathBuf::from(DEFAULT_INIT_FILE));
    let current_dir = std::env::current_dir().ok();
    [foundry_config::Config::foundry_dir(), current_dir]
        .iter()
        .filter_map(|v| v.clone().map(|p| p.join(init_file.clone())))
        .filter(|p| p.exists())
        .collect()
}
