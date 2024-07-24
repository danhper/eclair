use std::sync::Arc;

use crate::interpreter::{evaluate_statement, types::HashableIndexMap, Env, Value};

use super::{Function, FunctionDef, FunctionParam};
use anyhow::{anyhow, Result};
use futures::{future::BoxFuture, FutureExt};
use solang_parser::pt::Statement;

#[derive(Debug)]
pub struct UserDefinedFunction {
    func_name: String,
    params: Vec<FunctionParam>,
    body: Statement,
}

impl From<UserDefinedFunction> for Value {
    fn from(f: UserDefinedFunction) -> Self {
        Value::Func(Box::new(Function::new(Arc::new(f), None)))
    }
}

impl FunctionDef for UserDefinedFunction {
    fn name(&self) -> String {
        self.func_name.clone()
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
            evaluate_statement(env, Box::new(self.body.clone()))
                .await
                .map(|v| v.value().cloned().unwrap_or(Value::Null))
        }
        .boxed()
    }
}

impl TryFrom<solang_parser::pt::FunctionDefinition> for UserDefinedFunction {
    type Error = anyhow::Error;

    fn try_from(f: solang_parser::pt::FunctionDefinition) -> Result<Self> {
        let name = f.name.clone().ok_or(anyhow!("require function name"))?.name;
        let body = f.body.clone().ok_or(anyhow!("missing function body"))?;
        let params = f
            .params
            .iter()
            .map(|(_, p)| {
                p.clone()
                    .ok_or(anyhow!("require param"))
                    .and_then(FunctionParam::try_from)
            })
            .collect::<Result<Vec<_>>>()?;

        let func = UserDefinedFunction {
            func_name: name,
            params,
            body,
        };
        Ok(func)
    }
}
