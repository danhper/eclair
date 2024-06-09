mod completer;
mod helper;
mod parsing;

use anyhow::Result;
use helper::{create_editor, MyHelper};
use rustyline::error::ReadlineError;
use rustyline::sqlite_history::SQLiteHistory;
use rustyline::Editor;

pub struct Repl {
    rl: Editor<MyHelper, SQLiteHistory>,
}

impl Repl {
    pub fn create() -> Result<Self> {
        let rl = create_editor()?;
        Ok(Repl { rl })
    }

    pub fn run(&mut self) {
        let mut interpreter = crate::interpreter::Interpreter::new();

        loop {
            let p = ">> ";
            self.rl
                .helper_mut()
                .expect("No helper")
                .set_prompt(&format!("\x1b[1;32m{p}\x1b[0m"));
            let readline = self.rl.readline(p);
            match readline {
                Ok(line) => match parsing::parse_statement(&line) {
                    Ok(stmt) => {
                        println!("{:#?}", stmt);
                        match interpreter.evaluate_statement(&stmt) {
                            Ok(result) => {
                                println!("{}", result);
                            }
                            Err(e) => println!("{}", e),
                        }
                    }
                    Err(e) => {
                        println!("{}", e);
                    }
                },
                Err(ReadlineError::Interrupted) => break,
                Err(ReadlineError::Eof) => break,
                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            }
        }
    }
}
