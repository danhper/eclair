use anyhow::{bail, Result};
use solang_parser::pt::Expression;

pub(crate) fn expr_as_var(expr: &Expression) -> Result<String> {
    if let Expression::Variable(id) = expr {
        Ok(id.to_string())
    } else {
        bail!("left hand side invalid or not supported");
    }
}
