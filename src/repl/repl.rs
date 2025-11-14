use anyhow::Result;
use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::{Config, Editor};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::config::{get_init_files, history_file};
use super::solidity_helper::SolidityHelper;
use super::Cli;
use crate::interpreter::{self, Env};
use crate::loaders;

fn create_editor(env: Arc<Mutex<Env>>) -> Result<Editor<SolidityHelper, FileHistory>> {
    let config = Config::builder()
        .completion_type(rustyline::CompletionType::List)
        .auto_add_history(true)
        .build();
    let helper = SolidityHelper::new(env);
    let history = FileHistory::default();
    let mut rl: Editor<SolidityHelper, _> = Editor::with_history(config, history)?;

    rl.set_helper(Some(helper));
    Ok(rl)
}

pub struct Repl {
    rl: Editor<SolidityHelper, FileHistory>,
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

        repl._initialize_env(&cli.init_file_name).await?;

        Ok(repl)
    }

    async fn _initialize_env(&mut self, init_file_name: &Option<PathBuf>) -> Result<()> {
        let mut env = self.env.lock().await;
        let current_dir = std::env::current_dir()?;
        let projects = loaders::load(current_dir);
        interpreter::load_builtins(&mut env);
        for project in projects.iter() {
            interpreter::load_project(&mut env, project)?;
        }

        let init_files = get_init_files(init_file_name);
        for init_file in init_files.iter() {
            let code = std::fs::read_to_string(init_file)?;
            interpreter::evaluate_setup(&mut env, &code).await?;
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
            let readline = self.rl.readline(p);
            self.rl.helper_mut().unwrap().set_errored(false);
            match readline {
                Ok(line) => self.process_line(line.trim()).await,
                Err(ReadlineError::Interrupted) => continue,
                Err(ReadlineError::Eof) => break,
                Err(err) => {
                    self.rl.helper_mut().unwrap().set_errored(true);
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
        match interpreter::evaluate_code(&mut env, line.trim()).await {
            Ok(None) | Ok(Some(interpreter::Value::Null)) => (),
            Ok(Some(result)) => println!("{}", result),
            Err(e) => println!("Error: {:?}", e),
        }
    }
}
