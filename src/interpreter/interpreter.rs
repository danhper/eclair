use std::cmp::Ordering;
use std::ops::{Add, BitAnd, BitOr, BitXor, Div, Mul, Neg, Rem, Shl, Shr, Sub};
use std::str::FromStr;

use alloy::hex::FromHex;
use alloy::primitives::{I256, U256};
use anyhow::{anyhow, bail, Ok, Result};
use futures::future::{BoxFuture, FutureExt};
use indexmap::IndexMap;
use solang_parser::pt::{ContractPart, Expression, Statement};

use crate::loaders::types::Project;

use super::assignment::Lhs;
use super::builtins;
use super::functions::{AnonymousFunction, FunctionDef, UserDefinedFunction};
use super::parsing::ParsedCode;
use super::types::{HashableIndexMap, Type};
use super::utils::parse_rational_literal;
use super::{env::Env, parsing, value::Value};

pub const SETUP_FUNCTION_NAME: &str = "setUp";

#[derive(Debug, PartialEq, Clone)]
pub enum StatementResult {
    Empty,
    Value(Value),
    Continue,
    Break,
    Return(Value),
}

impl std::fmt::Display for StatementResult {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            StatementResult::Empty => write!(f, "Empty"),
            StatementResult::Value(v) => write!(f, "Value({})", v),
            StatementResult::Continue => write!(f, "Continue"),
            StatementResult::Break => write!(f, "Break"),
            StatementResult::Return(v) => write!(f, "Return({})", v),
        }
    }
}

impl StatementResult {
    pub fn value(&self) -> Option<&Value> {
        match self {
            StatementResult::Value(v) | StatementResult::Return(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_value(&self) -> Result<&Value> {
        self.value().ok_or(anyhow!("expected value, got {}", self))
    }
}

unsafe impl std::marker::Send for StatementResult {}

pub fn load_builtins(env: &mut Env) {
    builtins::VALUES.iter().for_each(|(name, value)| {
        env.set_var(name, value.clone());
    });
}

pub fn load_project(env: &mut Env, project: &Project) -> Result<()> {
    for contract_name in project.contract_names().iter() {
        let contract = project.get_contract(contract_name);
        env.add_contract(contract_name, contract.clone());
    }
    Ok(())
}

pub async fn evaluate_setup(env: &mut Env, code: &str) -> Result<()> {
    let def = parsing::parse_contract(code)?;
    evaluate_contract_parts(env, &def.parts).await?;
    let setup = env.get_var(SETUP_FUNCTION_NAME).cloned();
    if let Some(Value::Func(func)) = setup {
        func.execute_in_current_scope(env, &[]).await?;
        env.delete_var(SETUP_FUNCTION_NAME)
    }

    Ok(())
}

pub async fn evaluate_code(env: &mut Env, code: &str) -> Result<Option<Value>> {
    let parsed = parsing::parse_input(code)?;

    let result = match parsed {
        ParsedCode::Statements(stmts) => {
            if env.is_debug() {
                println!("{:#?}", stmts);
            }
            evaluate_statements(env, &stmts)
                .await
                .map(|v| v.value().cloned())
        }
        ParsedCode::ContractDefinition(def) => {
            if env.is_debug() {
                println!("{:#?}", def);
            }
            evaluate_contract_parts(env, &def.parts).await?;
            Ok(None)
        }
    }?;

    if let Some(value) = &result {
        env.set_var("_", value.clone());
    }

    Ok(result)
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
            let name = func.name().to_string();
            env.set_var(&name, func.into());
        }
        ContractPart::VariableDefinition(def) => {
            env.init_variable(&def.name, &def.ty, &def.initializer)
                .await?;
        }
        v => bail!("{} not supported", v),
    }
    Ok(())
}

pub async fn evaluate_statements(env: &mut Env, stmts: &[Statement]) -> Result<StatementResult> {
    let mut result = StatementResult::Empty;
    for stmt in stmts.iter() {
        match evaluate_statement(env, Box::new(stmt.clone())).await? {
            r @ StatementResult::Return(_) => return Ok(r),
            r @ StatementResult::Continue => return Ok(r),
            r @ StatementResult::Break => return Ok(r),
            r => result = r,
        }
    }
    Ok(result)
}

pub fn evaluate_statement(
    env: &mut Env,
    stmt: Box<Statement>,
) -> BoxFuture<'_, Result<StatementResult>> {
    async move {
        match stmt.as_ref() {
            Statement::Expression(_, expr) => evaluate_expression(env, Box::new(expr.clone()))
                .await
                .map(StatementResult::Value),

            Statement::If(_, cond, then_stmt, else_stmt) => {
                let cond = evaluate_expression(env, Box::new(cond.clone())).await?;
                match cond {
                    Value::Bool(true) => evaluate_statement(env, then_stmt.clone()).await,
                    Value::Bool(false) => {
                        if let Some(else_stmt) = else_stmt {
                            evaluate_statement(env, else_stmt.clone()).await
                        } else {
                            Ok(StatementResult::Empty)
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
                Ok(StatementResult::Return(result))
            }

            Statement::Continue(_) => Ok(StatementResult::Continue),

            Statement::Break(_) => Ok(StatementResult::Break),

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
                                match evaluate_statement(env, body.clone()).await? {
                                    StatementResult::Break => break,
                                    r @ StatementResult::Return(_) => return Ok(r),
                                    _ => (),
                                }
                            }
                            if let Some(update) = update {
                                evaluate_expression(env, update.clone()).await?;
                            }
                        }
                        Value::Bool(false) => break,
                        v => bail!("invalid type for for condition, expected bool, got {}", v),
                    }
                }

                Ok(StatementResult::Empty)
            }

