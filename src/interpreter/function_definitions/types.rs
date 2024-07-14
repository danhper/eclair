use core::fmt;

use crate::interpreter::{Env, Type, Value};
use anyhow::Result;
use futures::future::BoxFuture;
use itertools::Itertools;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionParam {
    name: String,
    type_: Type,
}

impl FunctionParam {
    pub fn new(name: &str, type_: Type) -> Self {
        Self {
            name: name.to_string(),
            type_,
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_type(&self) -> &Type {
        &self.type_
    }
}

impl fmt::Display for FunctionParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.type_, self.name)
    }
}

pub type Executor = for<'a> fn(&'a mut Env, &'a [Value]) -> BoxFuture<'a, Result<Value>>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionDefinition {
    pub(crate) name_: String,
    pub(crate) property: bool,
    pub(crate) valid_args: Vec<Vec<FunctionParam>>,
    pub(crate) execute_fn: Executor,
}

impl std::fmt::Display for FunctionDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_property() {
            return write!(f, "{}", self.name_);
        }

        if self.valid_args.is_empty() {
            return write!(f, "{}(...)", self.name_);
        }

        for (i, args) in self.valid_args.iter().enumerate() {
            if i > 0 {
                write!(f, "  | ")?;
            }
            write!(f, "{}(", self.name_)?;
            for (j, arg) in args.iter().enumerate() {
                if j > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", arg)?;
            }
            write!(f, ")")?;
            if i != self.valid_args.len() - 1 {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

impl FunctionDefinition {
    pub fn name(&self) -> &str {
        &self.name_
    }

    pub fn is_property(&self) -> bool {
        self.property
    }

    pub fn is_method(&self) -> bool {
        self.property
    }

    pub fn get_valid_args(&self) -> &Vec<Vec<FunctionParam>> {
        &self.valid_args
    }

    pub fn get_valid_args_lengths(&self) -> Vec<usize> {
        let valid_lengths = self.valid_args.iter().map(|args| args.len());
        valid_lengths.dedup().sorted().collect()
    }

    pub fn execute<'a>(&self, env: &'a mut Env, args: &'a [Value]) -> BoxFuture<'a, Result<Value>> {
        (self.execute_fn)(env, args)
    }
}
