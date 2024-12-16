use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiscoveryDocument {
    pub description: Option<String>,
    pub title: Option<String>,
    #[serde(rename = "discoveryVersion")]
    pub discovery_version: Option<String>,
    pub revision: Option<String>,
    #[serde(rename = "ownerDomain")]
    pub owner_domain: Option<String>,
    #[serde(rename = "baseUrl")]
    pub base_url: Option<String>,
    pub schemas: Option<HashMap<String, Schema>>,
    #[serde(rename = "documentationLink")]
    pub documentation_link: Option<String>,
    pub resources: Option<HashMap<String, Resource>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Schema {
    Object(ObjectSchema),
    Enum(EnumSchema),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ObjectSchema {
    pub properties: Option<HashMap<String, Property>>,
    #[serde(rename = "type")]
    pub schema_type: Option<String>,
    pub id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnumSchema {
    pub properties: Option<HashMap<String, Property>>,
    #[serde(rename = "type")]
    pub schema_type: Option<String>,
    pub id: Option<String>,
    pub enumeration: Vec<String>,
    #[serde(rename = "enumDescriptions")]
    pub enum_descriptions: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Property {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub property_type: Option<String>,
    #[serde(rename = "$ref", skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Resource {
    pub methods: Option<HashMap<String, Method>>,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Method {
    pub id: String,
    pub path: String,
    #[serde(rename = "httpMethod")]
    pub http_method: String,
    pub description: Option<String>,
    pub parameters: Option<HashMap<String, Parameter>>,
    pub request: Option<Request>,
    pub response: Option<Response>,
    pub scopes: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Request {
    #[serde(rename = "$ref")]
    pub reference: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Response {
    #[serde(rename = "$ref")]
    pub reference: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Parameter {
    #[serde(rename = "type")]
    pub param_type: Option<String>,
    pub description: Option<String>,
    pub required: Option<bool>,
    pub location: Option<String>,
}

pub fn parse_document(content: &str) -> Result<DiscoveryDocument> {
    let document: DiscoveryDocument = serde_json::from_str(content)
        .context("Failed to parse discovery document")?;
    Ok(document)
}

pub fn parse_all_documents(documents: Vec<(String, String)>) -> Result<HashMap<String, DiscoveryDocument>> {
    let mut parsed_documents = HashMap::new();
    for (service, content) in documents {
        let document = parse_document(&content)
            .with_context(|| format!("Failed to parse document for service: {}", service))?;
        parsed_documents.insert(service, document);
    }
    Ok(parsed_documents)
}