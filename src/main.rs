use std::sync::Arc;

use anyhow::Result;
use clap::Parser;

use eclair::interpreter::{self, Config, Env};
use eclair::repl::{initialize_env, Cli, Repl, ECLAIR_VERSION};
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<()> {
    foundry_cli::utils::load_dotenv();

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) if err.kind() == clap::error::ErrorKind::DisplayVersion => {
            eprintln!("{}", ECLAIR_VERSION);
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    };

    let foundry_conf = foundry_config::load_config().map_err(anyhow::Error::msg)?;

    let config = Config::new(cli.rpc_url.clone(), cli.debug, foundry_conf);

    if let Some(script_file) = cli.script_file.as_ref() {
        let mut env = Env::new(config);
        initialize_env(&mut env, &cli.init_file_name).await?;
        let code = std::fs::read_to_string(script_file)?;
        let result = interpreter::evaluate_code(&mut env, &code).await?;
        match result {
            None | Some(interpreter::Value::Null) => (),
            Some(value) => println!("{}", value),
        }
        return Ok(());
    }

    let env = Arc::new(Mutex::new(Env::new(config)));
    let mut repl = Repl::create(env, &cli).await?;
    repl.run().await;

    Ok(())
}
