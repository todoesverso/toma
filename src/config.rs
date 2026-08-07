use anyhow::{Context, Result};
use std::{fs, path::Path};

use serde::Deserialize;

fn bind_default() -> String {
    "0.0.0.0".to_string()
}

fn port_default() -> u16 {
    8080
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "bind_default")]
    pub bind: String,
    #[serde(default = "port_default")]
    pub port: u16,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let yaml_str = fs::read_to_string(path.as_ref()).context(format!(
            "Failed to read {}",
            path.as_ref().to_string_lossy().into_owned()
        ))?;
        let config: Config = serde_saphyr::from_str(&yaml_str).context("Failed to parse YAML")?;
        Ok(config)
    }
}
