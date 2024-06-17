use std::str::FromStr;
use std::{cell::RefCell, rc::Rc};

use anyhow::{anyhow, bail, Result};
use ethers::types::{Address, H160, U256};
use solang_parser::pt::{Expression, Statement};

use crate::project::types::Project;

use super::functions::CallType;
use super::{env::Env, parsing, utils::expr_as_var, value::Value};

pub struct Interpreter {
    env: Rc<RefCell<Env>>,
    debug: bool,
}

impl Interpreter {
    pub fn new(env: Rc<RefCell<Env>>) -> Self {
        Interpreter { env, debug: false }
    }

    pub fn load_project(&mut self, project: Box<dyn Project>) -> Result<()> {
        for contract_name in project.contract_names() {
            let contract = project.get_contract(&contract_name);
            self.env
                .borrow_mut()
                .set_type(&contract_name, contract.clone());
        }
        Ok(())
    }

    pub fn evaluate_line(&mut self, line: &str) -> Result<Option<Value>> {
        if line.starts_with('!') {
            return self.evaluate_directive(line);
        }
        let stmt = parsing::parse_statement(line)?;
        if self.debug {
            println!("{:#?}", stmt);
        }
        self.evaluate_statement(&stmt)
    }

    pub fn evaluate_directive(&mut self, line: &str) -> Result<Option<Value>> {
        match line {
            "!env" => {
                for k in self.env.borrow().list_vars() {
                    println!("{}: {}", k, self.env.borrow().get_var(&k).unwrap());
                }
            }
            "!debug" => self.debug = !self.debug,
            _ => bail!("Directive not supported"),
        }

        Ok(None)
    }

    fn create_call_type(&mut self, func: &Expression) -> Result<CallType> {
        match func {
            Expression::Variable(var) => {
                let id = var.to_string();
                if self.env.borrow().get_type(&id).is_some() {
                    Ok(CallType::ContractCast(id))
                } else if self.env.borrow().get_var(&id).is_some() {
                    Ok(CallType::RegularCall(id))
                } else {
                    bail!("{} is not defined", id);
                }
            }
            Expression::MemberAccess(_, expr, id) => {
                let receiver = expr_as_var(expr)?;
                if let Some(v) = self.env.borrow().get_var(&receiver) {
                    if matches!(v, Value::Contract(_, _, _)) {
                        Ok(CallType::ContractCall(receiver.to_string(), id.to_string()))
                    } else {
                        Ok(CallType::ModuleCall(receiver.to_string(), id.to_string()))
                    }
                } else {
                    bail!("{} is not defined", receiver);
                }
            }
            _ => bail!("{} not supported", func),
        }
    }

    pub fn evaluate_statement(&mut self, stmt: &Statement) -> Result<Option<Value>> {
        match stmt {
            Statement::Expression(_, expr) => self.evaluate_expression(expr).map(Some),
            Statement::VariableDefinition(_, var, expr) => {
                let id = var
                    .name
                    .clone()
                    .ok_or(anyhow!("invalid declaration {}", stmt))?
                    .name;
                if let Some(e) = expr {
                    let result = self.evaluate_expression(e)?;
                    self.env.borrow_mut().set_var(&id, result.clone());
                    Ok(None)
                } else {
                    bail!("declarations need rhs")
                }
            }
            _ => bail!("Statement not supported".to_string()),
        }
    }

    pub fn evaluate_expression(&mut self, expr: &Expression) -> Result<Value> {
        match expr {
            Expression::NumberLiteral(_, n, _, _) => {
                let parsed_n = U256::from_dec_str(n).map_err(|e| anyhow!("{}", e.to_string()))?;
                Ok(Value::Uint(parsed_n))
            }
            Expression::StringLiteral(strs) => Ok(Value::Str(strs[0].string.clone())),

            Expression::Assign(_, var, expr) => {
                let id = expr_as_var(var)?;
                let result = self.evaluate_expression(expr)?;
                self.env.borrow_mut().set_var(&id, result.clone());
                Ok(result)
            }

            Expression::Variable(var) => {
                let id = var.to_string();
                if let Some(result) = self.env.borrow().get_var(&id) {
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

            Expression::FunctionCall(_, name, args_) => {
                let call_type = self.create_call_type(name)?;
                let args = args_
                    .iter()
                    .map(|arg| self.evaluate_expression(arg))
                    .collect::<Result<Vec<Value>>>()?;
                call_type.execute(&mut self.env.borrow_mut(), &args)
            }

            Expression::HexNumberLiteral(_, n, _) => {
                let result = if n.len() == 42 {
                    let addr = H160::from_str(&n[2..])?;
                    Value::Addr(Address::try_from(addr)?)
                } else {
                    Value::Uint(U256::from_dec_str(n)?)
                };
                Ok(result)
            }

            Expression::Parenthesis(_, expr) => self.evaluate_expression(expr),

            v => bail!("{} not supported", v),
        }
    }

    fn eval_binop(&mut self, lexpr: &Expression, rexpr: &Expression, op: &str) -> Result<Value> {
        let lhs = self.evaluate_expression(lexpr)?;
        let rhs = self.evaluate_expression(rexpr)?;
        match (&lhs, &rhs) {
            (Value::Uint(l), Value::Uint(r)) => match op {
                "+" => Ok(Value::Uint(l + r)),
                "-" => Ok(Value::Uint(l - r)),
                "*" => Ok(Value::Uint(l * r)),
                "/" => Ok(Value::Uint(l / r)),
                "%" => Ok(Value::Uint(l % r)),
                _ => bail!("{} not supported", op),
            },
            _ => bail!("{} not supported for {} and {}", op, lhs, rhs),
        }
    }
}
