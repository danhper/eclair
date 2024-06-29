use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};
use std::str::FromStr;

use alloy::hex::FromHex;
use alloy::primitives::{Address, B256, I256, U256};
use anyhow::{anyhow, bail, Result};
use futures::future::{BoxFuture, FutureExt};
use solang_parser::pt::{ContractPart, Expression, Statement};

use crate::project::types::Project;

use super::assignment::Lhs;
use super::builtin_functions::BuiltinFunction;
use super::functions::{Function, UserDefinedFunction};
use super::parsing::ParsedCode;
use super::types::Type;
use super::{env::Env, parsing, value::Value};

pub const SETUP_FUNCTION_NAME: &str = "setUp";

pub fn load_builtins(env: &mut Env) {
    env.set_var("_", Value::TypeObject(Type::This));
    env.set_var("repl", Value::TypeObject(Type::Repl));
    env.set_var("console", Value::TypeObject(Type::Console));

    for name in BuiltinFunction::functions() {
        env.set_var(
            &name,
            Value::Func(Function::Builtin(
                BuiltinFunction::from_name(&name).unwrap(),
            )),
        );
    }
}

pub fn load_project(env: &mut Env, project: &Project) -> Result<()> {
    for contract_name in project.contract_names().iter() {
        let contract = project.get_contract(contract_name);
        env.set_type(
            contract_name,
            Type::Contract(contract_name.clone(), contract.clone()),
        );
    }
    Ok(())
}

pub async fn evaluate_setup(env: &mut Env, code: &str) -> Result<()> {
    let def = parsing::parse_contract(code)?;
    evaluate_contract_parts(env, &def.parts).await?;
    let setup = env.get_var(SETUP_FUNCTION_NAME).cloned();
    if let Some(Value::Func(func @ Function::UserDefined(_))) = setup {
        func.execute_in_current_scope(&[], env).await?;
        env.delete_var(SETUP_FUNCTION_NAME)
    }

    Ok(())
}

pub async fn evaluate_code(env: &mut Env, code: &str) -> Result<Option<Value>> {
    let parsed = parsing::parse_input(code)?;

    match parsed {
        ParsedCode::Statements(stmts) => {
            if env.is_debug() {
                println!("{:#?}", stmts);
            }
            evaluate_statements(env, &stmts).await
        }
        ParsedCode::ContractDefinition(def) => {
            if env.is_debug() {
                println!("{:#?}", def);
            }
            evaluate_contract_parts(env, &def.parts).await?;
            Ok(None)
        }
    }
}

pub async fn evaluate_contract_parts(
    env: &mut Env,
    parts: &[solang_parser::pt::ContractPart],
) -> Result<()> {
    for part in parts.iter() {
        evaluate_contract_part(env, part).await?;
    }
    Ok(())
}

pub async fn evaluate_contract_part(
    env: &mut Env,
    part: &solang_parser::pt::ContractPart,
) -> Result<()> {
    match part {
        ContractPart::FunctionDefinition(def) => {
            let func = UserDefinedFunction::try_from(def.as_ref().clone())?;
            env.set_var(&func.name, Value::Func(Function::UserDefined(func.clone())));
        }
        ContractPart::VariableDefinition(def) => {
            let id = def.name.clone().ok_or(anyhow!("invalid declaration"))?.name;
            if let Some(expr) = &def.initializer {
                let result = evaluate_expression(env, Box::new(expr.clone())).await?;
                env.set_var(&id, result.clone());
            } else {
                bail!("declarations need rhs")
            }
        }
        v => bail!("{} not supported", v),
    }
    Ok(())
}

pub async fn evaluate_statements(env: &mut Env, stmts: &[Statement]) -> Result<Option<Value>> {
    let mut result = Ok(None);
    for stmt in stmts.iter() {
        result = evaluate_statement(env, Box::new(stmt.clone())).await;
        if let Statement::Return(_, _) = stmt {
            break;
        }
    }
    result
}

