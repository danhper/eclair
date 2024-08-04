use std::path::PathBuf;

use clap::Parser;

pub const ECLAIR_VERSION: &str = env!("ECLAIR_VERSION");

#[derive(Parser)]
#[command(version = ECLAIR_VERSION, about, long_about = None)]
pub struct Cli {
    /// Set the RPC URL to use
    #[arg(long, value_name = "URL", env = "ETH_RPC_URL")]
    pub rpc_url: Option<String>,

    /// Turn debugging information on
    #[arg(long, env = "DEBUG")]
    pub debug: bool,

    /// File where to store history
    #[arg(long, value_name = "FILE", env = "ECLAIR_HISTORY_FILE")]
    pub history_file: Option<PathBuf>,

    /// File where to store history
    #[arg(long, value_name = "FILE_NAME", env = "INIT_FILE_NAME")]
    pub init_file_name: Option<PathBuf>,
}
