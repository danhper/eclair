mod completer;
mod helper;

use anyhow::Result;
use helper::{create_editor, MyHelper};
use rustyline::error::ReadlineError;
use rustyline::sqlite_history::SQLiteHistory;
use rustyline::Editor;

use crate::interpreter::{EvalResult, Interpreter};

pub struct Repl {
    rl: Editor<MyHelper, SQLiteHistory>,
    interpreter: crate::interpreter::Interpreter,
}

impl Repl {
    pub fn create() -> Result<Self> {
        let rl = create_editor()?;
        let interpreter = Interpreter::new();
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
            Ok(EvalResult::Empty) => (),
            Ok(result) => println!("{}", result),
            Err(e) => println!("Error: {:?}", e),
        }
    }
}
