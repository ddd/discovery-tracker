use serde::Serialize;
use reqwest::Client;
use anyhow::{Result, Context};
use crate::change_logger::{LoggedChange, ChangeSummary};
use crate::config::DiscordWebhookConfig;

#[derive(Serialize)]
struct DiscordWebhook {
    content: Option<String>,
    embeds: Vec<DiscordEmbed>,
}

#[derive(Serialize)]
struct DiscordEmbed {
    description: String,
    color: u32,
    author: DiscordEmbedAuthor,
    footer: DiscordEmbedFooter,
}

#[derive(Serialize)]
struct DiscordEmbedAuthor {
    name: String,
    url: String,
}

#[derive(Serialize)]
struct DiscordEmbedFooter {
    text: String,
}

pub struct DiscordNotifier {
    client: Client,
    config: DiscordWebhookConfig,
}

impl DiscordNotifier {
    pub fn new(config: DiscordWebhookConfig) -> Self {
        DiscordNotifier {
            client: Client::new(),
            config,
        }
    }

    pub async fn notify(&self, change: &LoggedChange) -> Result<()> {
        // Find the service configuration
        let service_config = self.config.services
            .iter()
            .find(|s| s.service == change.service)
            .context("Service not found in Discord webhook configuration")?;

        // Build mention string if tags match configured roles
        let mentions = self.build_mentions(&change.summary.tags);
        
        // Build the embed description
        let description = self.build_description(&change.summary);

        // Create the webhook payload
        let webhook = DiscordWebhook {
            content: if mentions.is_empty() { None } else { Some(mentions) },
            embeds: vec![DiscordEmbed {
                description,
                color: 5814783, // Blue color
                author: DiscordEmbedAuthor {
                    name: service_config.name.clone(),
                    url: format!("{}/api/changes/{}/{}/diff", 
                        self.config.tracker_api_url, 
                        change.service, 
                        change.timestamp
                    ),
                },
                footer: DiscordEmbedFooter {
                    text: format!("Change ID: {}", change.timestamp),
                },
            }],
        };

        // Send the webhook
        self.client.post(&service_config.webhook_url)
            .json(&webhook)
            .send()
            .await
            .context("Failed to send Discord webhook")?;

        Ok(())
    }

    fn build_mentions(&self, tags: &[String]) -> String {
        let mentions: Vec<String> = self.config.tag_mention_role_ids
            .iter()
            .filter(|tm| tags.contains(&tm.tag))
            .map(|tm| format!("<@&{}>", tm.role_id))
            .collect();

        mentions.join(" ")
    }

    fn build_description(&self, summary: &ChangeSummary) -> String {
        let mut parts = Vec::new();
        
        if summary.additions > 0 {
            parts.push(format!("**+{}** additions", summary.additions));
        }
        if summary.modifications > 0 {
            parts.push(format!("**~{}** changes", summary.modifications));
        }
        if summary.deletions > 0 {
            parts.push(format!("**-{}** removed", summary.deletions));
        }

        parts.join("\n")
    }
}