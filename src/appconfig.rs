use std::{fs::File, io::BufReader, path::Path};

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use serde::{Deserialize, Serialize};

use crate::ui::options::Options;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub camera_index: usize,
    pub printer_url: String,
    pub num_tools: usize,
    pub bounce_amount: f64,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            camera_index: 0,
            printer_url: "".to_string(),
            num_tools: 1,
            bounce_amount: 0.5,
        }
    }
}

impl AppSettings {
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let s = toml::to_string_pretty(self).context("Failed to serialize AppSettings to TOML")?;
        std::fs::write(path, s)?;
        Ok(())
    }
}

pub fn read_options_from_file<P: AsRef<Path>>(path: P, options: &mut Options) -> Result<()> {
    let appsettings: AppSettings = toml::from_str(&std::fs::read_to_string(&path)?)?;

    options.printer_url = appsettings.printer_url;
    options.camera_index = appsettings.camera_index.to_string();
    options.num_tools = appsettings.num_tools;
    options.bounce_amount = appsettings.bounce_amount;

    Ok(())
}
