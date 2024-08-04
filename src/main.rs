use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use foundry_cli::utils;

use eclair::interpreter::Config;
use eclair::interpreter::Env;
use eclair::repl::{Cli, Repl, ECLAIR_VERSION};
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<()> {
    utils::load_dotenv();

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) if err.kind() == clap::error::ErrorKind::DisplayVersion => {
            eprintln!("{}", ECLAIR_VERSION);
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    };

    let foundry_conf = foundry_config::load_config();

    let config = Config::new(cli.rpc_url.clone(), cli.debug, foundry_conf);
    let env = Arc::new(Mutex::new(Env::new(config)));
    let mut repl = Repl::create(env, &cli).await?;
    repl.run().await;

    Ok(())
}
