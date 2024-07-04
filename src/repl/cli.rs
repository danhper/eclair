use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Set the RPC URL to use
    #[arg(
        long,
        value_name = "URL",
        default_value = "http://localhost:8545",
        env = "ETH_RPC_URL"
    )]
    pub rpc_url: String,

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
