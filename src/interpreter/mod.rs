mod parsing;

use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
};

use anyhow::{anyhow, bail, Result};
use primitive_types::U256;
use solang_parser::pt::{Expression, Statement};

#[derive(Debug, Clone)]
pub enum EvalResult {
    Empty,
    Num(U256),
    Str(String),
}

impl Display for EvalResult {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            EvalResult::Empty => write!(f, ""),
            EvalResult::Num(n) => write!(f, "{}", n),
            EvalResult::Str(s) => write!(f, "\"{}\"", s),
        }
    }
}

fn expr_as_var(expr: &Expression) -> Result<String> {
    if let Expression::Variable(id) = expr {
        Ok(id.to_string())
    } else {
        bail!("Invalid expression");
    }
}

pub struct Interpreter {
    env: HashMap<String, EvalResult>,
    debug: bool,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            env: HashMap::new(),
            debug: false,
        }
    }

    pub fn evaluate_line(&mut self, line: &str) -> Result<EvalResult> {
        if line.starts_with('!') {
            return self.evaluate_directive(line);
        }
        let stmt = parsing::parse_statement(line)?;
        if self.debug {
            println!("{:#?}", stmt);
        }
        self.evaluate_statement(&stmt)
    }

    pub fn evaluate_directive(&mut self, line: &str) -> Result<EvalResult> {
        match line {
            "!env" => {
                for (k, v) in &self.env {
                    println!("{}: {}", k, v);
                }
            }
            "!debug" => self.debug = !self.debug,
            _ => bail!("Directive not supported"),
        }

        Ok(EvalResult::Empty)
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
            }
            Expression::StringLiteral(strs) => Ok(EvalResult::Str(strs[0].string.clone())),

            Expression::Assign(_, var, expr) => {
                let id = expr_as_var(var)?;
                let result = self.evaluate_expression(expr)?;
                self.env.insert(id.to_string(), result.clone());
                Ok(result)
            }

            Expression::Variable(var) => {
                let id = var.to_string();
                if let Some(result) = self.env.get(&id) {
                    Ok(result.clone())
                } else {
                    bail!("{} is not defined", id);
                }
            }

            Expression::Add(_, lhs, rhs) => {
                let lhs = self.evaluate_expression(lhs)?;
                let rhs = self.evaluate_expression(rhs)?;
                match (lhs, rhs) {
                    (EvalResult::Num(lhs), EvalResult::Num(rhs)) => Ok(EvalResult::Num(lhs + rhs)),
                    _ => bail!("Invalid result not supported".to_string()),
                }
            }
            v => bail!("{} not supported", v),
        }
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}
