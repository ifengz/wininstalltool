#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use thiserror::Error;

pub const DEFAULT_CONFIG_PATH: &str = "config/apps.example.json";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AppManifest {
    pub schema_version: u32,
    pub generated_at: String,
    pub default_install_root: String,
    pub apps: Vec<AppEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub category: String,
    pub homepage_url: Option<String>,
    pub enabled_by_default: bool,
    pub verification_status: String,
    pub source: PackageSource,
    pub install: InstallSpec,
    pub detect: DetectSpec,
    pub notes: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PackageSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub package_id: Option<String>,
    pub url: Option<String>,
    pub repo: Option<String>,
    pub asset_pattern: Option<String>,
}

impl PackageSource {
    pub fn label(&self) -> String {
        match (&self.package_id, &self.url, &self.repo) {
            (Some(package_id), _, _) => format!("{}: {}", self.source_type, package_id),
            (_, Some(url), _) => format!("{}: {}", self.source_type, url),
            (_, _, Some(repo)) => format!("{}: {}", self.source_type, repo),
            _ => self.source_type.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InstallSpec {
    pub method: String,
    pub requires_admin: bool,
    pub supports_custom_path: bool,
    pub args: Option<Vec<String>>,
    pub silent_args: Option<Vec<String>>,
    pub direct_silent_args: Option<Vec<String>>,
    pub direct_install_location_arg: Option<String>,
    pub fallback_notes: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DetectSpec {
    #[serde(rename = "type")]
    pub detect_type: String,
    pub rules: Vec<DetectRule>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DetectRule {
    #[serde(rename = "type")]
    pub rule_type: String,
    pub value: String,
}

#[derive(Debug, Error)]
pub enum LoadConfigError {
    #[error("failed to read config: {0}")]
    Read(#[from] std::io::Error),
    #[error("failed to parse config json: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("config contains duplicate app id: {0}")]
    DuplicateAppId(String),
}

#[derive(Debug, Error)]
pub enum SaveConfigError {
    #[error("failed to serialize config json: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("failed to write config: {0}")]
    Write(#[from] std::io::Error),
}

impl AppManifest {
    pub fn load_from_default_path() -> Result<Self, LoadConfigError> {
        Self::load_from_path(DEFAULT_CONFIG_PATH)
    }

    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, LoadConfigError> {
        let data = fs::read_to_string(path)?;
        let manifest: Self = serde_json::from_str(&data)?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn save_to_default_path(&self) -> Result<(), SaveConfigError> {
        self.save_to_path(DEFAULT_CONFIG_PATH)
    }

    pub fn save_to_path(&self, path: impl AsRef<Path>) -> Result<(), SaveConfigError> {
        let data = serde_json::to_string_pretty(self)?;
        fs::write(path, format!("{data}\n"))?;
        Ok(())
    }

    fn validate(&self) -> Result<(), LoadConfigError> {
        let mut ids = std::collections::HashSet::new();

        for app in &self.apps {
            if !ids.insert(app.id.as_str()) {
                return Err(LoadConfigError::DuplicateAppId(app.id.clone()));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::AppManifest;

    #[test]
    fn loads_example_manifest() {
        let manifest = AppManifest::load_from_default_path().expect("apps example should load");

        assert_eq!(manifest.schema_version, 1);
        assert!(manifest.apps.iter().any(|app| app.id == "chrome"));
        assert!(manifest.apps.iter().any(|app| app.id == "notepadpp"));
    }
}
