mod cli;
mod completer;
mod config;
#[allow(clippy::module_inception)]
mod repl;
mod solidity_helper;

pub use cli::{Cli, ECLAIR_VERSION};
pub use repl::{initialize_env, Repl};
