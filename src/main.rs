use std::env;

use anyhow::Result;

use sorepl::{project::types::Project, repl::Repl};

fn main() -> Result<()> {
    // let mut repl = Repl::create()?;
    // repl.run();
    let project_path = env::home_dir()
        .unwrap()
        .join("workspace/organizations/tlx/protocol-dev");
    let project =
        sorepl::project::foundry::FoundryProject::load(project_path.to_owned().to_str().unwrap())?;
    println!("{:?}", project.contract_names());

    Ok(())
}
