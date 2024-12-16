use std::fs::{File, OpenOptions, create_dir_all};
use std::path::{Path, PathBuf};
use std::io::Write;
use chrono::Utc;
use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use crate::diff_engine::{Change, ChangeSet};
use crate::parser::DiscoveryDocument;

#[derive(Debug, Serialize, Deserialize)]
pub struct LoggedChange {
    pub revision: String,
    pub timestamp: u64,  // Unix timestamp
    pub service: String,
    pub summary: ChangeSummary,
    pub modifications: Vec<Change>,
    pub additions: Vec<Change>,
    pub deletions: Vec<Change>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChangeSummary {
    pub additions: usize,
    pub modifications: usize,
    pub deletions: usize,
    pub tags: Vec<String>,
}

#[derive(Clone)]
pub struct ChangeLogger {
    base_path: PathBuf,
}

impl ChangeLogger {
    pub fn new<P: AsRef<Path>>(base_path: P) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        create_dir_all(&base_path).context("Failed to create change log directory")?;
        Ok(ChangeLogger { base_path })
    }

    pub fn log_changes(&self, change_set: ChangeSet, _before: &DiscoveryDocument, after: &DiscoveryDocument) -> Result<LoggedChange> {
        let mut tags = Vec::new();
        if self.has_new_method(&change_set) {
            tags.push("new_method".to_string());
        }
        if self.has_removed_method(&change_set) {
            tags.push("removed_method".to_string());
        }

        let summary = ChangeSummary {
            additions: change_set.additions.len(),
            modifications: change_set.modifications.len(),
            deletions: change_set.deletions.len(),
            tags,
        };

        let logged_change = LoggedChange {
            revision: after.revision.clone().unwrap_or_else(|| "unknown".to_string()),
            timestamp: Utc::now().timestamp() as u64,
            service: change_set.service.clone(),
            summary,
            modifications: change_set.modifications,
            additions: change_set.additions,
            deletions: change_set.deletions,
        };

        let file_name = format!("{}-{}.json", logged_change.service, logged_change.timestamp);
        let file_path = self.base_path.join(file_name);

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(file_path)
            .context("Failed to create change log file")?;

        let json = serde_json::to_string_pretty(&logged_change)
            .context("Failed to serialize logged change")?;

        file.write_all(json.as_bytes())
            .context("Failed to write change log")?;

        Ok(logged_change)
    }

    pub fn get_all_changes(&self, offset: usize, limit: usize) -> Result<Vec<LoggedChange>> {
        let mut changes = Vec::new();
        for entry in std::fs::read_dir(&self.base_path).context("Failed to read change log directory")? {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                let file = File::open(path).context("Failed to open change log file")?;
                let logged_change: LoggedChange = serde_json::from_reader(file)
                    .context("Failed to deserialize logged change")?;
                changes.push(logged_change);
            }
        }
        changes.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(changes.into_iter().skip(offset).take(limit).collect())
    }

    pub fn get_changes_for_service(&self, service: &str, offset: usize, limit: usize) -> Result<Vec<LoggedChange>> {
        let mut changes = Vec::new();
        for entry in std::fs::read_dir(&self.base_path).context("Failed to read change log directory")? {
            let entry = entry.context("Failed to read directory entry")?;
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                if let Some(file_name) = path.file_stem() {
                    if file_name.to_str().unwrap().starts_with(service) {
                        let file = File::open(path).context("Failed to open change log file")?;
                        let logged_change: LoggedChange = serde_json::from_reader(file)
                            .context("Failed to deserialize logged change")?;
                        changes.push(logged_change);
                    }
                }
            }
        }
        changes.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(changes.into_iter().skip(offset).take(limit).collect())
    }

    pub fn get_specific_change(&self, service: &str, timestamp: &str) -> Result<LoggedChange> {
        let file_name = format!("{}-{}.json", service, timestamp);
        let file_path = self.base_path.join(file_name);

        let file = File::open(file_path).context("Failed to open change log file")?;
        let logged_change: LoggedChange = serde_json::from_reader(file)
            .context("Failed to deserialize logged change")?;

        Ok(logged_change)
    }

    fn has_new_method(&self, change_set: &ChangeSet) -> bool {
        change_set.additions.iter().any(|change| {
            let path_segments: Vec<&str> = change.path.split('/').collect();
            path_segments.len() >= 4 
                && path_segments[path_segments.len() - 2] == "methods"
                && change.value.is_some()
                && change.old_value.is_none()
        })
    }

    fn has_removed_method(&self, change_set: &ChangeSet) -> bool {
        change_set.deletions.iter().any(|change| {
            let path_segments: Vec<&str> = change.path.split('/').collect();
            path_segments.len() >= 4 
                && path_segments[path_segments.len() - 2] == "methods"
                && change.value.is_none()
                && change.old_value.is_some()
        })
    }
}