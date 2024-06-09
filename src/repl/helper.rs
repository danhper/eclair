use anyhow::Result;
use rustyline::{
    highlight::{Highlighter, MatchingBracketHighlighter},
    sqlite_history::SQLiteHistory,
    validate::MatchingBracketValidator,
    Completer, Config, Editor, Helper, Hinter, Validator,
};
use std::borrow::Cow::{self, Borrowed, Owned};

use crate::repl::completer::MyCompleter;

#[derive(Helper, Completer, Hinter, Validator)]
pub(crate) struct MyHelper {
    #[rustyline(Completer)]
    completer: MyCompleter,
    highlighter: MatchingBracketHighlighter,
    #[rustyline(Validator)]
    validator: MatchingBracketValidator,
    colored_prompt: String,
}

impl MyHelper {
    pub fn new() -> Self {
        MyHelper {
            completer: MyCompleter::new(),
            highlighter: MatchingBracketHighlighter::new(),
            colored_prompt: ">> ".to_owned(),
            validator: MatchingBracketValidator::new(),
        }
    }

    pub fn set_prompt(&mut self, prompt: &str) {
        self.colored_prompt = prompt.to_owned();
    }
}

impl Highlighter for MyHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        if default {
            Borrowed(&self.colored_prompt)
        } else {
            Borrowed(prompt)
        }
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
    }

    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_char(&self, line: &str, pos: usize, forced: bool) -> bool {
        self.highlighter.highlight_char(line, pos, forced)
    }
}

pub fn create_editor() -> Result<Editor<MyHelper, SQLiteHistory>> {
    let config =
        Config::builder()
            .completion_type(rustyline::CompletionType::List)
            .auto_add_history(true)
            .build();
    let history = rustyline::sqlite_history::SQLiteHistory::open(config, "tmp/history.sqlite3")?;
    let helper = MyHelper::new();
    let mut rl: Editor<MyHelper, _> = Editor::with_history(config, history)?;
    rl.set_helper(Some(helper));
    Ok(rl)
}
