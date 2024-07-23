use std::sync::Arc;

use crate::interpreter::{functions::FunctionParam, types::HashableIndexMap, Env, Value};
use anyhow::{anyhow, Result};
use futures::{future::BoxFuture, FutureExt};

pub trait FunctionDef: std::fmt::Debug + Send + Sync {
    fn name(&self) -> &str;

    fn get_valid_args(&self, receiver: &Option<Value>) -> Vec<Vec<FunctionParam>>;

    fn is_property(&self) -> bool;

    fn execute<'a>(
        &'a self,
        env: &'a mut Env,
        values: &'a [Value],
        options: &'a HashableIndexMap<String, Value>,
    ) -> BoxFuture<'a, Result<Value>>;

    fn member_access(&self, _receiver: &Option<Value>, _member: &str) -> Option<Value> {
        None
    }
}

#[derive(Debug)]
pub struct SyncProperty {
    name: String,
    f: fn(&Env, &Value) -> Result<Value>,
}

impl SyncProperty {
    pub fn arc(name: &str, f: fn(&Env, &Value) -> Result<Value>) -> Arc<dyn FunctionDef> {
        Arc::new(Self {
            name: name.to_string(),
            f,
        })
    }
}

impl FunctionDef for SyncProperty {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_valid_args(&self, _: &Option<Value>) -> Vec<Vec<FunctionParam>> {
        vec![vec![]]
    }

    fn is_property(&self) -> bool {
        true
    }

    fn execute<'a>(
        &'a self,
        env: &'a mut Env,
        values: &'a [Value],
        _options: &'a HashableIndexMap<String, Value>,
    ) -> BoxFuture<'a, Result<Value>> {
        async move {
            let receiver = values.first().ok_or(anyhow!("no receiver"))?;
            (self.f)(env, receiver)
        }
        .boxed()
    }
}

#[derive(Debug)]
pub struct AsyncProperty {
    name: String,
    f: for<'a> fn(&'a Env, &'a Value) -> BoxFuture<'a, Result<Value>>,
}

impl AsyncProperty {
    pub fn arc(
        name: &str,
        f: for<'a> fn(&'a Env, &'a Value) -> BoxFuture<'a, Result<Value>>,
    ) -> Arc<dyn FunctionDef> {
        Arc::new(Self {
            name: name.to_string(),
            f,
        })
    }
}

impl FunctionDef for AsyncProperty {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_valid_args(&self, _: &Option<Value>) -> Vec<Vec<FunctionParam>> {
        vec![vec![]]
    }

    fn is_property(&self) -> bool {
        true
    }

    fn execute<'a>(
        &'a self,
        env: &'a mut Env,
        values: &'a [Value],
        _options: &'a HashableIndexMap<String, Value>,
    ) -> BoxFuture<'a, Result<Value>> {
        async move {
            let receiver = values.first().ok_or(anyhow!("no receiver"))?;
            (self.f)(env, receiver).await
        }
        .boxed()
    }
}

#[derive(Debug)]
pub struct SyncMethod {
    name: String,
    f: fn(&mut Env, &Value, &[Value]) -> Result<Value>,
    valid_args: Vec<Vec<FunctionParam>>,
}

impl SyncMethod {
    pub fn arc(
        name: &str,
        f: fn(&mut Env, &Value, &[Value]) -> Result<Value>,
        valid_args: Vec<Vec<FunctionParam>>,
    ) -> Arc<dyn FunctionDef> {
        Arc::new(Self {
            name: name.to_string(),
            f,
            valid_args,
        })
    }
}

impl FunctionDef for SyncMethod {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_valid_args(&self, _: &Option<Value>) -> Vec<Vec<FunctionParam>> {
        self.valid_args.clone()
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
            let receiver = values.first().ok_or(anyhow!("no receiver"))?;
            (self.f)(env, receiver, &values[1..])
        }
        .boxed()
    }
}

#[derive(Debug)]
pub struct SyncFunction {
    name: String,
    f: fn(&Env, &[Value]) -> Result<Value>,
    valid_args: Vec<Vec<FunctionParam>>,
}

impl SyncFunction {
    pub fn arc(
        name: &str,
        f: fn(&Env, &[Value]) -> Result<Value>,
        valid_args: Vec<Vec<FunctionParam>>,
    ) -> Arc<dyn FunctionDef> {
        Arc::new(Self {
            name: name.to_string(),
            f,
            valid_args,
        })
    }
}

impl FunctionDef for SyncFunction {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_valid_args(&self, _: &Option<Value>) -> Vec<Vec<FunctionParam>> {
        self.valid_args.clone()
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
        async move { (self.f)(env, values) }.boxed()
    }
}

#[derive(Debug)]
pub struct AsyncMethod {
    name: String,
    f: for<'a> fn(&'a mut Env, &'a Value, &'a [Value]) -> BoxFuture<'a, Result<Value>>,
    valid_args: Vec<Vec<FunctionParam>>,
}

impl AsyncMethod {
    pub fn arc(
        name: &str,
        f: for<'a> fn(&'a mut Env, &'a Value, &'a [Value]) -> BoxFuture<'a, Result<Value>>,
        valid_args: Vec<Vec<FunctionParam>>,
    ) -> Arc<dyn FunctionDef> {
        Arc::new(Self {
            name: name.to_string(),
            f,
            valid_args,
        })
    }
}

impl FunctionDef for AsyncMethod {
    fn name(&self) -> &str {
        &self.name
    }

    fn get_valid_args(&self, _: &Option<Value>) -> Vec<Vec<FunctionParam>> {
        self.valid_args.clone()
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
            let receiver = values.first().ok_or(anyhow!("no receiver"))?;
            (self.f)(env, receiver, &values[1..]).await
        }
        .boxed()
    }
}
