use anyhow::Result;
use rustyline::error::ReadlineError;
use rustyline::sqlite_history::SQLiteHistory;
use rustyline::Editor;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::helper::{create_editor, MyHelper};
use crate::interpreter::{Env, Interpreter};
use crate::project::foundry::FoundryProject;

pub struct Repl {
    rl: Editor<MyHelper, SQLiteHistory>,
    interpreter: crate::interpreter::Interpreter,
}

impl Repl {
    pub async fn create(env: Arc<Mutex<Env>>) -> Result<Self> {
        let rl = create_editor(env.clone())?;
        let mut interpreter = Interpreter::new(env);
        let current_dir = std::env::current_dir()?;
        if FoundryProject::is_valid(&current_dir) {
            let project = FoundryProject::load(&current_dir)?;
            interpreter.load_project(Box::new(project)).await?;
        }
        Ok(Repl { rl, interpreter })
    }

    pub async fn run(&mut self) {
        loop {
            let p = ">> ";
            self.rl
                .helper_mut()
                .expect("No helper")
                .set_prompt(&format!("\x1b[1;32m{p}\x1b[0m"));
            let readline = self.rl.readline(p);
            match readline {
                Ok(line) => self.process_line(line.trim()).await,
                Err(ReadlineError::Interrupted) => break,
                Err(ReadlineError::Eof) => break,
                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            }
        }
    }

    async fn process_line(&mut self, line: &str) {
        if line.is_empty() {
            return;
        }
        match self.interpreter.evaluate_line(line.trim()).await {
            Ok(None) => (),
            Ok(Some(result)) => println!("{}", result),
            Err(e) => println!("Error: {:?}", e),
        }
    }
}
