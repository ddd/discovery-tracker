use std::collections::HashMap;
use std::fs::{File, create_dir_all};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use crate::parser::DiscoveryDocument;

#[derive(Clone)]

pub struct Storage {
    base_path: PathBuf,
}

impl Storage {
    pub fn new<P: AsRef<Path>>(base_path: P) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        create_dir_all(&base_path).context("Failed to create storage directory")?;
        Ok(Storage { base_path })
    }

    pub fn store(&self, service: &str, document: &DiscoveryDocument) -> Result<()> {
        let path = self.get_path(service);
        let mut file = File::create(path).context("Failed to create file for storing document")?;
        let json = serde_json::to_string(document).context("Failed to serialize document")?;
        file.write_all(json.as_bytes()).context("Failed to write document to file")
    }

    pub fn retrieve(&self, service: &str) -> Result<Option<DiscoveryDocument>> {
        let path = self.get_path(service);
        if path.exists() {
            let mut file = File::open(path).context("Failed to open file for retrieving document")?;
            let mut contents = String::new();
            file.read_to_string(&mut contents).context("Failed to read document from file")?;
            let document = serde_json::from_str(&contents).context("Failed to deserialize document")?;
            Ok(Some(document))
        } else {
            Ok(None)
        }
    }

    pub fn retrieve_all(&self) -> Result<HashMap<String, DiscoveryDocument>> {
        let mut documents = HashMap::new();
        for entry in std::fs::read_dir(&self.base_path).context("Failed to read storage directory")? {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                let service = path.file_stem().unwrap().to_str().unwrap().to_string();
                if let Some(doc) = self.retrieve(&service)? {
                    documents.insert(service, doc);
                }
            }
        }
        Ok(documents)
    }

    fn get_path(&self, service: &str) -> PathBuf {
        self.base_path.join(format!("{}.json", service))
    }
}