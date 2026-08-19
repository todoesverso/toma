use std::{fmt, path::PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ServiceType {
    File { path: PathBuf },
    PlainText { text: String },
}

fn enabled_default() -> bool {
    true
}
fn description_default() -> String {
    "Service".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct Service {
    pub name: String,
    #[serde(default = "description_default")]
    pub description: String,
    pub route: String,
    #[serde(default = "enabled_default")]
    pub enabled: bool,
    #[serde(flatten)]
    pub config: ServiceType,
}

impl fmt::Display for ServiceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::File { .. } => "file",
            Self::PlainText { .. } => "plain-text",
        };
        write!(f, "{}", s)
    }
}