            Statement::Block { statements, .. } => evaluate_statements(env, statements).await,

            Statement::Args(_, args) => {
                let mut result = vec![];
                for arg in args.iter() {
                    let evaled = evaluate_expression(env, Box::new(arg.expr.clone())).await?;
                    result.push((arg.name.to_string(), evaled));
                }
                let values = IndexMap::from_iter(result);
                let named_tuple = Value::NamedTuple("Args".to_string(), HashableIndexMap(values));
                Ok(StatementResult::Value(named_tuple))
            }

            Statement::VariableDefinition(_, var, expr) => {
                env.init_variable(&var.name, &var.ty, expr).await?;
                Ok(StatementResult::Empty)
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
                Ok(Value::Uint(parsed_n, 256))
            }
            Expression::StringLiteral(strs) => Ok(Value::Str(strs[0].string.clone())),

            Expression::PreIncrement(_, expr) => {
                let current_value = evaluate_expression(env, expr.clone()).await?;
                let lhs = Lhs::try_from_expr(expr.as_ref().clone(), env).await?;
                let new_value = (current_value + 1u64.into())?;
                lhs.execute_assign(new_value.clone(), env)?;
                Ok(new_value)
            }
            Expression::PreDecrement(_, expr) => {
                let current_value = evaluate_expression(env, expr.clone()).await?;
                let lhs = Lhs::try_from_expr(expr.as_ref().clone(), env).await?;
                let new_value = (current_value - 1u64.into())?;
                lhs.execute_assign(new_value.clone(), env)?;
                Ok(new_value)
            }
            Expression::PostIncrement(_, expr) => {
                let current_value = evaluate_expression(env, expr.clone()).await?;
                let lhs = Lhs::try_from_expr(expr.as_ref().clone(), env).await?;
                lhs.execute_assign((current_value.clone() + 1u64.into())?, env)?;
                Ok(current_value)
            }
            Expression::PostDecrement(_, expr) => {
                let current_value = evaluate_expression(env, expr.clone()).await?;
                let lhs = Lhs::try_from_expr(expr.as_ref().clone(), env).await?;
                lhs.execute_assign((current_value.clone() - 1u64.into())?, env)?;
                Ok(current_value)
            }

            Expression::AssignAdd(_, left, right) => {
                _eval_binop_assign(env, left, right, |a, b| a + b).await
            }
            Expression::AssignSubtract(_, left, right) => {
                _eval_binop_assign(env, left, right, |a, b| a - b).await
            }
            Expression::AssignMultiply(_, left, right) => {
                _eval_binop_assign(env, left, right, |a, b| a * b).await
            }
            Expression::AssignDivide(_, left, right) => {
                _eval_binop_assign(env, left, right, |a, b| a / b).await
            }
            Expression::AssignModulo(_, left, right) => {
                _eval_binop_assign(env, left, right, |a, b| a % b).await
            }
            Expression::AssignAnd(_, left, right) => {
                _eval_binop_assign(env, left, right, |a, b| a & b).await
            }
            Expression::AssignOr(_, left, right) => {
                _eval_binop_assign(env, left, right, |a, b| a | b).await
            }
            Expression::AssignXor(_, left, right) => {
                _eval_binop_assign(env, left, right, |a, b| a ^ b).await
            }
            Expression::AssignShiftLeft(_, left, right) => {
                _eval_binop_assign(env, left, right, |a, b| a << b).await
            }
            Expression::AssignShiftRight(_, left, right) => {
                _eval_binop_assign(env, left, right, |a, b| a >> b).await
            }

