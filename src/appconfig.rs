use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

// #[derive(Debug, Default, Clone, Serialize, Deserialize)]
// #[serde(default)]
// pub struct AppSettings {
//     pub camera_index: Option<String>,
//     pub printer_url: Option<String>,
//     pub num_tools: Option<usize>,
//     pub bounce_amount: Option<f64>,
// }

// pub fn read_options_from_file<P: AsRef<Path>>(path: P) -> Result<Options> {
//     let file = File::open(path)?;
//     let reader = BufReader::new(file);
//     // let options: Options = toml::from_str(&std::fs::read_to_string(&path)?)?;

//     // Ok(options)
//     unimplemented!()
// }
