use anyhow::Result;
use git2_rs::Repository;

fn get_eclair_version() -> Result<(String, Vec<String>)> {
    let mut warnings = vec![];

    let repo_dir = std::env::current_dir()?;
    let repo = Repository::discover(repo_dir)?;
    let ref_head = repo.revparse_single("HEAD")?;
    let git_sha_buf = ref_head.short_id()?;
    let git_sha = git_sha_buf.as_str();

    let pkg_version = std::env::var("CARGO_PKG_VERSION")?;

    let mut version = format!("Eclair v{}", pkg_version);

    if pkg_version.contains("-dev") {
        if let Some(git_short_sha) = git_sha {
            version = format!("{} ({})", version, git_short_sha);
        } else {
            warnings.push("Failed to get git short sha".to_string());
        }
    }
    Ok((version, warnings))
}

fn main() -> Result<()> {
    let (version, warnings) = get_eclair_version()?;

    for warning in warnings {
        println!("cargo::warning={}", warning);
    }

    println!("cargo::rustc-env=ECLAIR_VERSION={}", version);

    Ok(())
}
