//! Configuration management for ZSEI
//!
//! This module provides configuration management for the Zero-Shot
//! Bolted Embedding Indexer.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::errors::{Result, ZseiError};
use crate::llm::LlmConfig;

/// Configuration for embedding generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Embedding dimension
    pub dimension: usize,

    /// Chunk size for text chunking
    pub chunk_size: usize,

    /// Chunk overlap for text chunking
    pub chunk_overlap: usize,

    /// Embedding cache size
    pub cache_size: usize,

    /// Whether to use GPU for embedding generation
    pub use_gpu: bool,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            dimension: 384,
            chunk_size: 1024,
            chunk_overlap: 128,
            cache_size: 10000,
            use_gpu: false,
        }
    }
}

/// Configuration for the indexing system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingConfig {
    /// Vector storage type
    pub vector_store_type: VectorStoreType,

    /// Whether to store metadata
    pub store_metadata: bool,

    /// Whether to store content
    pub store_content: bool,

    /// Maximum number of files to index
    pub max_files: Option<usize>,

    /// File extensions to include
    pub include_extensions: Vec<String>,

    /// File patterns to exclude
    pub exclude_patterns: Vec<String>,
}

impl Default for IndexingConfig {
    fn default() -> Self {
        Self {
            vector_store_type: VectorStoreType::Hnsw,
            store_metadata: true,
            store_content: true,
            max_files: None,
            include_extensions: vec![
                "rs".to_string(),
                "py".to_string(),
                "js".to_string(),
                "ts".to_string(),
                "go".to_string(),
                "c".to_string(),
                "cpp".to_string(),
                "h".to_string(),
                "hpp".to_string(),
                "java".to_string(),
                "kt".to_string(),
                "cs".to_string(),
            ],
            exclude_patterns: vec![
                "**/target/**".to_string(),
                "**/node_modules/**".to_string(),
                "**/.git/**".to_string(),
                "**/venv/**".to_string(),
                "**/__pycache__/**".to_string(),
                "**/dist/**".to_string(),
                "**/build/**".to_string(),
            ],
        }
    }
}

/// Vector store types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VectorStoreType {
    /// Flat vector store
    Flat,

    /// Hierarchical Navigable Small World vector store
    Hnsw,

    /// Faiss vector store
    Faiss,
}

/// Configuration for the refactoring system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactorConfig {
    /// Number of branches to create
    pub num_branches: usize,

    /// Number of iterations to keep
    pub keep_iterations: usize,

    /// Whether to automatically apply changes
    pub auto_apply: bool,

    /// Maximum number of files to modify
    pub max_modified_files: usize,
}

impl Default for RefactorConfig {
    fn default() -> Self {
        Self {
            num_branches: 5,
            keep_iterations: 3,
            auto_apply: false,
            max_modified_files: 50,
        }
    }
}

/// Main configuration for ZSEI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Project root directory
    project_root: PathBuf,

    /// LLM configuration (default)
    pub llm: LlmConfig,

    /// Phase 1 (Prompt Analysis) LLM configuration (optional)
    pub phase1_llm: Option<LlmConfig>,

    /// Phase 2 (Code Refactoring) LLM configuration (optional)
    pub phase2_llm: Option<LlmConfig>,

    /// Embedding configuration
    pub embedding: EmbeddingConfig,

    /// Indexing configuration
    pub indexing: IndexingConfig,

    /// Refactor configuration
    pub refactor: RefactorConfig,

    /// Additional project paths to index
    pub additional_project_paths: Vec<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            project_root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            llm: LlmConfig::default(),
            phase1_llm: None,
            phase2_llm: None,
            embedding: EmbeddingConfig::default(),
            indexing: IndexingConfig::default(),
            refactor: RefactorConfig::default(),
            additional_project_paths: Vec::new(),
        }
    }
}

impl Config {
    /// Load configuration from a file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .map_err(|e| ZseiError::Config(format!("Failed to read config file: {}", e)))?;

        let mut config: Config = toml::from_str(&content)
            .map_err(|e| ZseiError::Config(format!("Failed to parse config file: {}", e)))?;

        // Ensure project root is set
        if config.project_root == PathBuf::from("") {
            config.project_root = std::env::current_dir().map_err(|e| {
                ZseiError::Config(format!("Failed to get current directory: {}", e))
            })?;
        }

        Ok(config)
    }

    /// Save configuration to a file
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| ZseiError::Config(format!("Failed to serialize config: {}", e)))?;

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| ZseiError::Config(format!("Failed to create directories: {}", e)))?;
        }

        fs::write(path, content)
            .map_err(|e| ZseiError::Config(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }

    /// Get the project root directory
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// Set the project root directory
    pub fn set_project_root(&mut self, path: PathBuf) {
        self.project_root = path;
    }

    /// Get the index path
    pub fn index_path(&self) -> Result<PathBuf> {
        Ok(self.project_root.join(".zsei").join("index"))
    }

    /// Get the metadata path
    pub fn metadata_path(&self) -> Result<PathBuf> {
        Ok(self.project_root.join(".zsei").join("metadata"))
    }

    /// Get the branches path
    pub fn branches_path(&self) -> Result<PathBuf> {
        Ok(self.project_root.join(".zsei").join("branches"))
    }

    /// Add an additional project path
    pub fn add_project_path(&mut self, path: PathBuf) {
        self.additional_project_paths.push(path);
    }

    /// Get Phase 1 LLM configuration
    pub fn get_phase1_llm_config(&self) -> &LlmConfig {
        self.phase1_llm.as_ref().unwrap_or(&self.llm)
    }

    /// Get Phase 2 LLM configuration
    pub fn get_phase2_llm_config(&self) -> &LlmConfig {
        self.phase2_llm.as_ref().unwrap_or(&self.llm)
    }
}
