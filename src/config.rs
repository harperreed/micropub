// ABOUTME: Configuration management for micropub CLI
// ABOUTME: Handles XDG directories, config file parsing, and profile management

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Get the XDG config directory for micropub
pub fn get_config_dir() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Could not determine config directory")?
        .join("micropub");

    fs::create_dir_all(&config_dir)
        .context("Failed to create config directory")?;

    Ok(config_dir)
}

/// Get the XDG data directory for micropub
pub fn get_data_dir() -> Result<PathBuf> {
    let data_dir = dirs::data_dir()
        .context("Could not determine data directory")?
        .join("micropub");

    fs::create_dir_all(&data_dir)
        .context("Failed to create data directory")?;

    Ok(data_dir)
}

/// Get the drafts directory
pub fn get_drafts_dir() -> Result<PathBuf> {
    let drafts_dir = get_data_dir()?.join("drafts");
    fs::create_dir_all(&drafts_dir)?;
    Ok(drafts_dir)
}

/// Get the archive directory
pub fn get_archive_dir() -> Result<PathBuf> {
    let archive_dir = get_data_dir()?.join("archive");
    fs::create_dir_all(&archive_dir)?;
    Ok(archive_dir)
}

/// Get the tokens directory
pub fn get_tokens_dir() -> Result<PathBuf> {
    let tokens_dir = get_data_dir()?.join("tokens");
    fs::create_dir_all(&tokens_dir)?;
    Ok(tokens_dir)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub default_profile: String,
    pub editor: Option<String>,
    pub profiles: HashMap<String, Profile>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Profile {
    pub domain: String,
    pub micropub_endpoint: Option<String>,
    pub media_endpoint: Option<String>,
    pub token_endpoint: Option<String>,
    pub authorization_endpoint: Option<String>,
}

impl Config {
    /// Load config from file, or create default if not exists
    pub fn load() -> Result<Self> {
        let config_path = get_config_dir()?.join("config.toml");

        if config_path.exists() {
            let contents = fs::read_to_string(&config_path)
                .context("Failed to read config file")?;
            let config: Config = toml::from_str(&contents)
                .context("Failed to parse config file")?;
            Ok(config)
        } else {
            // Return default config
            Ok(Config {
                default_profile: String::new(),
                editor: None,
                profiles: HashMap::new(),
            })
        }
    }

    /// Save config to file
    pub fn save(&self) -> Result<()> {
        let config_path = get_config_dir()?.join("config.toml");
        let contents = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;
        fs::write(&config_path, contents)
            .context("Failed to write config file")?;
        Ok(())
    }

    /// Get a profile by name
    pub fn get_profile(&self, name: &str) -> Option<&Profile> {
        self.profiles.get(name)
    }

    /// Add or update a profile
    pub fn upsert_profile(&mut self, name: String, profile: Profile) {
        self.profiles.insert(name, profile);
    }
}

/// Load authentication token for a profile
pub fn load_token(profile_name: &str) -> Result<String> {
    let token_path = get_tokens_dir()?.join(format!("{}.token", profile_name));
    let token = fs::read_to_string(&token_path)
        .context("Token not found. Run 'micropub auth <domain>' to authenticate")?
        .trim()
        .to_string();

    if token.is_empty() {
        anyhow::bail!("Token file is empty. Re-authenticate with: micropub auth <domain>");
    }

    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serialization() {
        let mut config = Config {
            default_profile: "test".to_string(),
            editor: Some("vim".to_string()),
            profiles: HashMap::new(),
        };

        config.upsert_profile(
            "test".to_string(),
            Profile {
                domain: "example.com".to_string(),
                micropub_endpoint: Some("https://example.com/micropub".to_string()),
                media_endpoint: None,
                token_endpoint: None,
                authorization_endpoint: None,
            },
        );

        let toml = toml::to_string(&config).unwrap();
        assert!(toml.contains("example.com"));
    }
}
