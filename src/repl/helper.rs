use std::sync::Arc;

use anyhow::Result;
use rustyline::history::FileHistory;
use rustyline::{
    highlight::Highlighter, validate::MatchingBracketValidator, Completer, Config, Editor, Helper,
    Hinter, Validator,
};
use tokio::sync::Mutex;

use crate::interpreter::Env;
use crate::repl::completer::MyCompleter;

#[derive(Helper, Completer, Hinter, Validator)]
pub(crate) struct MyHelper {
    #[rustyline(Completer)]
    completer: MyCompleter,
    #[rustyline(Validator)]
    validator: MatchingBracketValidator,
    colored_prompt: String,
}

impl MyHelper {
    pub fn new(env: Arc<Mutex<Env>>) -> Self {
        MyHelper {
            completer: MyCompleter::new(env),
            colored_prompt: ">> ".to_owned(),
            validator: MatchingBracketValidator::new(),
        }
    }

    pub fn set_prompt(&mut self, prompt: &str) {
        prompt.clone_into(&mut self.colored_prompt)
    }
}

pub(crate) fn create_editor(env: Arc<Mutex<Env>>) -> Result<Editor<MyHelper, FileHistory>> {
    let config = Config::builder()
        .completion_type(rustyline::CompletionType::List)
        .auto_add_history(true)
        .build();
    let helper = MyHelper::new(env);
    let history = FileHistory::default();
    let mut rl: Editor<MyHelper, _> = Editor::with_history(config, history)?;

    rl.set_helper(Some(helper));
    Ok(rl)
}

impl Highlighter for MyHelper {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> std::borrow::Cow<'l, str> {
        chisel::solidity_helper::SolidityHelper::highlight(line)
    }
}
