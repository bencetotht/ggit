use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;

pub fn db_path() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("GGIT_DB_PATH") {
        return Ok(PathBuf::from(path));
    }

    let home = dirs::home_dir().ok_or_else(|| anyhow!("could not determine home directory"))?;
    Ok(home.join(".ggit").join("ggit.db"))
}

pub fn ensure_parent(path: &std::path::Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("could not create {}", parent.display()))?;
    }
    Ok(())
}
