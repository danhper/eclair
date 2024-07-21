use crate::interpreter::{evaluate_statement, Env, Value};

use super::{FunctionDefinition, FunctionDefinitionBuilder, FunctionParam};
use anyhow::{anyhow, Result};
use futures::{future::BoxFuture, FutureExt};

fn execute_user_defined<'a>(
    def: &'a FunctionDefinition,
    env: &'a mut Env,
    args: &'a [Value],
) -> BoxFuture<'a, Result<Value>> {
    async move {
        let params = def
            .get_valid_args()
            .first()
            .ok_or(anyhow!("require params"))?;
        for (param, arg) in params.iter().zip(args.iter()) {
            let casted_arg = param.get_type().cast(arg)?;
            env.set_var(param.get_name(), casted_arg);
        }
        let body = env
            .get_function_body(def.name())
            .ok_or(anyhow!("require function body"))?;
        evaluate_statement(env, Box::new(body.clone()))
            .await
            .map(|v| v.value().cloned().unwrap_or(Value::Null))
    }
    .boxed()
}

impl TryFrom<solang_parser::pt::FunctionDefinition> for FunctionDefinition {
    type Error = anyhow::Error;

    fn try_from(f: solang_parser::pt::FunctionDefinition) -> Result<Self> {
        let name = f.name.clone().ok_or(anyhow!("require function name"))?.name;
        let params = f
            .params
            .iter()
            .map(|(_, p)| {
                p.clone()
                    .ok_or(anyhow!("require param"))
                    .and_then(FunctionParam::try_from)
            })
            .collect::<Result<Vec<_>>>()?;
        let func = FunctionDefinitionBuilder::new(&name, execute_user_defined)
            .add_valid_args(&params)
            .build();
        Ok(func)
    }
}
