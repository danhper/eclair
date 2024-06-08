use std::borrow::Cow::{self, Borrowed, Owned};

use rustyline::completion::{FilenameCompleter, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::HistoryHinter;
use rustyline::validate::MatchingBracketValidator;
use rustyline::{Completer, Config, Context, Editor, Helper, Hinter, Result, Validator};

use solang_parser::pt::ContractDefinition;
use solang_parser::{
    parse,
    pt::{ContractPart, SourceUnitPart},
};

#[derive(Helper, Completer, Hinter, Validator)]
struct MyHelper {
    #[rustyline(Completer)]
    completer: MyCompleter,
    highlighter: MatchingBracketHighlighter,
    #[rustyline(Validator)]
    validator: MatchingBracketValidator,
    #[rustyline(Hinter)]
    hinter: HistoryHinter,
    colored_prompt: String,
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

struct MyCompleter {
    filename_completer: FilenameCompleter,
}

impl MyCompleter {
    fn new() -> Self {
        MyCompleter {
            filename_completer: FilenameCompleter::new(),
        }
    }
}

impl rustyline::completion::Completer for MyCompleter {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, _ctx: &Context<'_>) -> Result<(usize, Vec<Pair>)> {
        if line.starts_with("./") || line.starts_with("../") || line.starts_with('/') {
            return self.filename_completer.complete(line, pos, _ctx);
        }

        let completions = vec![
            Pair {
                display: "Hello".to_owned(),
                replacement: "Hello".to_owned(),
            },
            Pair {
                display: "World".to_owned(),
                replacement: "World".to_owned(),
            },
        ];
        let matches = completions
            .into_iter()
            .filter(|item| item.display.starts_with(&line[..pos]))
            .collect();
        Ok((0, matches))
    }
}

fn main() -> Result<()> {
    let config =
        Config::builder()
            .completion_type(rustyline::CompletionType::List)
            .auto_add_history(true)
            .build();
    let history = rustyline::sqlite_history::SQLiteHistory::open(config, "tmp/history.sqlite3")?;
    let h = MyHelper {
        completer: MyCompleter::new(),
        highlighter: MatchingBracketHighlighter::new(),
        hinter: HistoryHinter::new(),
        colored_prompt: ">> ".to_owned(),
        validator: MatchingBracketValidator::new(),
    };

    let mut rl: Editor<MyHelper, _> = Editor::with_history(config, history)?;
    rl.set_helper(Some(h));

    // loop {
    //     let p = ">> ";
    //     rl.helper_mut().expect("No helper").colored_prompt = format!("\x1b[1;32m{p}\x1b[0m");
    //     let readline = rl.readline(p);
    //     match readline {
    //         Ok(line) => {
    //             println!("Line: {}", line);
    //         }
    //         Err(ReadlineError::Interrupted) => break,
    //         Err(ReadlineError::Eof) => break,
    //         Err(err) => {
    //             println!("Error: {:?}", err);
    //             break;
    //         }
    //     }
    // }

    let mut statement = "BaseContract base = new BaseContract();".to_owned();
    if !statement.ends_with(';') {
        statement = format!("{};", statement);
    }
    let code = format!(r#"contract ReplContract {{

    function replFunction() external {{
        {}
    }}
}}
"#, statement);

    let (tree, comments) = parse(&code, 0).unwrap();
    let function_parts = match &tree.0[0] {
        SourceUnitPart::ContractDefinition(def) => {
            def.parts.clone()
        }
        _ => unreachable!(),
    };
    let parts = match &function_parts[0] {
        ContractPart::FunctionDefinition(def) => {
            def.body.clone()
        }
        _ => unreachable!(),
    };

    println!("{:#?}", parts);
    Ok(())
}
