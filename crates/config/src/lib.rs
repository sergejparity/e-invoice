use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const APP_NAME: &str = "lv-einvoice-app";
const KEYCHAIN_SERVICE: &str = "lv.einvoice.credentials";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub provider: ProviderConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            provider: ProviderConfig {
                kind: "mock".to_string(),
                base_url: None,
                client_id: None,
                token_url: None,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderConfig {
    #[serde(default = "default_provider_kind")]
    pub kind: String, // "mock" | "unifiedpost"
    pub base_url: Option<String>,
    pub client_id: Option<String>,
    pub token_url: Option<String>,
}

fn default_provider_kind() -> String {
    "mock".to_string()
}

pub fn load() -> Result<AppConfig> {
    let cfg: AppConfig = confy::load(APP_NAME, None).context("Failed to load app config")?;
    Ok(cfg)
}

pub fn store(cfg: &AppConfig) -> Result<()> {
    confy::store(APP_NAME, None, cfg).context("Failed to store app config")?;
    Ok(())
}

/// Store a secret in the OS keychain
pub fn store_secret(key: &str, value: &str) -> Result<()> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, key)?;
    entry.set_password(value)?;
    Ok(())
}

/// Retrieve a secret from the OS keychain
pub fn get_secret(key: &str) -> Result<String> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, key)?;
    let password = entry.get_password()?;
    Ok(password)
}

/// Delete a secret from the OS keychain
pub fn delete_secret(key: &str) -> Result<()> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, key)?;
    entry.delete_password()?;
    Ok(())
}
