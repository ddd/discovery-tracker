use anyhow::{Result, Context};
use reqwest::Client;
use crate::config::{Config, ServiceConfig};

pub struct Fetcher {
    client: Client,
    config: Config,
}

impl Fetcher {
    pub fn new(config: Config) -> Result<Self> {
        let client = Client::new();
        Ok(Fetcher { client, config })
    }

    pub async fn fetch_all(&self) -> Result<Vec<(String, String)>> {
        let mut results = Vec::new();
        for service in &self.config.services {
            let content = self.fetch_document(service).await
                .with_context(|| format!("Failed to fetch document for service: {}", service.service))?;
            results.push((service.service.clone(), content));
        }
        Ok(results)
    }

    async fn fetch_document(&self, service: &ServiceConfig) -> Result<String> {
        let url = self.build_url(service);
        let mut request = if service.features.contains(&"post_to_get_override".to_string()) {
            self.client.post(&url)
        } else {
            self.client.get(&url)
        };

        if let Some(key) = &service.key {
            request = request.header("x-goog-api-key", key);
        }

        if service.features.contains(&"post_to_get_override".to_string()) {
            request = request.header("x-http-method-override", "GET");
            request = request.header("content-length", "0");
        }

        let response = request.send().await?;
        let content = response.text().await?;
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