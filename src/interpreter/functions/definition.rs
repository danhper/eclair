use crate::interpreter::{functions::FunctionParam, Env, Value};
use anyhow::Result;
use futures::future::BoxFuture;
use itertools::Itertools;

pub type Executor =
    for<'a> fn(&'a FunctionDefinition, &'a mut Env, &'a [Value]) -> BoxFuture<'a, Result<Value>>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionDefinition {
    name_: String,
    property: bool,
    valid_args: Vec<Vec<FunctionParam>>,
    execute_fn: Executor,
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

    pub fn get_variants(&self) -> Vec<String> {
        self.valid_args
            .iter()
            .map(|args| {
                let args = args
                    .iter()
                    .map(|arg| arg.to_string())
                    .collect::<Vec<String>>()
                    .join(", ");
                format!("{}({})", self.name_, args)
            })
            .collect()
    }

    pub fn get_valid_args(&self) -> &Vec<Vec<FunctionParam>> {
        &self.valid_args
    }

    pub fn get_valid_args_lengths(&self) -> Vec<usize> {
        let valid_lengths = self.valid_args.iter().map(|args| args.len());
        valid_lengths.dedup().sorted().collect()
    }

    pub fn execute<'a>(
        &'a self,
        env: &'a mut Env,
        args: &'a [Value],
    ) -> BoxFuture<'a, Result<Value>> {
        (self.execute_fn)(self, env, args)
    }
}

pub struct FunctionDefinitionBuilder {
    name: String,
    property: bool,
    valid_args: Vec<Vec<FunctionParam>>,
    execute_fn: Executor,
}

impl FunctionDefinitionBuilder {
    pub fn new(name: &str, execute_fn: Executor) -> Self {
        Self {
            name: name.to_string(),
            property: false,
            valid_args: vec![],
            execute_fn,
        }
    }

    pub fn property(name: &str, execute_fn: Executor) -> Self {
        Self {
            name: name.to_string(),
            property: true,
            valid_args: vec![vec![]],
            execute_fn,
        }
    }

    pub fn add_valid_args(mut self, valid_args: &[FunctionParam]) -> Self {
        self.valid_args.push(valid_args.to_vec());
        self
    }

    pub fn build(self) -> FunctionDefinition {
        FunctionDefinition {
            name_: self.name,
            property: self.property,
            valid_args: self.valid_args,
            execute_fn: self.execute_fn,
        }
    }
}
