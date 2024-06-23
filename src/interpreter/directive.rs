use anyhow::Result;

pub enum Directive {
    Env,
    Rpc(String),
    Debug,
    Exec(String, Vec<String>),
}

impl Directive {
    pub fn all() -> Vec<String> {
        ["!env", "!rpc", "!debug", "!exec"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    pub fn parse(line: &str) -> Result<Self> {
        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("!env") => Ok(Directive::Env),
            Some("!rpc") => {
                let url = parts.next().unwrap_or_default().to_string();
                if url.is_empty() {
                    return Err(anyhow::anyhow!("!rpc directive requires a URL"));
                }
                Ok(Directive::Rpc(url))
            }
            Some("!debug") => Ok(Directive::Debug),
            Some("!exec") => {
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
