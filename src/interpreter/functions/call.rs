use crate::interpreter::{utils::join_with_final, Env, Value};
use anyhow::{bail, Result};
use std::fmt;

use super::{FunctionDefinition, FunctionParam};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct FunctionCall {
    def: FunctionDefinition,
    receiver: Option<Value>,
}

impl fmt::Display for FunctionCall {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let variants = self.def.get_variants();
        for (i, variant) in variants.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            if let Some(receiver) = &self.receiver {
                write!(f, "{}.", receiver)?;
            }
            write!(f, "{}", variant)?;
        }
        Ok(())
    }
}

impl FunctionCall {
    pub fn new(def: &FunctionDefinition, receiver: Option<&Value>) -> Self {
        FunctionCall {
            def: def.clone(),
            receiver: receiver.cloned(),
        }
    }

    pub fn method(def: &FunctionDefinition, receiver: &Value) -> Self {
        Self::new(def, Some(receiver))
    }

    pub fn function(def: &FunctionDefinition) -> Self {
        Self::new(def, None)
    }

    pub fn is_property(&self) -> bool {
        self.def.is_property()
    }

    pub async fn execute(&self, env: &mut Env, args: &[Value]) -> Result<Value> {
        let mut unified_args = self.get_unified_args(args)?;
        if let Some(receiver) = &self.receiver {
            unified_args.insert(0, receiver.clone());
        }
        self.def.execute(env, &unified_args).await
    }

    fn get_unified_args(&self, args: &[Value]) -> Result<Vec<Value>> {
        let valid_args_lengths = self.def.get_valid_args_lengths();

        // skip validation if no valid args are specified
        if valid_args_lengths.is_empty() {
            return Ok(args.to_vec());
        }

        if !valid_args_lengths.contains(&args.len()) {
            bail!(
                "function {} expects {} arguments, but got {}",
                self,
                join_with_final(", ", " or ", valid_args_lengths),
                args.len()
            );
        }

        let potential_types = self
            .def
            .get_valid_args()
            .iter()
            .filter(|a| a.len() == args.len());

        for (i, arg_types) in potential_types.enumerate() {
            let res = self._unify_types(args, arg_types.as_slice());
            if res.is_ok() || i == valid_args_lengths.len() - 1 {
                return res;
            }
        }

        unreachable!()
    }

    fn _unify_types(&self, args: &[Value], types: &[FunctionParam]) -> Result<Vec<Value>> {
        let mut result = vec![];
        for (i, (arg, param)) in args.iter().zip(types).enumerate() {
            match param.get_type().cast(arg) {
                Ok(v) => result.push(v),
                Err(e) => bail!(
                    "expected {} argument {} to be {}, but got {} ({})",
                    self,
                    i,
                    param.get_type(),
                    arg.get_type(),
                    e
                ),
            }
        }
        Ok(result)
    }
}
