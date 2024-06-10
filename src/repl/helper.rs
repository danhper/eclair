use anyhow::Result;
use rustyline::{
    sqlite_history::SQLiteHistory, validate::MatchingBracketValidator, Completer, Config, Editor,
    Helper, Highlighter, Hinter, Validator,
};

use crate::repl::completer::MyCompleter;

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
    pub fn new() -> Self {
        MyHelper {
            completer: MyCompleter::new(),
            highlighter: (),
            colored_prompt: ">> ".to_owned(),
            validator: MatchingBracketValidator::new(),
        }
    }

    pub fn set_prompt(&mut self, prompt: &str) {
        self.colored_prompt = prompt.to_owned();
    }
}

pub(crate) fn create_editor() -> Result<Editor<MyHelper, SQLiteHistory>> {
    let config = Config::builder()
        .completion_type(rustyline::CompletionType::List)
        .auto_add_history(true)
        .build();
    let history = rustyline::sqlite_history::SQLiteHistory::open(config, "tmp/history.sqlite3")?;
    let helper = MyHelper::new();
    let mut rl: Editor<MyHelper, _> = Editor::with_history(config, history)?;
    rl.set_helper(Some(helper));
    Ok(rl)
}
