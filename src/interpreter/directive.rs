use anyhow::Result;

pub enum Directive {
    Env,
    Rpc(String),
    Debug,
}

impl Directive {
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
            _ => Err(anyhow::anyhow!("Invalid directive")),
        }
    }
}
