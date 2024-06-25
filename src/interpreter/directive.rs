use anyhow::Result;

pub enum Directive {
    ListVars,
    ListTypes,
    ShowRpc,
    SetRpc(String),
    Debug,
    Exec(String, Vec<String>),
}

impl Directive {
    pub fn all() -> Vec<String> {
        ["!types", "!vars", "!rpc", "!debug", "!exec"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    pub fn parse(line: &str) -> Result<Self> {
        let mut parts = line.split_whitespace();
        let directive = parts.next().ok_or(anyhow::anyhow!("Empty directive"))?;
        match directive {
            "vars" => Ok(Directive::ListVars),
            "types" => Ok(Directive::ListTypes),
            "rpc" => {
                let url = parts.next().unwrap_or_default().to_string();
                if url.is_empty() {
                    return Ok(Directive::ShowRpc);
                }
                Ok(Directive::SetRpc(url))
            }
            "debug" => Ok(Directive::Debug),
            "exec" | "e" => {
                let cmd = parts.next().unwrap_or_default().to_string();
                if cmd.is_empty() {
                    return Err(anyhow::anyhow!("!exec directive requires a command"));
                }
                let args = parts.map(|s| s.to_string()).collect();
                Ok(Directive::Exec(cmd, args))
            }
            _ => Err(anyhow::anyhow!("Invalid directive")),
        }
    }
}
