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
    title: Option<String>,
    description: String,
    color: u32,
    author: DiscordEmbedAuthor,
    footer: Option<DiscordEmbedFooter>,
}

#[derive(Serialize)]
struct DiscordEmbedAuthor {
    name: String,
    url: Option<String>,
}

#[derive(Serialize)]
struct DiscordEmbedFooter {
    text: String,
}

pub struct DiscordNotifier {
    client: Client,
    pub config: DiscordWebhookConfig,
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
                title: None,
                description,
                color: 5814783, // Blue color
                author: DiscordEmbedAuthor {
                    name: service_config.name.clone(),
                    url: Some(format!("{}/api/changes/{}/{}/diff", 
                        self.config.tracker_api_url, 
                        change.service, 
                        change.timestamp
                    )),
                },
                footer: Some(DiscordEmbedFooter {
                    text: format!("Change ID: {}", change.timestamp),
                }),
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

    pub async fn notify_error(&self, service_name: &str, error_message: &str) -> Result<()> {
        // Build error mention if configured
        let error_mention = match &self.config.error_mention_role_id {
            Some(role_id) => Some(format!("<@&{}>", role_id)),
            None => None,
        };

        // Check if we have a dedicated error webhook URL
        if let Some(error_webhook_url) = &self.config.error_webhook_url {
            // Create a generic error webhook with all services in one place
            let webhook = DiscordWebhook {
                content: error_mention,
                embeds: vec![DiscordEmbed {
                    title: Some(format!("Error: {}", service_name)),
                    description: format!("```\n{}\n```", error_message),
                    color: 16711680, // Red color
                    author: DiscordEmbedAuthor {
                        name: "Discovery Document Tracker".to_string(),
                        url: None,
                    },
                    footer: None,
                }],
            };

            // Send to the error webhook URL
            self.client.post(error_webhook_url)
                .json(&webhook)
                .send()
                .await
                .context("Failed to send error Discord webhook")?;

            return Ok(());
        }

        // If no dedicated error webhook, fall back to service-specific webhook
        let service_config = self.config.services
            .iter()
            .find(|s| s.service == service_name)
            .context("Service not found in Discord webhook configuration")?;

        // Create the webhook payload
        let webhook = DiscordWebhook {
            content: error_mention,
            embeds: vec![DiscordEmbed {
                title: Some("Service Error".to_string()),
                description: format!("```\n{}\n```", error_message),
                color: 16711680, // Red color
                author: DiscordEmbedAuthor {
                    name: service_config.name.clone(),
                    url: None,
                },
                footer: None,
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