pub fn evaluate_statement(
    env: &mut Env,
    stmt: Box<Statement>,
) -> BoxFuture<'_, Result<Option<Value>>> {
    async move {
        match stmt.as_ref() {
            Statement::Expression(_, expr) => evaluate_expression(env, Box::new(expr.clone()))
                .await
                .map(Some),

            Statement::If(_, cond, then_stmt, else_stmt) => {
                let cond = evaluate_expression(env, Box::new(cond.clone())).await?;
                match cond {
                    Value::Bool(true) => evaluate_statement(env, then_stmt.clone()).await,
                    Value::Bool(false) => {
                        if let Some(else_stmt) = else_stmt {
                            evaluate_statement(env, else_stmt.clone()).await
                        } else {
                            Ok(None)
                        }
                    }
                    v => bail!("invalid type for if condition, expected bool, got {}", v),
                }
            }

            Statement::Return(_, expr) => {
                let result = if let Some(expr) = expr {
                    evaluate_expression(env, Box::new(expr.clone())).await?
                } else {
                    Value::Null
                };
                Ok(Some(result))
            }

            Statement::For(_, init, cond, update, body) => {
                if let Some(init) = init {
                    evaluate_statement(env, init.clone()).await?;
                }

                loop {
                    let cond = match cond {
                        Some(cond) => evaluate_expression(env, cond.clone()).await?,
                        None => Value::Bool(true),
                    };
                    match cond {
                        Value::Bool(true) => {
                            if let Some(body) = body {
                                evaluate_statement(env, body.clone()).await?;
                            }
                            if let Some(update) = update {
                                evaluate_expression(env, update.clone()).await?;
                            }
                        }
                        Value::Bool(false) => break,
                        v => bail!("invalid type for for condition, expected bool, got {}", v),
                    }
                }

                Ok(None)
            }

            Statement::Block { statements, .. } => evaluate_statements(env, statements).await,

            Statement::VariableDefinition(_, var, expr) => {
                let id = var
                    .name
                    .clone()
                    .ok_or(anyhow!("invalid declaration {}", stmt))?
                    .name;
                if let Some(e) = expr {
                    let result = evaluate_expression(env, Box::new(e.clone())).await?;
                    env.set_var(&id, result.clone());
                    Ok(None)
                } else {
                    bail!("declarations need rhs")
                }
            }
            stmt => bail!("statement {:?} not supported", stmt),
        }
    }
    .boxed()
}

