use crate::interpreter::Env;
use std::{cell::RefCell, rc::Rc};

use rustyline::{
    completion::{FilenameCompleter, Pair},
    Context,
};

pub(crate) struct MyCompleter {
    filename_completer: FilenameCompleter,
    env: Rc<RefCell<Env>>,
}

impl MyCompleter {
    pub fn new(env: Rc<RefCell<Env>>) -> Self {
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
    let start = line[..pos].rfind(' ').map_or(0, |i| i + 1);
    &line[start..pos]
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

        let mut types = self.env.borrow().list_types();
        let mut vars_and_types = self.env.borrow().list_vars();
        vars_and_types.append(&mut types);

        let completions: Vec<_> = vars_and_types
            .iter()
            .map(|var| Pair {
                display: var.to_owned(),
                replacement: var.to_owned(),
            })
            .collect();

        let current_word = get_current_word(line, pos);

        let matches = completions
            .into_iter()
            .filter(|item| item.display.starts_with(current_word))
            .collect();
        Ok((pos - current_word.len(), matches))
    }
}
