use itertools::Itertools;
use std::{path, sync::Arc};

use rustyline::{
    completion::{FilenameCompleter, Pair},
    Context,
};
use tokio::sync::Mutex;

use crate::interpreter::{Env, Type};

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

fn get_current_word(line: &str, pos: usize) -> (&str, usize) {
    let start = line[..pos]
        .rfind([' ', '(', ')', ',', '[', ']'])
        .map_or(0, |i| i + 1);
    (&line[start..pos], start)
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

fn pair_from_string(s: &str) -> Pair {
    Pair {
        display: s.to_owned(),
        replacement: s.to_owned(),
    }
}

fn get_function_completion(
    current_word: &str,
    pos: usize,
    env: &Env,
) -> rustyline::Result<(usize, Vec<Pair>)> {
    let (func_name, receiver) = current_word
        .rsplitn(2, '.')
        .map(|s| s.trim())
        .collect_tuple()
        .unwrap_or_default();

    let names = env
        .get_var(receiver)
        .map(|value| value.get_type())
        .or(env
            .get_type(receiver)
            .cloned()
            .map(|t| Type::Type(Box::new(t))))
        .map_or(vec![], |t| t.functions());

    let completions = names
        .iter()
        .filter(|name| name.starts_with(func_name))
        .map(|name| pair_from_string(name))
        .collect();

    Ok((pos - func_name.len(), completions))
}

impl rustyline::completion::Completer for MyCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let (current_word, current_word_start) = get_current_word(line, pos);

        if current_word.contains(path::MAIN_SEPARATOR) {
            let (pos, pairs) =
                self.filename_completer
                    .complete(current_word, pos - current_word_start, _ctx)?;
            return Ok((pos + current_word_start, pairs));
        }

        let env = self
            .env
            .clone()
            .try_lock_owned()
            .map_err(|_e| rustyline::error::ReadlineError::Interrupted)?;

        if is_completing_func_name(line, pos) {
            return get_function_completion(current_word, pos, &env);
        }

        let mut types = env.list_types();
        let mut builtins = Type::builtins();
        let mut vars_and_types = env.list_vars();
        vars_and_types.append(&mut types);
        vars_and_types.append(&mut builtins);

        let completions: Vec<_> = vars_and_types.iter().map(|s| pair_from_string(s)).collect();

        let matches = completions
            .into_iter()
            .filter(|item| item.display.starts_with(current_word))
            .collect();
        Ok((pos - current_word.len(), matches))
    }
}
