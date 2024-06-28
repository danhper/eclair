use anyhow::Result;
use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::Editor;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::helper::{create_editor, history_file, MyHelper};
use super::Cli;
use crate::interpreter;
use crate::project;

pub struct Repl {
    rl: Editor<MyHelper, FileHistory>,
    env: Arc<Mutex<interpreter::Env>>,
    history_file: Option<PathBuf>,
}

impl Repl {
    pub async fn create(env: Arc<Mutex<interpreter::Env>>, cli: &Cli) -> Result<Self> {
        let rl = create_editor(env.clone())?;
        let history_file = cli.history_file.clone().or(history_file());
        let mut repl = Repl {
            rl,
            env,
            history_file,
        };
        repl._initialize_env().await?;

        Ok(repl)
    }

    async fn _initialize_env(&mut self) -> Result<()> {
        let mut env = self.env.lock().await;
        let current_dir = std::env::current_dir()?;
        let projects = project::load(current_dir);
        interpreter::load_builtins(&mut env);
        for project in projects.iter() {
            interpreter::load_project(&mut env, project)?;
        }
        Ok(())
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
        let mut env = self.env.lock().await;
        match interpreter::evaluate_line(&mut env, line.trim()).await {
            Ok(None) | Ok(Some(interpreter::Value::Null)) => (),
            Ok(Some(result)) => println!("{}", result),
            Err(e) => println!("Error: {:?}", e),
        }
    }
}
