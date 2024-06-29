use anyhow::{anyhow, Result};
use itertools::Itertools;
use solang_parser::pt::{ContractDefinition, ContractPart, SourceUnitPart, Statement};

fn wrap_contract(function: &str) -> String {
    format!(
        r#"contract ReplContract {{
            {}
}}
"#,
        function
    )
}

fn wrap_statement(stmt: &str) -> String {
    let mut statement = stmt.to_owned();
    let last_non_whitespace = statement.trim_end().chars().last().unwrap_or(';');
    if last_non_whitespace != ';' && last_non_whitespace != '}' {
        statement = format!("{};", statement);
    }
    wrap_contract(&format!(
        r#"function replFunction() external {{
        {}
}}
"#,
        statement
    ))
}

#[derive(Debug)]
pub enum ParsedCode {
    Statements(Vec<Statement>),
    ContractDefinition(ContractDefinition),
}

fn parse_code(code: &str) -> Result<ContractDefinition> {
    match solang_parser::parse(code, 0) {
        Ok((tree, _comments)) => match &tree.0[0] {
            SourceUnitPart::ContractDefinition(def) => Ok(*def.clone()),
            _ => Err(anyhow!("parse error: {}", code)),
        },
        Err(e) => Err(anyhow!(
            "parse error: {}",
            e.iter().map(|d| d.message.clone()).join("\n")
        )),
    }
}

pub fn parse_input(input: &str) -> Result<ParsedCode> {
    match parse_code(&wrap_statement(input)) {
        Ok(ContractDefinition { parts, .. }) => {
            let func = match &parts[0] {
                ContractPart::FunctionDefinition(def) => {
                    def.body.clone().expect("require function body")
                }
                _ => return Err(anyhow!("parse error: {}", input)),
            };
            let statements = match &func {
                Statement::Block { statements, .. } => statements.clone(),
                _ => return Err(anyhow!("parse error: {}", input)),
            };

            Ok(ParsedCode::Statements(statements.clone()))
        }
        Err(e) => match parse_code(&wrap_contract(input)) {
            Ok(def @ ContractDefinition { .. }) => Ok(ParsedCode::ContractDefinition(def)),
            Err(_) => Err(e),
        },
    }
}
