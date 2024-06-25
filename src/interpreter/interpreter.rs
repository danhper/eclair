use std::cmp::Ordering;
use std::ops::Neg;
use std::process::Command;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;

use alloy::hex::FromHex;
use alloy::primitives::{Address, B256, I256, U256};
use alloy::providers::{Provider, ProviderBuilder, RootProvider};
use alloy::transports::http::{Client, Http};
use anyhow::{anyhow, bail, Result};
use futures::future::{BoxFuture, FutureExt};
use solang_parser::pt::{Expression, Statement};

use crate::project::types::Project;

use super::builtin_functions::BuiltinFunction;
use super::functions::Function;
use super::types::Type;
use super::{directive::Directive, env::Env, parsing, utils::expr_as_var, value::Value};

#[derive(Debug)]
pub struct Interpreter {
    env: Arc<Mutex<Env>>,
    debug: bool,
    provider: Arc<RootProvider<Http<Client>>>,
}

unsafe impl std::marker::Send for Interpreter {}
unsafe impl Sync for Interpreter {}

impl Interpreter {
    pub fn new(env: Arc<Mutex<Env>>, provider_url: &str, debug: bool) -> Self {
        let rpc_url = provider_url.parse().unwrap();
        let provider = ProviderBuilder::new().on_http(rpc_url);

        Interpreter {
            env,
            debug,
            provider: Arc::new(provider),
        }
    }

    pub async fn load_project(&mut self, project: Box<dyn Project>) -> Result<()> {
        let mut env = self.env.lock().await;
        for contract_name in project.contract_names().iter() {
            let contract = project.get_contract(contract_name);
            env.set_type(
                contract_name,
                Type::Contract(contract_name.clone(), contract.clone()),
            );
        }
        Ok(())
    }

    pub async fn list_vars(&self) {
        let env = self.env.lock().await;
        for k in env.list_vars().iter() {
            println!("{}: {}", k, env.get_var(k).unwrap());
        }
    }

    fn set_provider(&mut self, url: &str) {
        let rpc_url = url.parse().unwrap();
        let provider = ProviderBuilder::new().on_http(rpc_url);
        self.provider = Arc::new(provider);
    }

    pub async fn evaluate_line(&mut self, line: &str) -> Result<Option<Value>> {
        if let Some(directive_str) = line.strip_prefix('!') {
            if let Ok(directive) = Directive::parse(directive_str) {
                return self._evaluate_directive(directive).await;
            }
        }
        let stmt = parsing::parse_statement(line)?;
        if self.debug {
            println!("{:#?}", stmt);
        }
        self.evaluate_statement(&stmt).await
    }

    async fn _evaluate_directive(&mut self, directive: Directive) -> Result<Option<Value>> {
        match directive {
            Directive::Env => self.list_vars().await,
            Directive::SetRpc(rpc_url) => self.set_provider(&rpc_url),
            Directive::ShowRpc => {
                println!("{}", self.provider.root().client().transport().url())
            }
            Directive::Debug => self.debug = !self.debug,
            Directive::Exec(cmd, args) => {
                Command::new(cmd).args(args).spawn()?;
            }
        }

        Ok(None)
    }

    pub async fn evaluate_statement(&mut self, stmt: &Statement) -> Result<Option<Value>> {
        match stmt {
            Statement::Expression(_, expr) => self
                .evaluate_expression(Box::new(expr.clone()))
                .await
                .map(Some),
            Statement::VariableDefinition(_, var, expr) => {
                let id = var
                    .name
                    .clone()
                    .ok_or(anyhow!("invalid declaration {}", stmt))?
                    .name;
                if let Some(e) = expr {
                    let result = self.evaluate_expression(Box::new(e.clone())).await?;
                    let mut env = self.env.lock().await;
                    env.set_var(&id, result.clone());
                    Ok(None)
                } else {
                    bail!("declarations need rhs")
                }
            }
            _ => bail!("Statement not supported".to_string()),
        }
    }

