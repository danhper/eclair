use anyhow::Result;
use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::Editor;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::helper::{create_editor, history_file, MyHelper};
use super::Cli;
use crate::interpreter::{Env, Interpreter};
use crate::project::foundry::FoundryProject;

pub struct Repl {
    rl: Editor<MyHelper, FileHistory>,
    interpreter: crate::interpreter::Interpreter,
    history_file: Option<PathBuf>,
}

impl Repl {
    pub async fn create(env: Arc<Mutex<Env>>, cli: &Cli) -> Result<Self> {
        let rl = create_editor(env.clone())?;
        let mut interpreter = Interpreter::new(env, &cli.rpc_url, cli.debug);
        let current_dir = std::env::current_dir()?;
        if FoundryProject::is_valid(&current_dir) {
            let project = FoundryProject::load(&current_dir)?;
            interpreter.load_project(Box::new(project)).await?;
        }
        let history_file = cli.history_file.clone().or(history_file());
        Ok(Repl {
            rl,
            interpreter,
            history_file,
        })
    }

    pub async fn run(&mut self) {
        if let Some(history_file) = &self.history_file {
            let _ = self.rl.load_history(history_file);
        }

        self.run_repl().await;

        if let Some(history_file) = &self.history_file {
            let _ = self.rl.save_history(&history_file);
        }
    }

    async fn run_repl(&mut self) {
        loop {
            let p = ">> ";
            self.rl
                .helper_mut()
                .expect("No helper")
                .set_prompt(&format!("\x1b[1;32m{p}\x1b[0m"));
            let readline = self.rl.readline(p);
            match readline {
                Ok(line) => self.process_line(line.trim()).await,
                Err(ReadlineError::Interrupted) => continue,
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
