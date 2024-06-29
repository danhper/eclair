use std::fmt::Display;

use anyhow::{bail, Result};
use itertools::Itertools;
use solang_parser::pt::Expression;

use super::{Env, Value};

#[derive(Debug, Clone)]
pub enum Lhs {
    Empty,
    Name(String),
    Member(String, String),
    Components(Vec<Lhs>),
}

impl Display for Lhs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Lhs::Empty => write!(f, ""),
            Lhs::Name(name) => write!(f, "{}", name),
            Lhs::Member(name, field) => write!(f, "{}.{}", name, field),
            Lhs::Components(components) => {
                let items = components.iter().map(|c| format!("{}", c)).join(", ");
                write!(f, "{}", items)
            }
        }
    }
}

impl Lhs {
    pub fn try_from_expr(expr: Expression) -> Result<Self> {
        match expr {
            Expression::Variable(id) => Ok(Lhs::Name(id.to_string())),
            Expression::MemberAccess(_, expr, member) => match expr.as_ref() {
                Expression::Variable(id) => Ok(Lhs::Member(id.to_string(), member.to_string())),
                _ => bail!("can only use member access on variables in lhs"),
            },
            Expression::ArrayLiteral(_, exprs) => exprs
                .into_iter()
                .map(Lhs::try_from_expr)
                .collect::<Result<Vec<_>>>()
                .map(Lhs::Components),
            Expression::List(_, params) => {
                let mut components = Vec::new();
                for opt_param in params.iter() {
                    if let (_, Some(param)) = opt_param {
                        if let Some(name) = param.name.clone() {
                            components.push(Lhs::Name(name.to_string()));
                        } else {
                            components.push(Lhs::try_from_expr(param.ty.clone())?);
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

    pub fn execute_assign(&self, value: Value, env: &mut Env) -> Result<()> {
        match self {
            Lhs::Name(name) if env.get_var(name).map_or(false, Value::is_builtin) => {
                bail!("cannot assign to builtin variable {}", name)
            }
            Lhs::Name(name) => env.set_var(name, value),
            Lhs::Member(name, field) => match env.get_var_mut(name) {
                Some(Value::NamedTuple(_, v)) => {
                    v.insert(field.clone(), value);
                }
                Some(v) => bail!("trying to assign field to {}", v.get_type()),
                None => bail!("variable {} not found", name),
            },
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
