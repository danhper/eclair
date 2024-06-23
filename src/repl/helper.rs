use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use rustyline::{
    sqlite_history::SQLiteHistory, validate::MatchingBracketValidator, Completer, Config, Editor,
    Helper, Highlighter, Hinter, Validator,
};
use tokio::sync::Mutex;

use crate::interpreter::Env;
use crate::repl::completer::MyCompleter;

const SOREPL_HISTORY_FILE_NAME: &str = ".sorepl_history.sqlite3";

#[derive(Helper, Completer, Hinter, Validator, Highlighter)]
pub(crate) struct MyHelper {
    #[rustyline(Completer)]
    completer: MyCompleter,
    #[rustyline(Highlighter)]
    highlighter: (),
    #[rustyline(Validator)]
    validator: MatchingBracketValidator,
    colored_prompt: String,
}

impl MyHelper {
    pub fn new(env: Arc<Mutex<Env>>) -> Self {
        MyHelper {
            completer: MyCompleter::new(env),
            highlighter: (),
            colored_prompt: ">> ".to_owned(),
            validator: MatchingBracketValidator::new(),
        }
    }

    pub fn set_prompt(&mut self, prompt: &str) {
        prompt.clone_into(&mut self.colored_prompt)
    }
}

fn history_file() -> Option<PathBuf> {
    foundry_config::Config::foundry_dir().map(|p| p.join(SOREPL_HISTORY_FILE_NAME))
}

pub(crate) fn create_editor(env: Arc<Mutex<Env>>) -> Result<Editor<MyHelper, SQLiteHistory>> {
    let config = Config::builder()
        .completion_type(rustyline::CompletionType::List)
        .auto_add_history(true)
        .build();
    let helper = MyHelper::new(env);
    let history_file_path = history_file().ok_or(anyhow!("Could not find foundry directory"))?;
    let history = rustyline::sqlite_history::SQLiteHistory::open(config, &history_file_path)?;
    let mut rl: Editor<MyHelper, _> = Editor::with_history(config, history)?;

    rl.set_helper(Some(helper));
    Ok(rl)
}
