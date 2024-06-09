use anyhow::Result;

use sorepl::repl::Repl;

fn main() -> Result<()> {
    let mut repl = Repl::create()?;
    repl.run();
    Ok(())
}
