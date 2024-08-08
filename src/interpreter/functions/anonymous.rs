use std::sync::Arc;

use crate::interpreter::{evaluate_expression, types::HashableIndexMap, Env, Type, Value};

use super::{Function, FunctionDef, FunctionParam};
use anyhow::{anyhow, bail, Result};
use futures::{future::BoxFuture, FutureExt};
use solang_parser::pt::Expression;

#[derive(Debug)]
pub struct AnonymousFunction {
    params: Vec<FunctionParam>,
    body: Expression,
}

impl AnonymousFunction {
    pub fn new(params: Vec<FunctionParam>, body: Expression) -> Self {
        Self { params, body }
    }

    pub fn parse_lhs(lhs: &Expression) -> Result<Vec<FunctionParam>> {
        match lhs {
            Expression::Parenthesis(_, expr) => {
                if let Expression::Variable(id) = expr.as_ref() {
                    Ok(vec![FunctionParam::new(&id.name, Type::Any)])
                } else {
                    bail!("expected variable, found {:?}", expr)
                }
            }
            Expression::List(_, params) => params
                .iter()
                .map(|(_, param)| {
                    let param = param.clone().ok_or(anyhow!("no param given"))?;
                    match (param.name, param.ty) {
                        (Some(id), Expression::Type(_, t)) => {
                            Ok(FunctionParam::new(&id.name, t.try_into()?))
                        }
                        (None, Expression::Variable(id)) => {
                            Ok(FunctionParam::new(&id.name, Type::Any))
                        }
                        _ => bail!("invalid function parameter"),
                    }
                })
                .collect::<Result<Vec<_>>>(),
            _ => bail!("Invalid function parameter"),
        }
    }
}

impl From<AnonymousFunction> for Value {
    fn from(f: AnonymousFunction) -> Self {
        Value::Func(Box::new(Function::new(Arc::new(f), None)))
    }
}

impl FunctionDef for AnonymousFunction {
    fn name(&self) -> String {
        "function".to_string()
    }

    fn get_valid_args(&self, _receiver: &Option<Value>) -> Vec<Vec<FunctionParam>> {
        vec![self.params.clone()]
    }

    fn is_property(&self) -> bool {
        false
    }

    fn execute<'a>(
        &'a self,
        env: &'a mut Env,
        values: &'a [Value],
        _options: &'a HashableIndexMap<String, Value>,
    ) -> BoxFuture<'a, Result<Value>> {
        async move {
            for (param, arg) in self.params.iter().zip(values.iter()) {
                env.set_var(param.get_name(), arg.clone());
            }
            evaluate_expression(env, Box::new(self.body.clone())).await
        }
        .boxed()
    }
}