            Expression::HexNumberLiteral(_, n, _) => Value::from_hex(n),

            Expression::RationalNumberLiteral(_, whole, raw_fraction, raw_exponent, _) => {
                parse_rational_literal(&whole, &raw_fraction, &raw_exponent)
                    .map(|v| Value::Uint(v, 256))
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
                let eq = _equals(env, lhs.clone(), rhs.clone()).await?;
                Ok(Value::Bool(eq))
            }
            Expression::NotEqual(_, lhs, rhs) => {
                let eq = _equals(env, lhs.clone(), rhs.clone()).await?;
                Ok(Value::Bool(!eq))
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
                Value::Int(n, s) => Ok(Value::Int(n.neg(), s)),
                Value::Uint(n, s) => Ok(Value::Int(I256::from_raw(n).neg(), s)),
                v => bail!("invalid type for negate, expected uint, got {}", v),
            },

            Expression::Assign(_, lhs_expr, expr) => {
                let lhs = Lhs::try_from_expr(lhs_expr.as_ref().clone(), env).await?;
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
                let receiver = evaluate_expression(env, receiver_expr).await?;
                match receiver.member_access(&method.name) {
                    Result::Ok(Value::Func(f)) if f.is_property() => f.execute(env, &[]).await,
                    v => v,
                }
            }

            Expression::List(_, items) => {
                let mut values = vec![];
                for (_, item) in items.iter() {
                    match item {
                        Some(item) => {
                            values.push(evaluate_expression(env, Box::new(item.ty.clone())).await?);
                        }
                        None => values.push(Value::Null),
                    }
                }
                Ok(Value::Tuple(values))
            }

            Expression::ArrayLiteral(_, exprs) => {
                let mut values = vec![];
                for expr in exprs.iter() {
                    values.push(evaluate_expression(env, Box::new(expr.clone())).await?);
                }
                let type_ = values
                    .first()
                    .map(|v| v.get_type().clone())
                    .unwrap_or(Type::Any);
                Ok(Value::Array(values, Box::new(type_)))
            }

            Expression::ArraySubscript(_, expr, subscript_opt) => {
                let lhs = evaluate_expression(env, expr).await?;
                match lhs {
                    Value::Tuple(values) | Value::Array(values, _) => {
                        let subscript = subscript_opt
                            .ok_or(anyhow!("tuples and arrays do not support empty subscript"))?;
                        let index = evaluate_expression(env, subscript).await?.as_usize()?;
                        if index >= values.len() {
                            bail!("index out of bounds");
                        }
                        Ok(values[index].clone())
                    }
                    Value::Mapping(values, kt, _) => {
                        let subscript = subscript_opt
                            .ok_or(anyhow!("mappings do not support empty subscript"))?;
                        let key = evaluate_expression(env, subscript).await?;
                        let key = kt.cast(&key)?;
                        Ok(values.0.get(&key).cloned().unwrap_or(Value::Null))
                    }
                    Value::TypeObject(type_) => match subscript_opt {
                        Some(subscript) => {
                            let value = evaluate_expression(env, subscript).await?;
                            Ok(Value::TypeObject(Type::FixedArray(
                                Box::new(type_),
                                value.as_usize()?,
                            )))
                        }
                        None => Ok(Value::TypeObject(Type::Array(Box::new(type_)))),
                    },
                    v => bail!("invalid type for subscript, expected tuple, got {}", v),
                }
            }

            Expression::ArraySlice(_, arr_expr, start_expr, end_expr) => {
                let value = evaluate_expression(env, arr_expr).await?;
                let start = match start_expr {
                    Some(expr) => Some(evaluate_expression(env, expr).await?.as_usize()?),
                    None => None,
                };
                let end = match end_expr {
                    Some(expr) => Some(evaluate_expression(env, expr).await?.as_usize()?),
                    None => None,
                };
                value.slice(start, end)
            }

