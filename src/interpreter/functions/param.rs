use alloy::{dyn_abi::Specifier, json_abi::Param};
use anyhow::{bail, Result};
use solang_parser::pt::{Expression, Identifier, Parameter};
use std::fmt;

use crate::interpreter::Type;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionParam {
    name: String,
    type_: Type,
    vararg: bool,
}

impl FunctionParam {
    pub fn new(name: &str, type_: Type) -> Self {
        Self {
            name: name.to_string(),
            type_,
            vararg: false,
        }
    }

    pub fn vararg(name: &str, type_: Type) -> Self {
        Self {
            name: name.to_string(),
            type_,
            vararg: true,
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

impl TryFrom<Param> for FunctionParam {
    type Error = anyhow::Error;

    fn try_from(param: Param) -> std::result::Result<Self, Self::Error> {
        let type_ = param.resolve()?.into();
        Ok(FunctionParam::new(&param.name, type_))
    }
}

impl TryFrom<Parameter> for FunctionParam {
    type Error = anyhow::Error;

    fn try_from(p: Parameter) -> Result<Self> {
        match (p.name, p.ty) {
            (Some(Identifier { name, .. }), Expression::Type(_, t)) => {
                let type_ = t.try_into()?;
                Ok(FunctionParam::new(&name, type_))
            }
            (None, Expression::Variable(Identifier { name, .. })) => {
                Ok(FunctionParam::new(&name, Type::Any))
            }
            _ => bail!("require param name or type and name"),
        }
    }
}
