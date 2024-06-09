use anyhow::{anyhow, Result};
use solang_parser::pt::{ContractPart, SourceUnitPart, Statement};

fn wrap_statement(stmt: &str) -> String {
    let mut statement = stmt.to_owned();
    if !statement.ends_with(';') {
        statement = format!("{};", statement);
    }
    format!(
        r#"contract ReplContract {{
    function replFunction() external {{
        {}
    }}
}}
"#,
        statement
    )
}

pub fn parse_statement(raw_stmt: &str) -> Result<Statement> {
    let code = wrap_statement(raw_stmt);
    let (tree, _comments) =
        solang_parser::parse(&code, 0).map_err(|d| {
            anyhow!(
                "Parse error: {}",
                d.iter()
                    .map(|d| format!("{:?}", d))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        })?;

    let function_parts = match &tree.0[0] {
        SourceUnitPart::ContractDefinition(def) => def.parts.clone(),
        _ => return Err(anyhow!("Parse error: {}", code)),
    };
    let func = match &function_parts[0] {
        ContractPart::FunctionDefinition(def) => def.body.clone().unwrap(),
        _ => return Err(anyhow!("Parse error: {}", code)),
    };
    let statements = match &func {
        Statement::Block { statements, .. } => statements.clone(),
        _ => return Err(anyhow!("Parse error: {}", code)),
    };

    if statements.len() != 1 {
        return Err(anyhow!("should only contain one statement"));
    }
    Ok(statements[0].clone())
}
