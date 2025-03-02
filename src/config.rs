use serde::Deserialize;
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use anyhow::{Result, Context};

#[derive(Clone, Deserialize)]
pub struct Config {
    pub storage_path: PathBuf,
    pub log_path: PathBuf,
    pub check_interval: u64,
    pub services: Vec<ServiceConfig>,
    #[serde(default)]
    pub enable_discord_webhooks: bool,
    pub discord_webhook_config: Option<DiscordWebhookConfig>,
}

#[derive(Clone, Deserialize)]
pub struct ServiceConfig {
    pub service: String,
    pub key: Option<String>,
    pub spatula: Option<String>,
    pub visibility_label: Option<String>,
    #[serde(default = "default_format")]
    pub format: String,
}

#[derive(Clone, Deserialize)]
pub struct DiscordWebhookConfig {
    pub tracker_api_url: String,
    pub tag_mention_role_ids: Vec<TagMentionRoleId>,
    pub services: Vec<ServiceWebhook>,
    pub error_webhook_url: Option<String>,
    pub error_mention_role_id: Option<String>,
}

#[derive(Clone, Deserialize)]
pub struct TagMentionRoleId {
    pub tag: String,
    pub role_id: String,
}

#[derive(Clone, Deserialize)]
pub struct ServiceWebhook {
    pub service: String,
    pub name: String,
    pub webhook_url: String,
}

fn default_format() -> String {
    "rest".to_string()
}

impl Config {
    pub async fn load() -> Result<Self> {
        let mut file = File::open("config.yaml")
            .await
            .context("Failed to open config.yaml")?;
        
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .await
            .context("Failed to read config.yaml")?;

        let config: Config = serde_yaml::from_str(&contents)
            .context("Failed to parse config.yaml")?;

        Ok(config)
    }
}