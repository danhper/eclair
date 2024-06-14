use super::helper::{create_editor, MyHelper};
use anyhow::Result;
use rustyline::error::ReadlineError;
use rustyline::sqlite_history::SQLiteHistory;
use rustyline::Editor;

use crate::interpreter::Interpreter;
use crate::project::foundry::FoundryProject;

pub struct Repl {
    rl: Editor<MyHelper, SQLiteHistory>,
    interpreter: crate::interpreter::Interpreter,
}

impl Repl {
    pub fn create() -> Result<Self> {
        let rl = create_editor()?;
        let mut interpreter = Interpreter::new();
        let current_dir = std::env::current_dir()?;
        if FoundryProject::is_valid(&current_dir) {
            let project = FoundryProject::load(&current_dir)?;
            interpreter.load_project(Box::new(project))?;
        }
        Ok(Repl { rl, interpreter })
    }

    pub fn run(&mut self) {
        loop {
            let p = ">> ";
            self.rl
                .helper_mut()
                .expect("No helper")
                .set_prompt(&format!("\x1b[1;32m{p}\x1b[0m"));
            let readline = self.rl.readline(p);
            match readline {
                Ok(line) => self.process_line(line.trim()),
                Err(ReadlineError::Interrupted) => break,
                Err(ReadlineError::Eof) => break,
                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            }
        }
    }

    fn process_line(&mut self, line: &str) {
        if line.is_empty() {
            return;
        }
        match self.interpreter.evaluate_line(line.trim()) {
            Ok(None) => (),
            Ok(Some(result)) => println!("{}", result),
            Err(e) => println!("Error: {:?}", e),
        }
    }
}
