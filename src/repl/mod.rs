mod cli;
mod completer;
mod config;
mod helper;
#[allow(clippy::module_inception)]
mod repl;

pub use cli::{Cli, ECLAIR_VERSION};
pub use repl::Repl;
