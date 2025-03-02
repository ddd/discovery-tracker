use anyhow::{Result, Context, anyhow};
use reqwest::Client;
use tracing::warn;
use crate::config::{Config, ServiceConfig};

pub struct Fetcher {
    client: Client,
    config: Config,
}

#[derive(Debug)]
pub struct FetchResult {
    pub service: String,
    pub content: Option<String>,
    pub error: Option<String>,
}

impl Fetcher {
    pub fn new(config: Config) -> Result<Self> {
        let client = Client::new();
        Ok(Fetcher { client, config })
    }

    pub async fn fetch_all(&self) -> Result<Vec<FetchResult>> {
        let mut results = Vec::new();
        for service in &self.config.services {
            match self.fetch_document(service).await {
                Ok(content) => {
                    results.push(FetchResult {
                        service: service.service.clone(),
                        content: Some(content),
                        error: None,
                    });
                }
                Err(e) => {
                    let error_msg = format!("Failed to fetch document for service {}: {}", service.service, e);
                    warn!("{}", error_msg);
                    results.push(FetchResult {
                        service: service.service.clone(),
                        content: None,
                        error: Some(error_msg),
                    });
                }
            }
        }
        Ok(results)
    }

    async fn fetch_document(&self, service: &ServiceConfig) -> Result<String> {
        let url = self.build_url(service);
        let mut request = self.client.get(&url);
 
        if let Some(key) = &service.key {
            request = request.header("x-goog-api-key", key);
        }

        if let Some(spatula) = &service.spatula {
            request = request.header("x-goog-spatula", spatula);
        }

        let response = request.send().await
            .with_context(|| format!("HTTP request failed for service: {}", service.service))?;
            
        if !response.status().is_success() {
            return Err(anyhow!("Received non-success status code: {} for service: {}", 
                response.status(), service.service));
        }
        
        let content = response.text().await
            .with_context(|| format!("Failed to read response body for service: {}", service.service))?;
            
        // Basic validation that it's a valid discovery document
        if !content.contains("\"discoveryVersion\"") {
            return Err(anyhow!("Response doesn't appear to be a valid discovery document for service: {}", 
                service.service));
        }
        
        Ok(content)
    }

    fn build_url(&self, service: &ServiceConfig) -> String {
        let mut url = format!("https://{}/$discovery/{}", service.service, service.format);
        if let Some(label) = &service.visibility_label {
            url.push_str(&format!("?label={}", label));
        }
        url
    }
}