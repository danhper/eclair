use std::{collections::HashMap, str::FromStr};

use anyhow::{anyhow, bail, Result};
use ethers::types::{Address, H160, U256};
use solang_parser::pt::{Expression, Statement};

use super::{eval_result::EvalResult, parsing, utils::expr_as_var};

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
                Ok(EvalResult::Uint(parsed_n))
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

            Expression::Add(_, lhs, rhs) => self.eval_binop(lhs, rhs, "+"),
            Expression::Subtract(_, lhs, rhs) => self.eval_binop(lhs, rhs, "-"),
            Expression::Multiply(_, lhs, rhs) => self.eval_binop(lhs, rhs, "*"),
            Expression::Divide(_, lhs, rhs) => self.eval_binop(lhs, rhs, "/"),
            Expression::Modulo(_, lhs, rhs) => self.eval_binop(lhs, rhs, "%"),

            Expression::HexNumberLiteral(_, n, _) => {
                let result = if n.len() == 42 {
                    let addr = H160::from_str(&n[2..])?;
                    EvalResult::Addr(Address::try_from(addr)?)
                } else {
                    EvalResult::Uint(U256::from_dec_str(n)?)
                };
                Ok(result)
            }

            Expression::Parenthesis(_, expr) => self.evaluate_expression(expr),

            v => bail!("{} not supported", v),
        }
    }

    fn eval_binop(
        &mut self,
        lexpr: &Expression,
        rexpr: &Expression,
        op: &str,
    ) -> Result<EvalResult> {
        let lhs = self.evaluate_expression(lexpr)?;
        let rhs = self.evaluate_expression(rexpr)?;
        match (&lhs, &rhs) {
            (EvalResult::Uint(l), EvalResult::Uint(r)) => match op {
                "+" => Ok(EvalResult::Uint(l + r)),
                "-" => Ok(EvalResult::Uint(l - r)),
                "*" => Ok(EvalResult::Uint(l * r)),
                "/" => Ok(EvalResult::Uint(l / r)),
                "%" => Ok(EvalResult::Uint(l % r)),
                _ => bail!("{} not supported", op),
            },
            _ => bail!("{} not supported for {} and {}", op, lhs, rhs),
        }
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}