            Expression::Add(_, lhs, rhs) => _eval_binop(env, lhs, rhs, Value::add).await,
            Expression::Subtract(_, lhs, rhs) => _eval_binop(env, lhs, rhs, Value::sub).await,
            Expression::Multiply(_, lhs, rhs) => _eval_binop(env, lhs, rhs, Value::mul).await,
            Expression::Divide(_, lhs, rhs) => _eval_binop(env, lhs, rhs, Value::div).await,
            Expression::Modulo(_, lhs, rhs) => _eval_binop(env, lhs, rhs, Value::rem).await,
            Expression::BitwiseAnd(_, lhs, rhs) => _eval_binop(env, lhs, rhs, Value::bitand).await,
            Expression::BitwiseOr(_, lhs, rhs) => _eval_binop(env, lhs, rhs, Value::bitor).await,
            Expression::BitwiseXor(_, lhs, rhs) => _eval_binop(env, lhs, rhs, Value::bitxor).await,
            Expression::ShiftLeft(_, lhs, rhs) => _eval_binop(env, lhs, rhs, Value::shl).await,

            // We overload shift right to also create anonymous functions
            Expression::ShiftRight(_, lhs, rhs) => {
                match AnonymousFunction::parse_lhs(lhs.as_ref()) {
                    Result::Ok(params) => {
                        let body = rhs.as_ref();
                        let func = AnonymousFunction::new(params, body.clone());
                        Ok(func.into())
                    }
                    Result::Err(_) => _eval_binop(env, lhs, rhs, Value::shr).await,
                }
            }

            Expression::Power(_, lhs, rhs) => {
                let left = evaluate_expression(env, lhs).await?;
                let right = evaluate_expression(env, rhs).await?;
                match (&left, &right) {
                    (Value::Uint(l, s1), Value::Uint(r, s2)) => {
                        Ok(Value::Uint(l.pow(*r), *s1.max(s2)))
                    }
                    (Value::Int(l, s1), Value::Uint(r, s2)) => {
                        Ok(Value::Int(l.pow(*r), *s1.max(s2)))
                    }
                    (Value::Uint(l, s1), Value::Int(r, s2)) => {
                        if r.is_negative() {
                            bail!("exponentiation with negative exponent")
                        }
                        Ok(Value::Uint(l.pow(r.unchecked_into()), *s1.max(s2)))
                    }
                    (Value::Int(l, s1), Value::Int(r, s2)) => {
                        if r.is_negative() {
                            bail!("exponentiation with negative exponent")
                        }
                        Ok(Value::Int(l.pow(r.unchecked_into()), *s1.max(s2)))
                    }
                    _ => bail!("{} not supported for {} and {}", "^", left, right),
                }
                .and_then(Value::validate_int)
            }

            Expression::NamedFunctionCall(_, name_expr, args) => {
                let id = if let Expression::Variable(id) = name_expr.as_ref() {
                    id.to_string()
                } else {
                    bail!("expected variable, found {:?}", name_expr);
                };
                let mut fields = IndexMap::new();
                for arg in args.iter() {
                    let value = evaluate_expression(env, Box::new(arg.expr.clone())).await?;
                    fields.insert(arg.name.name.clone(), value);
                }
                Ok(Value::NamedTuple(id, HashableIndexMap(fields)))
            }

            Expression::FunctionCall(_, func_expr, args_) => {
                let mut args = vec![];
                for arg in args_.iter() {
                    args.push(evaluate_expression(env, Box::new(arg.clone())).await?);
                }
                match evaluate_expression(env, func_expr).await? {
                    Value::Func(f) => f.execute(env, &args).await,
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

            Expression::FunctionCallBlock(_, func_expr, stmt) => {
                let res = evaluate_statement(env, stmt).await?;
                match evaluate_expression(env, func_expr).await? {
                    Value::Func(f) => {
                        let opts = res.as_value()?.as_record()?.clone();
                        Ok(f.with_opts(opts).into())
                    }
                    _ => bail!("expected function"),
                }
            }

            Expression::Type(_, type_) => Ok(Value::TypeObject(Type::try_from(type_)?)),
            Expression::Parenthesis(_, expr) => evaluate_expression(env, expr).await,

            v => bail!("{} not supported", v),
        }
    }
    .boxed()
}

async fn _equals(env: &mut Env, lexpr: Box<Expression>, rexpr: Box<Expression>) -> Result<bool> {
    let lhs = evaluate_expression(env, lexpr).await?;
    let rhs = evaluate_expression(env, rexpr).await?;
    Ok(lhs == rhs)
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

async fn _eval_binop_assign<F>(
    env: &mut Env,
    lexpr: Box<Expression>,
    rexpr: Box<Expression>,
    f: F,
) -> Result<Value>
where
    F: FnOnce(Value, Value) -> Result<Value>,
{
    let lhs = Lhs::try_from_expr(lexpr.as_ref().clone(), env).await?;
    let new_value = _eval_binop(env, lexpr, rexpr, f).await?;
    lhs.execute_assign(new_value.clone(), env)?;
    Ok(new_value)
}
