use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub github_token: Option<String>,
    pub ghcr_registry: String,
    pub nju_registry: String,
}

impl Config {
    pub async fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        
        if !config_path.exists() {
            return Ok(Self::default());
        }
        
        let content = fs::read_to_string(&config_path).await?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }
    
    pub async fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&config_path, content).await?;
        Ok(())
    }
    
    fn config_path() -> Result<PathBuf> {
        let mut path = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
        path.push("docker-sync-cli");
        path.push("config.json");
        Ok(path)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            github_token: None,
            ghcr_registry: "ghcr.io".to_string(),
            nju_registry: "ghcr.nju.edu.cn".to_string(),
        }
    }
}