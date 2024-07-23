use crate::interpreter::{types::HashableIndexMap, utils::join_with_final, Env, Value};
use anyhow::{anyhow, bail, Result};
use itertools::Itertools;
use std::{fmt, sync::Arc};

use super::{definition::FunctionDef, FunctionParam};

#[derive(Debug, Clone)]
pub struct Function {
    def: Arc<dyn FunctionDef>,
    receiver: Option<Value>,
    options: HashableIndexMap<String, Value>,
}

impl std::hash::Hash for Function {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.receiver.hash(state);
        self.def.name().hash(state);
        let args = self.def.get_valid_args(&self.receiver);
        args.hash(state)
    }
}

impl std::cmp::PartialEq for Function {
    fn eq(&self, other: &Self) -> bool {
        if self.receiver != other.receiver || self.def.name() != other.def.name() {
            return false;
        }
        let args = self.def.get_valid_args(&self.receiver);
        let other_args = other.def.get_valid_args(&other.receiver);
        args == other_args
    }
}

impl std::cmp::Eq for Function {}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let variants = self.get_variants();
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

impl Function {
    pub fn new(def: Arc<dyn FunctionDef>, receiver: Option<&Value>) -> Self {
        Function {
            def: def.clone(),
            receiver: receiver.cloned(),
            options: HashableIndexMap::default(),
        }
    }

    pub fn member_access(&self, member: &str) -> Result<Value> {
        self.def
            .member_access(&self.receiver, member)
            .ok_or(anyhow!("no member {} for {}", member, self))
    }

    pub fn with_opts(self, opts: HashableIndexMap<String, Value>) -> Self {
        let mut new = self;
        new.options = opts;
        new
    }

    pub fn get_valid_args_lengths(&self) -> Vec<usize> {
        let args = self.def.get_valid_args(&self.receiver);
        let valid_lengths = args.iter().map(|args| args.len());
        valid_lengths.dedup().sorted().collect()
    }

    pub fn get_variants(&self) -> Vec<String> {
        self.def
            .get_valid_args(&self.receiver)
            .iter()
            .map(|args| {
                let args = args
                    .iter()
                    .map(|arg| arg.to_string())
                    .collect::<Vec<String>>()
                    .join(", ");
                format!("{}({})", self.def.name(), args)
            })
            .collect()
    }

    pub fn method(def: Arc<dyn FunctionDef>, receiver: &Value) -> Self {
        Self::new(def, Some(receiver))
    }

    pub fn is_property(&self) -> bool {
        self.def.is_property()
    }

    pub async fn execute(&self, env: &mut Env, args: &[Value]) -> Result<Value> {
        env.push_scope();
        let result = self.execute_in_current_scope(env, args).await;
        env.pop_scope();
        result
    }

    pub async fn execute_in_current_scope(&self, env: &mut Env, args: &[Value]) -> Result<Value> {
        let mut unified_args = self.get_unified_args(args)?;
        if let Some(receiver) = &self.receiver {
            unified_args.insert(0, receiver.clone());
        }
        self.def.execute(env, &unified_args, &self.options).await
    }

    fn get_unified_args(&self, args: &[Value]) -> Result<Vec<Value>> {
        let valid_args_lengths = self.get_valid_args_lengths();

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

        let valid_args = self.def.get_valid_args(&self.receiver);
        let potential_types = valid_args.iter().filter(|a| a.len() == args.len());

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
