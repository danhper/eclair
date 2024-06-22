use std::sync::Arc;

use anyhow::Result;
use foundry_cli::{handler, utils};

use sorepl::interpreter::Env;
use sorepl::repl::Repl;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<()> {
    handler::install();
    utils::load_dotenv();

    let env = Arc::new(Mutex::new(Env::new()));
    let mut repl = Repl::create(env).await?;
    repl.run().await;

    Ok(())
}
