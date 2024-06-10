mod eval_result;
#[allow(clippy::module_inception)]
mod interpreter;
mod parsing;
mod utils;

pub use eval_result::EvalResult;
pub use interpreter::Interpreter;
