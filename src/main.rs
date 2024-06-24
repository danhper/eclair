use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use foundry_cli::utils;

use sorepl::interpreter::Env;
use sorepl::repl::{Cli, Repl};
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<()> {
    utils::load_dotenv();

    let cli = Cli::try_parse()?;
    let env = Arc::new(Mutex::new(Env::new()));
    let mut repl = Repl::create(env, &cli).await?;
    repl.run().await;

    Ok(())
}
