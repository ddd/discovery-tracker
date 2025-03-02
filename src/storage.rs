use std::collections::HashMap;
use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use tokio::fs::{self, File};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::parser::DiscoveryDocument;

#[derive(Clone)]
pub struct Storage {
    base_path: PathBuf,
}

impl Storage {
    pub async fn new<P: AsRef<Path>>(base_path: P) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        fs::create_dir_all(&base_path).await.context("Failed to create storage directory")?;
        Ok(Storage { base_path })
    }

    pub async fn store(&self, service: &str, document: &DiscoveryDocument) -> Result<()> {
        let path = self.get_path(service);
        let json = serde_json::to_string(document).context("Failed to serialize document")?;
        let mut file = File::create(path).await.context("Failed to create file for storing document")?;
        file.write_all(json.as_bytes()).await.context("Failed to write document to file")
    }

    pub async fn retrieve(&self, service: &str) -> Result<Option<DiscoveryDocument>> {
        let path = self.get_path(service);
        if fs::try_exists(&path).await? {
            let mut file = File::open(path).await.context("Failed to open file for retrieving document")?;
            let mut contents = String::new();
            file.read_to_string(&mut contents).await.context("Failed to read document from file")?;
            let document = serde_json::from_str(&contents).context("Failed to deserialize document")?;
            Ok(Some(document))
        } else {
            Ok(None)
        }
    }

    pub async fn retrieve_all(&self) -> Result<HashMap<String, DiscoveryDocument>> {
        let mut documents = HashMap::new();
        let mut read_dir = fs::read_dir(&self.base_path).await.context("Failed to read storage directory")?;
        
        while let Some(entry) = read_dir.next_entry().await.context("Failed to read directory entry")? {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                if let Some(stem) = path.file_stem() {
                    if let Some(service) = stem.to_str() {
                        if let Some(doc) = self.retrieve(service).await? {
                            documents.insert(service.to_string(), doc);
                        }
                    }
                }
            }
        }
        
        Ok(documents)
    }

    fn get_path(&self, service: &str) -> PathBuf {
        self.base_path.join(format!("{}.json", service))
    }
}