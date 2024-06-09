use std::fmt::{self, Display, Formatter};

use anyhow::{bail, anyhow, Result};
use primitive_types::U256;
use solang_parser::pt::{Expression, Statement};

pub enum EvalResult {
    Num(U256),
}

impl Display for EvalResult {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            EvalResult::Num(n) => write!(f, "{}", n),
        }
    }
}

pub struct Interpreter {}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {}
    }

    pub fn evaluate_statement(&mut self, stmt: &Statement) -> Result<EvalResult> {
        match stmt {
            Statement::Expression(_, expr) => self.evaluate_expression(expr),
            _ => bail!("Statement not supported".to_string()),
        }
    }

    pub fn evaluate_expression(&mut self, expr: &Expression) -> Result<EvalResult> {
        match expr {
            Expression::NumberLiteral(_, n, _, _) => {
                let parsed_n = U256::from_dec_str(n).map_err(|e| anyhow!("{}", e.to_string()))?;
                Ok(EvalResult::Num(parsed_n))
            },
            Expression::Add(_, lhs, rhs) => {
                let lhs = self.evaluate_expression(lhs)?;
                let rhs = self.evaluate_expression(rhs)?;
                match (lhs, rhs) {
                    (EvalResult::Num(lhs), EvalResult::Num(rhs)) => Ok(EvalResult::Num(lhs + rhs)),
                }
            },
            _ => bail!("Expression not supported".to_string()),
        }
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}
