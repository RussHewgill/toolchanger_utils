use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SavedData {
    pub camera_position: (f64, f64),
}

impl SavedData {
    pub fn save_to_file<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        let s = toml::to_string(&self)?;
        std::fs::write(path, s)?;
        Ok(())
    }

    pub fn load_from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let s = std::fs::read_to_string(path)?;
        let data: SavedData = toml::from_str(&s)?;
        Ok(data)
    }
}
