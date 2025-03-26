use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SavedData {
    camera_position: (f64, f64),
}

impl SavedData {
    pub fn save_to_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        let s = toml::to_string(&self)?;
        std::fs::write(path, s)?;
        Ok(())
    }
}
