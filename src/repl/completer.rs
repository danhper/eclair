use rustyline::{
    completion::{FilenameCompleter, Pair},
    Context,
};

pub(crate) struct MyCompleter {
    filename_completer: FilenameCompleter,
}

impl MyCompleter {
    pub fn new() -> Self {
        MyCompleter {
            filename_completer: FilenameCompleter::new(),
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
