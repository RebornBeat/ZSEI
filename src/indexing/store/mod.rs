//! Storage for ZSEI
//!
//! This module provides storage functionality for the
//! Zero-Shot Bolted Embedding Indexer.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use crate::errors::{Result, ZseiError};
use crate::indexing::FileMetadata;

/// Index store trait
pub trait IndexStore {
    /// Save store to disk
    fn save(&self, path: &Path) -> Result<()>;

    /// Load store from disk
    fn load(&mut self, path: &Path) -> Result<()>;
}

/// File metadata store trait
pub trait FileMetadataStore {
    /// Add file metadata
    fn add_file_metadata(&mut self, metadata: FileMetadata) -> Result<()>;

    /// Get file metadata by path
    fn get_file_metadata(&self, path: &Path) -> Option<&FileMetadata>;

    /// Get file metadata by embedding ID
    fn get_file_metadata_by_embedding_id(&self, embedding_id: &str) -> Option<&FileMetadata>;

    /// Remove file metadata
    fn remove_file_metadata(&mut self, path: &Path) -> Result<()>;
}

/// Metadata store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataStore {
    /// File metadata indexed by path
    file_metadata: HashMap<PathBuf, FileMetadata>,

    /// File metadata indexed by embedding ID
    file_metadata_by_embedding_id: HashMap<String, PathBuf>,
}

impl MetadataStore {
    /// Create a new metadata store
    pub fn new() -> Self {
        Self {
            file_metadata: HashMap::new(),
            file_metadata_by_embedding_id: HashMap::new(),
        }
    }

    /// Get all file metadata
    pub fn get_all_file_metadata(&self) -> &HashMap<PathBuf, FileMetadata> {
        &self.file_metadata
    }
}

impl FileMetadataStore for MetadataStore {
    fn add_file_metadata(&mut self, metadata: FileMetadata) -> Result<()> {
        // Add to both indexes
        let path = metadata.path.clone();
        let embedding_id = metadata.embedding_id.clone();

        self.file_metadata.insert(path.clone(), metadata);
        self.file_metadata_by_embedding_id
            .insert(embedding_id, path);

        Ok(())
    }

    fn get_file_metadata(&self, path: &Path) -> Option<&FileMetadata> {
        self.file_metadata.get(path)
    }

    fn get_file_metadata_by_embedding_id(&self, embedding_id: &str) -> Option<&FileMetadata> {
        self.file_metadata_by_embedding_id
            .get(embedding_id)
            .and_then(|path| self.file_metadata.get(path))
    }

    fn remove_file_metadata(&mut self, path: &Path) -> Result<()> {
        // Remove from both indexes
        if let Some(metadata) = self.file_metadata.remove(path) {
            self.file_metadata_by_embedding_id
                .remove(&metadata.embedding_id);
        }

        Ok(())
    }
}

impl IndexStore for MetadataStore {
    fn save(&self, path: &Path) -> Result<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| ZseiError::Indexing(format!("Failed to create directory: {}", e)))?;
        }

        // Serialize to JSON
        let file = File::create(path)
            .map_err(|e| ZseiError::Indexing(format!("Failed to create file: {}", e)))?;

        let writer = BufWriter::new(file);

        serde_json::to_writer_pretty(writer, self).map_err(|e| {
            ZseiError::Indexing(format!("Failed to serialize metadata store: {}", e))
        })?;

        Ok(())
    }

    fn load(&mut self, path: &Path) -> Result<()> {
        // Deserialize from JSON
        let file = File::open(path)
            .map_err(|e| ZseiError::Indexing(format!("Failed to open file: {}", e)))?;

        let reader = BufReader::new(file);

        let store: MetadataStore = serde_json::from_reader(reader).map_err(|e| {
            ZseiError::Indexing(format!("Failed to deserialize metadata store: {}", e))
        })?;

        self.file_metadata = store.file_metadata;
        self.file_metadata_by_embedding_id = store.file_metadata_by_embedding_id;

        Ok(())
    }
}
