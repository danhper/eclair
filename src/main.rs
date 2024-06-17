use std::{cell::RefCell, rc::Rc};

use anyhow::Result;
use foundry_cli::{handler, utils};

use sorepl::interpreter::Env;
use sorepl::repl::Repl;

fn main() -> Result<()> {
    handler::install();
    utils::load_dotenv();

    let env = Rc::new(RefCell::new(Env::new()));
    let mut repl = Repl::create(env)?;
    repl.run();

    Ok(())
}
