use itertools::Itertools;
use std::sync::Arc;

use rustyline::{
    completion::{FilenameCompleter, Pair},
    Context,
};
use tokio::sync::Mutex;

use crate::interpreter::{ContractInfo, Env, Value};

pub(crate) struct MyCompleter {
    filename_completer: FilenameCompleter,
    env: Arc<Mutex<Env>>,
}

impl MyCompleter {
    pub fn new(env: Arc<Mutex<Env>>) -> Self {
        MyCompleter {
            filename_completer: FilenameCompleter::new(),
            env,
        }
    }
}

fn is_completing_path(line: &str, pos: usize) -> bool {
    for c in line[..pos].chars().rev() {
        if c == ' ' {
            return false;
        }
        if c == '/' {
            return true;
        }
    }
    false
}

fn get_current_word(line: &str, pos: usize) -> &str {
    let start = line[..pos].rfind(&[' ', '(', ',']).map_or(0, |i| i + 1);
    &line[start..pos]
}

fn is_completing_func_name(line: &str, pos: usize) -> bool {
    for c in line[..pos].chars().rev() {
        if c == ' ' || c == '(' {
            return false;
        }
        if c == '.' {
            return true;
        }
    }
    false
}

impl rustyline::completion::Completer for MyCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        if is_completing_path(line, pos) {
            return self.filename_completer.complete(line, pos, _ctx);
        }

        let env = self
            .env
            .clone()
            .try_lock_owned()
            .map_err(|_e| rustyline::error::ReadlineError::Interrupted)?;
        let mut types = env.list_types();
        let mut vars_and_types = env.list_vars();
        vars_and_types.append(&mut types);

        let current_word = get_current_word(line, pos);

        if is_completing_func_name(line, pos) {
            let (func_name, receiver) = current_word
                .rsplitn(2, '.')
                .map(|s| s.trim())
                .collect_tuple()
                .unwrap_or_default();

            if let Some(Value::Contract(ContractInfo(_, _, abi))) = env.get_var(receiver) {
                let completions = abi
                    .functions
                    .iter()
                    .map(|func| Pair {
                        display: func.0.clone(),
                        replacement: func.0.clone(),
                    })
                    .filter(|item| item.display.starts_with(func_name))
                    .collect::<Vec<_>>();
                return Ok((pos - func_name.len(), completions));
            }
        }

        let completions: Vec<_> = vars_and_types
            .iter()
            .map(|var| Pair {
                display: var.to_owned(),
                replacement: var.to_owned(),
            })
            .collect();

        let matches = completions
            .into_iter()
            .filter(|item| item.display.starts_with(current_word))
            .collect();
        Ok((pos - current_word.len(), matches))
    }
}