pub fn evaluate_expression(env: &mut Env, expr: Box<Expression>) -> BoxFuture<'_, Result<Value>> {
    async move {
        match *expr {
            Expression::BoolLiteral(_, b) => Ok(Value::Bool(b)),
            Expression::NumberLiteral(_, n, decimals, _) => {
                let mut parsed_n = U256::from_str(&n).map_err(|e| anyhow!("{}", e.to_string()))?;
                if !decimals.is_empty() {
                    let parsed_decimals =
                        U256::from_str(&decimals).map_err(|e| anyhow!("{}", e.to_string()))?;
                    parsed_n *= U256::from(10).pow(parsed_decimals);
                }
                Ok(Value::Uint(parsed_n))
            }
            Expression::StringLiteral(strs) => Ok(Value::Str(strs[0].string.clone())),

            Expression::PreIncrement(_, expr) => {
                let current_value = evaluate_expression(env, expr.clone()).await?;
                let lhs = Lhs::try_from_expr(expr.as_ref().clone())?;
                let new_value = (current_value + 1.into())?;
                lhs.execute_assign(new_value.clone(), env)?;
                Ok(new_value)
            }
            Expression::PreDecrement(_, expr) => {
                let current_value = evaluate_expression(env, expr.clone()).await?;
                let lhs = Lhs::try_from_expr(expr.as_ref().clone())?;
                let new_value = (current_value - 1.into())?;
                lhs.execute_assign(new_value.clone(), env)?;
                Ok(new_value)
            }
            Expression::PostIncrement(_, expr) => {
                let current_value = evaluate_expression(env, expr.clone()).await?;
                let lhs = Lhs::try_from_expr(expr.as_ref().clone())?;
                lhs.execute_assign((current_value.clone() + 1.into())?, env)?;
                Ok(current_value)
            }
            Expression::PostDecrement(_, expr) => {
                let current_value = evaluate_expression(env, expr.clone()).await?;
                let lhs = Lhs::try_from_expr(expr.as_ref().clone())?;
                lhs.execute_assign((current_value.clone() - 1.into())?, env)?;
                Ok(current_value)
            }

            Expression::HexNumberLiteral(_, n, _) => {
                let result = if n.len() == 42 {
                    Value::Addr(Address::from_hex(n)?)
                } else if n.len() < 66 {
                    Value::FixBytes(B256::from_hex(&n)?, 32)
                } else {
                    Value::Bytes(Vec::from_hex(&n[2..])?)
                };
                Ok(result)
            }

            Expression::RationalNumberLiteral(_, whole, raw_fraction, raw_exponent, _) => {
                let mut n = if whole.is_empty() {
                    U256::from(0)
                } else {
                    U256::from_str(&whole).map_err(|e| anyhow!("{}", e.to_string()))?
                };
                let exponent = if raw_exponent.is_empty() {
                    U256::from(0)
                } else {
                    U256::from_str(&raw_exponent)?
                };
                n *= U256::from(10).pow(exponent);

                let fraction = if raw_fraction.is_empty() {
                    U256::from(0)
                } else {
                    U256::from_str(&raw_fraction)?
                };
                let decimals_count = if fraction.is_zero() {
                    U256::from(0)
                } else {
                    U256::from(fraction.log10() + 1)
                };
                if decimals_count > exponent {
                    bail!("fraction has more digits than decimals");
                }
                let adjusted_fraction = fraction * U256::from(10).pow(exponent - decimals_count);
                n += adjusted_fraction;

                Ok(Value::Uint(n))
            }

            Expression::And(_, lexpr, rexpr) => {
                let lhs = evaluate_expression(env, lexpr).await?;
                if let Value::Bool(false) = lhs {
                    return Ok(lhs);
                }
                let rhs = evaluate_expression(env, rexpr).await?;
                match (&lhs, &rhs) {
                    (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(*a && *b)),
                    _ => bail!("expected booleans for &&, got {} and {}", lhs, rhs),
                }
            }

            Expression::Or(_, lexpr, rexpr) => {
                let lhs = evaluate_expression(env, lexpr).await?;
                if let Value::Bool(true) = lhs {
                    return Ok(lhs);
                }
                let rhs = evaluate_expression(env, rexpr).await?;
                match (&lhs, &rhs) {
                    (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(*a || *b)),
                    _ => bail!("expected booleans for ||, got {} and {}", lhs, rhs),
                }
            }

            Expression::Not(_, expr) => match evaluate_expression(env, expr).await? {
                Value::Bool(b) => Ok(Value::Bool(!b)),
                v => bail!("invalid type for not, expected bool, got {}", v),
            },

            Expression::Equal(_, lhs, rhs) => {
                _eval_comparison(env, lhs, rhs, |o| o == Ordering::Equal).await
            }
            Expression::NotEqual(_, lhs, rhs) => {
                _eval_comparison(env, lhs, rhs, |o| o == Ordering::Equal).await
            }
            Expression::Less(_, lhs, rhs) => {
                _eval_comparison(env, lhs, rhs, |o| o == Ordering::Less).await
            }
            Expression::LessEqual(_, lhs, rhs) => {
                _eval_comparison(env, lhs, rhs, |o| {
                    o == Ordering::Less || o == Ordering::Equal
                })
                .await
            }
            Expression::More(_, lhs, rhs) => {
                _eval_comparison(env, lhs, rhs, |o| o == Ordering::Greater).await
            }
            Expression::MoreEqual(_, lhs, rhs) => {
                _eval_comparison(env, lhs, rhs, |o| {
                    o == Ordering::Greater || o == Ordering::Equal
                })
                .await
            }

            Expression::Negate(_, expr) => match evaluate_expression(env, expr).await? {
                Value::Int(n) => Ok(Value::Int(n.neg())),
                Value::Uint(n) => Ok(Value::Int(I256::from_raw(n).neg())),
                v => bail!("invalid type for negate, expected uint, got {}", v),
            },

            Expression::Assign(_, lhs_expr, expr) => {
                let lhs = Lhs::try_from_expr(lhs_expr.as_ref().clone())?;
                let result = evaluate_expression(env, expr).await?;
                lhs.execute_assign(result, env)?;
                Ok(Value::Null)
            }

            Expression::Variable(var) => {
                let id = var.to_string();
                if let Some(result) = env.get_var(&id) {
                    Ok(result.clone())
                } else if let Some(type_) = env.get_type(&id) {
                    Ok(Value::TypeObject(type_.clone()))
                } else {
                    bail!("{} is not defined", id);
                }
            }

            Expression::MemberAccess(_, receiver_expr, method) => {
                match evaluate_expression(env, receiver_expr).await? {
                    Value::Contract(c) => {
                        Ok(Value::Func(Function::ContractCall(c, method.name.clone())))
                    }
                    Value::NamedTuple(_, fields) => {
                        let field = fields
                            .get(&method.name)
                            .ok_or(anyhow!("field {} not found", method.name))?;
                        Ok(field.clone())
                    }
                    v => {
                        let method = BuiltinFunction::with_receiver(&v, &method.name)?;
                        if method.is_property() {
                            Ok(method.execute(&[], env).await?)
                        } else {
                            Ok(Value::Func(Function::Builtin(method)))
                        }
                    }
                }
            }

            Expression::ArrayLiteral(_, exprs) => {
                let mut values = vec![];
                for expr in exprs.iter() {
                    values.push(evaluate_expression(env, Box::new(expr.clone())).await?);
                }
                Ok(Value::Array(values))
            }

            Expression::ArraySubscript(_, expr, subscript_opt) => {
                let lhs = evaluate_expression(env, expr).await?;
                match lhs {
                    Value::Tuple(values) | Value::Array(values) => {
                        let subscript = subscript_opt
                            .ok_or(anyhow!("tuples and arrays do not support empty subscript"))?;
                        let index = evaluate_expression(env, subscript).await?.as_usize()?;
                        if index >= values.len() {
                            bail!("index out of bounds");
                        }
                        Ok(values[index].clone())
                    }
                    v => bail!("invalid type for subscript, expected tuple, got {}", v),
                }
            }

            Expression::ArraySlice(_, arr_expr, start_expr, end_expr) => {
                let values = match evaluate_expression(env, arr_expr).await? {
                    Value::Array(v) => v,
                    v => bail!("invalid type for slice, expected tuple, got {}", v),
                };
                let start = match start_expr {
                    Some(expr) => evaluate_expression(env, expr).await?.as_usize()?,
                    None => 0,
                };
                let end = match end_expr {
                    Some(expr) => evaluate_expression(env, expr).await?.as_usize()?,
                    None => values.len(),
                };
                if end > values.len() {
                    bail!("end index out of bounds");
                }
                Ok(Value::Array(values[start..end].to_vec()))
            }

            Expression::Add(_, lhs, rhs) => _eval_binop(env, lhs, rhs, Value::add).await,
            Expression::Subtract(_, lhs, rhs) => _eval_binop(env, lhs, rhs, Value::sub).await,
            Expression::Multiply(_, lhs, rhs) => _eval_binop(env, lhs, rhs, Value::mul).await,
            Expression::Divide(_, lhs, rhs) => _eval_binop(env, lhs, rhs, Value::div).await,
            Expression::Modulo(_, lhs, rhs) => _eval_binop(env, lhs, rhs, Value::rem).await,
            Expression::Power(_, lhs, rhs) => {
                let left = evaluate_expression(env, lhs).await?;
                let right = evaluate_expression(env, rhs).await?;
                match (&left, &right) {
                    (Value::Uint(l), Value::Uint(r)) => Ok(Value::Uint(l.pow(*r))),
                    (Value::Int(l), Value::Uint(r)) => Ok(Value::Int(l.pow(*r))),
                    (Value::Uint(l), Value::Int(r)) => {
                        if r.is_negative() {
                            bail!("exponentiation with negative exponent")
                        }
                        Ok(Value::Uint(l.pow(r.unchecked_into())))
                    }
                    (Value::Int(l), Value::Int(r)) => {
                        if r.is_negative() {
                            bail!("exponentiation with negative exponent")
                        }
                        Ok(Value::Int(l.pow(r.unchecked_into())))
                    }
                    _ => bail!("{} not supported for {} and {}", "^", left, right),
                }
            }

            Expression::NamedFunctionCall(_, name_expr, args) => {
                let id = if let Expression::Variable(id) = name_expr.as_ref() {
                    id.to_string()
                } else {
                    bail!("expected variable, found {:?}", name_expr);
                };
                let mut fields = BTreeMap::new();
                for arg in args.iter() {
                    let value = evaluate_expression(env, Box::new(arg.expr.clone())).await?;
                    fields.insert(arg.name.name.clone(), value);
                }
                Ok(Value::NamedTuple(id, fields))
            }

            Expression::FunctionCall(_, func_expr, args_) => {
                let mut args = vec![];
                for arg in args_.iter() {
                    args.push(evaluate_expression(env, Box::new(arg.clone())).await?);
                }
                match evaluate_expression(env, func_expr).await? {
                    Value::Func(f) => f.execute(&args, env).await,
                    Value::TypeObject(type_) => {
                        if let [arg] = &args[..] {
                            type_.cast(arg)
                        } else {
                            bail!("cast requires a single argument")
                        }
                    }
                    v => bail!("invalid type, expected function, got {}", v),
                }
            }

            Expression::Type(_, type_) => Ok(Value::TypeObject(Type::try_from(type_)?)),
            Expression::Parenthesis(_, expr) => evaluate_expression(env, expr).await,

            v => bail!("{} not supported", v),
        }
    }
    .boxed()
}

async fn _eval_comparison(
    env: &mut Env,
    lexpr: Box<Expression>,
    rexpr: Box<Expression>,
    op: fn(Ordering) -> bool,
) -> Result<Value> {
    let lhs = evaluate_expression(env, lexpr).await?;
    let rhs = evaluate_expression(env, rexpr).await?;
    match lhs.partial_cmp(&rhs) {
        Some(ordering) => Ok(Value::Bool(op(ordering))),
        None => bail!("cannot compare {} and {}", lhs, rhs),
    }
}

async fn _eval_binop<F>(
    env: &mut Env,
    lexpr: Box<Expression>,
    rexpr: Box<Expression>,
    f: F,
) -> Result<Value>
where
    F: FnOnce(Value, Value) -> Result<Value>,
{
    let lhs = evaluate_expression(env, lexpr).await?;
    let rhs = evaluate_expression(env, rexpr).await?;
    f(lhs, rhs)
}
