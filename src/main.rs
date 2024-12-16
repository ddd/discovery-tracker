use anyhow::{Result, Context};
use tracing::{info, error, warn};
use std::time::Duration;
use tokio::time;
use std::net::SocketAddr;

mod api;
mod config;
mod fetcher;
mod parser;
mod diff_engine;
mod storage;
mod change_logger;
mod webhook;

use crate::config::Config;
use crate::fetcher::Fetcher;
use crate::diff_engine::DiffEngine;
use crate::storage::Storage;
use crate::change_logger::ChangeLogger;
use crate::webhook::DiscordNotifier;

#[tokio::main]
async fn main() -> Result<()> {
    // Create logs directory if it doesn't exist
    std::fs::create_dir_all("logs").context("Failed to create logs directory")?;

    let file_appender = tracing_appender::rolling::daily("logs", "discovery.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .json()
        .init();

    info!("Starting Google Discovery Document Tracker");

    // Load configuration
    let config = Config::load().context("Failed to load configuration")?;

    // Initialize components
    let fetcher = Fetcher::new(config.clone())?;
    let diff_engine = DiffEngine::new();
    let storage = Storage::new(&config.storage_path)?;
    let change_logger = ChangeLogger::new(&config.log_path)?;

    let discord_notifier = if config.enable_discord_webhooks {
        if let Some(discord_config) = config.discord_webhook_config.clone() {
            Some(DiscordNotifier::new(
                discord_config,
            ))
        } else {
            None
        }
    } else {
        None
    };

    // Initialize API
    let api = crate::api::Api::new(storage.clone(), change_logger.clone());
    let api_addr = SocketAddr::from(([0, 0, 0, 0], 3000));

    // Start API server
    tokio::spawn(async move {
        api.run(api_addr).await;
    });

    // Main loop
    loop {
        info!("Starting discovery document check");

        // Fetch and parse documents
        let parsed_documents = match fetcher.fetch_all().await {
            Ok(results) => parser::parse_all_documents(results)?,
            Err(e) => {
                error!("Error occurred while fetching documents: {}", e);
                continue;
            }
        };

        // Retrieve stored documents
        let stored_documents = storage.retrieve_all()?;

        for (service, new_doc) in &parsed_documents {
            if let Some(old_doc) = stored_documents.get(service) {
                let changes = diff_engine.diff(old_doc, new_doc, service);
                if !changes.modifications.is_empty() || !changes.additions.is_empty() || !changes.deletions.is_empty() {
                    info!("Changes detected for service: {}", service);
                    let logged_change = change_logger.log_changes(changes, &old_doc, &new_doc)?;

                    if let Some(notifier) = &discord_notifier {
                        if let Err(e) = notifier.notify(&logged_change).await {
                            error!("Failed to send Discord notification: {}", e);
                        }
                    }
                } else {
                    info!("No changes detected for service: {}", service);
                }
            } else {
                info!("New service discovered: {}", service);
                // For new services, we just store the document without diffing
            }

            // Store the new document version
            storage.store(service, new_doc)?;
        }

        // Check for removed services
        for service in stored_documents.keys() {
            if !parsed_documents.contains_key(service) {
                warn!("Service no longer available: {}", service);
                // You might want to implement a method to mark services as inactive or remove them
                // storage.mark_inactive(service)?;
            }
        }

        info!("Completed discovery document check");

        // Wait for the next check interval
        time::sleep(Duration::from_secs(config.check_interval)).await;
    }
}
