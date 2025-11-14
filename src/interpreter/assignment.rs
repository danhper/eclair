use std::fmt::Display;

use anyhow::{anyhow, bail, Result};
use futures::{future::BoxFuture, FutureExt};
use itertools::Itertools;
use solang_parser::pt::Expression;

use super::{evaluate_expression, Env, Value};

#[derive(Debug, Clone)]
pub enum Lhs {
    Empty,
    Name(String),
    Member(String, String),
    Index(String, Value),
    Components(Vec<Lhs>),
}

impl Display for Lhs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Lhs::Empty => write!(f, ""),
            Lhs::Name(name) => write!(f, "{}", name),
            Lhs::Member(name, field) => write!(f, "{}.{}", name, field),
            Lhs::Index(name, index) => write!(f, "{}[{}]", name, index),
            Lhs::Components(components) => {
                let items = components.iter().map(|c| format!("{}", c)).join(", ");
                write!(f, "{}", items)
            }
        }
    }
}

impl Lhs {
    pub fn try_from_expr(expr: Expression, env: &mut Env) -> BoxFuture<'_, Result<Self>> {
        async move {
            match expr {
                Expression::Variable(id) => Ok(Lhs::Name(id.to_string())),
                Expression::MemberAccess(_, expr, member) => match expr.as_ref() {
                    Expression::Variable(id) => Ok(Lhs::Member(id.to_string(), member.to_string())),
                    _ => bail!("can only use member access on variables in lhs"),
                },
                Expression::ArrayLiteral(_, exprs) => {
                    let mut components = vec![];
                    for expr in exprs {
                        components.push(Lhs::try_from_expr(expr, env).await?);
                    }
                    Ok(Lhs::Components(components))
                }
                Expression::ArraySubscript(_, var_expr, sub) => {
                    let var = match var_expr.as_ref() {
                        Expression::Variable(id) => id.to_string(),
                        _ => bail!("can only use member access on variables in lhs"),
                    };
                    let index =
                        evaluate_expression(env, sub.ok_or(anyhow!("no index given"))?).await?;
                    Ok(Lhs::Index(var, index))
                }
                Expression::List(_, params) => {
                    let mut components = Vec::new();
                    for opt_param in params.iter() {
                        if let (_, Some(param)) = opt_param {
                            if let Some(name) = param.name.clone() {
                                components.push(Lhs::Name(name.to_string()));
                            } else {
                                components.push(Lhs::try_from_expr(param.ty.clone(), env).await?);
                            }
                        } else {
                            components.push(Lhs::Empty);
                        }
                    }
                    Ok(Lhs::Components(components))
                }
                _ => bail!("expected variable, found {:?}", expr),
            }
        }
        .boxed()
    }

    pub fn execute_assign(&self, value: Value, env: &mut Env) -> Result<()> {
        match self {
            Lhs::Name(name) if env.get_var(name).is_some_and(Value::is_builtin) => {
                bail!("cannot assign to builtin variable {}", name)
            }
            Lhs::Name(name) => env.set_var(name, value),
            Lhs::Member(name, field) => match env.get_var_mut(name) {
                Some(Value::NamedTuple(_, v)) => {
                    v.0.insert(field.clone(), value);
                }
                Some(v) => bail!("trying to assign field to {}", v.get_type()),
                None => bail!("variable {} not found", name),
            },
            Lhs::Index(name, index) => {
                let lhs_value = env
                    .get_var_mut(name)
                    .ok_or(anyhow!("variable {} not found", name))?;
                lhs_value.set_index(index, value.clone())?;
            }
            Lhs::Components(components) => {
                let items = value.get_items()?;
                if components.len() != items.len() {
                    bail!(
                        "trying to assign {} values to {} variables",
                        items.len(),
                        components.len()
                    );
                }
                for (component, item) in components.iter().zip(items.iter()) {
                    component.execute_assign(item.clone(), env)?;
                }
            }
            Lhs::Empty => (),
        };
        Ok(())
    }
}
