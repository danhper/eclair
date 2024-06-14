use anyhow::Result;

use foundry_cli::{handler, utils};
use sorepl::repl::Repl;

fn main() -> Result<()> {
    handler::install();
    utils::load_dotenv();

    let mut repl = Repl::create()?;
    repl.run();

    Ok(())
}
