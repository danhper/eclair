use anyhow::Result;
use git2_rs::Repository;
use std::collections::BTreeMap;

use vergen::{AddCustomEntries, CargoRerunIfChanged, CargoWarning, DefaultConfig, Emitter};

#[derive(Default)]
struct VersionEntryBuilder {}

impl AddCustomEntries<&str, String> for VersionEntryBuilder {
    fn add_calculated_entries(
        &self,
        _idempotent: bool,
        cargo_rustc_env_map: &mut BTreeMap<&str, String>,
        _cargo_rerun_if_changed: &mut CargoRerunIfChanged,
        cargo_warning: &mut CargoWarning,
    ) -> Result<()> {
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
                cargo_warning.push("Failed to get git short sha".to_string());
            }
        }

        cargo_rustc_env_map.insert("ECLAIR_VERSION", version);

        Ok(())
    }

    fn add_default_entries(
        &self,
        _config: &DefaultConfig,
        _cargo_rustc_env_map: &mut BTreeMap<&str, String>,
        _cargo_rerun_if_changed: &mut CargoRerunIfChanged,
        _cargo_warning: &mut CargoWarning,
    ) -> Result<()> {
        Ok(())
    }
}

fn main() -> Result<()> {
    Emitter::default()
        .add_custom_instructions(&VersionEntryBuilder::default())?
        .emit()
}