    pub fn evaluate_expression(&mut self, expr: Box<Expression>) -> BoxFuture<'_, Result<Value>> {
        async move {
            match *expr {
                Expression::BoolLiteral(_, b) => Ok(Value::Bool(b)),
                Expression::NumberLiteral(_, n, decimals, _) => {
                    let mut parsed_n =
                        U256::from_str(&n).map_err(|e| anyhow!("{}", e.to_string()))?;
                    if !decimals.is_empty() {
                        let parsed_decimals =
                            U256::from_str(&decimals).map_err(|e| anyhow!("{}", e.to_string()))?;
                        parsed_n *= U256::from(10).pow(parsed_decimals);
                    }
                    Ok(Value::Uint(parsed_n))
                }
                Expression::StringLiteral(strs) => Ok(Value::Str(strs[0].string.clone())),

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
                    let adjusted_fraction =
                        fraction * U256::from(10).pow(exponent - decimals_count);
                    n += adjusted_fraction;

                    Ok(Value::Uint(n))
                }

                Expression::And(_, lexpr, rexpr) => {
                    let lhs = self.evaluate_expression(lexpr).await?;
                    if let Value::Bool(false) = lhs {
                        return Ok(lhs);
                    }
                    let rhs = self.evaluate_expression(rexpr).await?;
                    match (&lhs, &rhs) {
                        (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(*a && *b)),
                        _ => bail!("expected booleans for &&, got {} and {}", lhs, rhs),
                    }
                }

                Expression::Or(_, lexpr, rexpr) => {
                    let lhs = self.evaluate_expression(lexpr).await?;
                    if let Value::Bool(true) = lhs {
                        return Ok(lhs);
                    }
                    let rhs = self.evaluate_expression(rexpr).await?;
                    match (&lhs, &rhs) {
                        (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(*a || *b)),
                        _ => bail!("expected booleans for ||, got {} and {}", lhs, rhs),
                    }
                }

                Expression::Not(_, expr) => match self.evaluate_expression(expr).await? {
                    Value::Bool(b) => Ok(Value::Bool(!b)),
                    v => bail!("invalid type for not, expected bool, got {}", v),
                },

                Expression::Equal(_, lhs, rhs) => {
                    self._eval_comparison(lhs, rhs, |o| o == Ordering::Equal)
                        .await
                }
                Expression::NotEqual(_, lhs, rhs) => {
                    self._eval_comparison(lhs, rhs, |o| o == Ordering::Equal)
                        .await
                }
                Expression::Less(_, lhs, rhs) => {
                    self._eval_comparison(lhs, rhs, |o| o == Ordering::Less)
                        .await
                }
                Expression::LessEqual(_, lhs, rhs) => {
                    self._eval_comparison(lhs, rhs, |o| o == Ordering::Less || o == Ordering::Equal)
                        .await
                }
                Expression::More(_, lhs, rhs) => {
                    self._eval_comparison(lhs, rhs, |o| o == Ordering::Greater)
                        .await
                }
                Expression::MoreEqual(_, lhs, rhs) => {
                    self._eval_comparison(lhs, rhs, |o| {
                        o == Ordering::Greater || o == Ordering::Equal
                    })
                    .await
                }

                Expression::Negate(_, expr) => match self.evaluate_expression(expr).await? {
                    Value::Int(n) => Ok(Value::Int(n.neg())),
                    Value::Uint(n) => Ok(Value::Int(I256::from_raw(n).neg())),
                    v => bail!("invalid type for negate, expected uint, got {}", v),
                },

                Expression::Assign(_, var, expr) => {
                    let id = expr_as_var(&var)?;
                    let result = self.evaluate_expression(expr).await?;
                    let mut env = self.env.lock().await;
                    env.set_var(&id, result.clone());
                    Ok(result)
                }

                Expression::Variable(var) => {
                    let id = var.to_string();
                    let env = self.env.lock().await;
                    if let Some(result) = env.get_var(&id) {
                        Ok(result.clone())
                    } else if let Some(type_) = env.get_type(&id) {
                        Ok(Value::TypeObject(type_.clone()))
                    } else if let Ok(func) = BuiltinFunction::from_name(&id) {
                        Ok(Value::Func(Function::Builtin(func)))
                    } else {
                        bail!("{} is not defined", id);
                    }
                }

                Expression::MemberAccess(_, receiver_expr, method) => {
                    match self.evaluate_expression(receiver_expr).await? {
                        Value::Contract(c) => {
                            Ok(Value::Func(Function::ContractCall(c, method.name.clone())))
                        }
                        v => {
                            let method = BuiltinFunction::with_receiver(&v, &method.name)?;
                            let mut env = self.env.lock().await;
                            if method.is_property() {
                                Ok(method.execute(&[], &mut env, &self.provider).await?)
                            } else {
                                Ok(Value::Func(Function::Builtin(method)))
                            }
                        }
                    }
                }

                Expression::ArraySubscript(_, expr, subscript_opt) => {
                    let lhs = self.evaluate_expression(expr).await?;
                    match lhs {
                        Value::Tuple(values) | Value::Array(values) => {
                            let subscript = subscript_opt.ok_or(anyhow!(
                                "tuples and arrays do not support empty subscript"
                            ))?;
                            let u256_index = match self.evaluate_expression(subscript).await? {
                                Value::Uint(n) => n,
                                Value::Int(n) => n.unchecked_into(),
                                v => bail!("invalid type for subscript, expected int, got {}", v),
                            };
                            if u256_index.ge(&U256::from(values.len() as u64)) {
                                bail!("index out of bounds");
                            }
                            Ok(values[u256_index.to::<usize>()].clone())
                        }
                        v => bail!("invalid type for subscript, expected tuple, got {}", v),
                    }
                }

                Expression::Add(_, lhs, rhs) => self._eval_binop_expr(lhs, rhs, "+").await,
                Expression::Subtract(_, lhs, rhs) => self._eval_binop_expr(lhs, rhs, "-").await,
                Expression::Multiply(_, lhs, rhs) => self._eval_binop_expr(lhs, rhs, "*").await,
                Expression::Divide(_, lhs, rhs) => self._eval_binop_expr(lhs, rhs, "/").await,
                Expression::Modulo(_, lhs, rhs) => self._eval_binop_expr(lhs, rhs, "%").await,
                Expression::Power(_, lhs, rhs) => {
                    let left = self.evaluate_expression(lhs).await?;
                    let right = self.evaluate_expression(rhs).await?;
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

                Expression::FunctionCall(_, func_expr, args_) => {
                    let mut args = vec![];
                    for arg in args_.iter() {
                        args.push(self.evaluate_expression(Box::new(arg.clone())).await?);
                    }
                    match self.evaluate_expression(func_expr).await? {
                        Value::Func(f) => {
                            let mut env = self.env.lock().await;
                            f.execute(&args, &mut env, &self.provider).await
                        }
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
                Expression::Parenthesis(_, expr) => self.evaluate_expression(expr).await,

                v => bail!("{} not supported", v),
            }
        }
        .boxed()
    }

    async fn _eval_comparison(
        &mut self,
        lexpr: Box<Expression>,
        rexpr: Box<Expression>,
        op: fn(Ordering) -> bool,
    ) -> Result<Value> {
        let lhs = self.evaluate_expression(lexpr).await?;
        let rhs = self.evaluate_expression(rexpr).await?;
        match lhs.partial_cmp(&rhs) {
            Some(ordering) => Ok(Value::Bool(op(ordering))),
            None => bail!("cannot compare {} and {}", lhs, rhs),
        }
    }

    async fn _eval_binop_expr(
        &mut self,
        lexpr: Box<Expression>,
        rexpr: Box<Expression>,
        op: &str,
    ) -> Result<Value> {
        let lhs = self.evaluate_expression(lexpr).await?;
        let rhs = self.evaluate_expression(rexpr).await?;
        match (&lhs, &rhs) {
            (Value::Uint(l), Value::Uint(r)) => self._eval_bin_op(*l, *r, op).map(Value::Uint),
            (Value::Int(_), Value::Uint(_))
            | (Value::Uint(_), Value::Int(_))
            | (Value::Int(_), Value::Int(_)) => {
                let (l, r) = match (lhs, rhs) {
                    (Value::Int(l), Value::Int(r)) => (l, r),
                    (Value::Uint(l), Value::Int(r)) => (I256::from_raw(l), r),
                    (Value::Int(l), Value::Uint(r)) => (l, I256::from_raw(r)),
                    _ => unreachable!(),
                };
                self._eval_bin_op(l, r, op).map(Value::Int)
            }
            _ => bail!("{} not supported for {} and {}", op, lhs, rhs),
        }
    }

    fn _eval_bin_op<T>(&self, l: T, r: T, op: &str) -> Result<T>
    where
        T: std::ops::Add<Output = T>
            + std::ops::Sub<Output = T>
            + std::ops::Mul<Output = T>
            + std::ops::Div<Output = T>
            + std::ops::Rem<Output = T>
            + std::fmt::Display,
    {
        match op {
            "+" => Ok(l + r),
            "-" => Ok(l - r),
            "*" => Ok(l * r),
            "/" => Ok(l / r),
            "%" => Ok(l % r),
            _ => bail!("{} not supported for {} and {}", op, l, r),
        }
    }
}
