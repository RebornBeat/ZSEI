//! Indexing and storage for ZSEI
//!
//! This module provides indexing and storage functionality for the
//! Zero-Shot Bolted Embedding Indexer.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info};

pub mod store;
pub mod vector;

use crate::analyzers::{Analyzer, ProgressUpdate};
use crate::core::config::Config;
use crate::core::project::Project;
use crate::embedding::{Embedding, EmbeddingFactory, EmbeddingGenerator, EmbeddingType};
use crate::errors::{Result, ZseiError};
use store::{FileMetadataStore, IndexStore, MetadataStore};
use vector::{VectorSearchParams, VectorStore};

/// Indexer struct
pub struct Indexer {
    /// Configuration
    config: Arc<Config>,

    /// Analyzer
    analyzer: Arc<Analyzer>,

    /// Vector store
    vector_store: Arc<RwLock<VectorStore>>,

    /// Metadata store
    metadata_store: Arc<RwLock<MetadataStore>>,

    /// Embedding factory
    embedding_factory: EmbeddingFactory,
}

impl Indexer {
    /// Create a new indexer
    pub fn new(
        config: Arc<Config>,
        analyzer: Arc<Analyzer>,
        llm: Arc<dyn crate::llm::Model>,
    ) -> Self {
        let vector_store = Arc::new(RwLock::new(VectorStore::new()));
        let metadata_store = Arc::new(RwLock::new(MetadataStore::new()));
        let embedding_factory = EmbeddingFactory::new(llm, config.clone());

        Self {
            config,
            analyzer,
            vector_store,
            metadata_store,
            embedding_factory,
        }
    }

    /// Index files (full indexing)
    pub async fn index_full(
        &self,
        paths: &[PathBuf],
        progress_tx: Option<mpsc::Sender<ProgressUpdate>>,
    ) -> Result<()> {
        info!("Starting full indexing");

        // Analyze files
        let analysis_result = self
            .analyzer
            .analyze_full(paths, progress_tx.clone())
            .await?;

        // Index files
        self.index_files(&analysis_result.file_analyses, progress_tx)
            .await?;

        Ok(())
    }

    /// Index files (incremental indexing)
    pub async fn index_incremental(
        &self,
        paths: &[PathBuf],
        progress_tx: Option<mpsc::Sender<ProgressUpdate>>,
    ) -> Result<()> {
        info!("Starting incremental indexing");

        // Analyze files
        let analysis_result = self
            .analyzer
            .analyze_incremental(paths, progress_tx.clone())
            .await?;

        // Index files
        self.index_files(&analysis_result.file_analyses, progress_tx)
            .await?;

        Ok(())
    }

    /// Index analyzed files
    async fn index_files(
        &self,
        file_analyses: &[crate::analyzers::common::FileAnalysis],
        progress_tx: Option<mpsc::Sender<ProgressUpdate>>,
    ) -> Result<()> {
        let total_files = file_analyses.len();

        // Create code embedding generator
        let code_generator = self.embedding_factory.create_code_generator();

        // Process each file
        for (i, analysis) in file_analyses.iter().enumerate() {
            // Send progress update
            if let Some(tx) = &progress_tx {
                let update = ProgressUpdate {
                    current: i + 1,
                    total: total_files,
                    current_item: analysis.path.display().to_string(),
                    message: format!("Indexing {}", analysis.path.display()),
                };

                if let Err(e) = tx.send(update).await {
                    debug!("Failed to send progress update: {}", e);
                }
            }

            // Generate embedding
            let embedding = code_generator.generate(analysis).await?;

            // Store embedding
            self.add_embedding(&embedding).await?;

            // Store metadata
            self.add_metadata(analysis, &embedding).await?;
        }

        Ok(())
    }

    /// Add embedding to vector store
    async fn add_embedding(&self, embedding: &Embedding) -> Result<()> {
        let mut vector_store = self.vector_store.write().await;
        vector_store.add_embedding(embedding.clone())?;
        Ok(())
    }

    /// Add metadata to metadata store
    async fn add_metadata(
        &self,
        analysis: &crate::analyzers::common::FileAnalysis,
        embedding: &Embedding,
    ) -> Result<()> {
        let mut metadata_store = self.metadata_store.write().await;

        // Create file metadata
        let file_metadata = FileMetadata {
            path: analysis.path.clone(),
            language: analysis.language.clone(),
            embedding_id: embedding.metadata.content_hash.clone(),
            loc: analysis.metrics.loc,
            complexity: analysis.metrics.complexity,
            functions: analysis.functions.iter().map(|f| f.name.clone()).collect(),
            classes: analysis.classes.iter().map(|c| c.name.clone()).collect(),
            imports: analysis.imports.iter().map(|i| i.path.clone()).collect(),
            last_indexed: chrono::Utc::now(),
        };

        metadata_store.add_file_metadata(file_metadata)?;

        Ok(())
    }

    /// Search for similar files
    pub async fn search(
        &self,
        query: &str,
        embedding_type: EmbeddingType,
        max_results: usize,
    ) -> Result<Vec<SearchResult>> {
        info!("Searching for: {}", query);

        // Create query embedding
        let query_embedding = self.create_query_embedding(query, embedding_type).await?;

        // Search vector store
        let params = VectorSearchParams {
            max_results,
            min_score: 0.5,
        };

        let vector_results = {
            let vector_store = self.vector_store.read().await;
            vector_store.search(&query_embedding.vector, params)?
        };

        // Get metadata for results
        let mut search_results = Vec::new();

        {
            let metadata_store = self.metadata_store.read().await;

            for result in vector_results {
                if let Some(metadata) = metadata_store.get_file_metadata_by_embedding_id(&result.id)
                {
                    search_results.push(SearchResult {
                        path: metadata.path.clone(),
                        score: result.score,
                        metadata: metadata.clone(),
                    });
                }
            }
        }

        Ok(search_results)
    }

    /// Create a query embedding
    async fn create_query_embedding(
        &self,
        query: &str,
        embedding_type: EmbeddingType,
    ) -> Result<Embedding> {
        // Use LLM to create embedding for the query
        let llm = Arc::new(crate::llm::LlmFactory::create_model(self.config.llm.clone()).await?);

        let vector = llm.embed(query).await?;

        let metadata = crate::embedding::EmbeddingMetadata {
            source_path: PathBuf::from("query"),
            content_hash: "query".to_string(),
            language: None,
            timestamp: chrono::Utc::now(),
            attributes: HashMap::new(),
        };

        Ok(Embedding {
            vector,
            embedding_type,
            metadata,
        })
    }

    /// Save index to disk
    pub async fn save(&self, path: &Path) -> Result<()> {
        info!("Saving index to {}", path.display());

        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ZseiError::Indexing(format!("Failed to create directory: {}", e)))?;
        }

        // Save vector store
        let vector_store_path = path.join("vectors.bin");
        {
            let vector_store = self.vector_store.read().await;
            vector_store.save(&vector_store_path)?;
        }

        // Save metadata store
        let metadata_store_path = path.join("metadata.json");
        {
            let metadata_store = self.metadata_store.read().await;
            metadata_store.save(&metadata_store_path)?;
        }

        Ok(())
    }

    /// Load index from disk
    pub async fn load(&self, path: &Path) -> Result<()> {
        info!("Loading index from {}", path.display());

        // Load vector store
        let vector_store_path = path.join("vectors.bin");
        if vector_store_path.exists() {
            let mut vector_store = self.vector_store.write().await;
            vector_store.load(&vector_store_path)?;
        } else {
            return Err(ZseiError::Indexing(format!(
                "Vector store file not found: {}",
                vector_store_path.display()
            )));
        }

        // Load metadata store
        let metadata_store_path = path.join("metadata.json");
        if metadata_store_path.exists() {
            let mut metadata_store = self.metadata_store.write().await;
            metadata_store.load(&metadata_store_path)?;
        } else {
            return Err(ZseiError::Indexing(format!(
                "Metadata store file not found: {}",
                metadata_store_path.display()
            )));
        }

        Ok(())
    }
}

/// File metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// File path
    pub path: PathBuf,

    /// Language
    pub language: String,

    /// Embedding ID
    pub embedding_id: String,

    /// Lines of code
    pub loc: usize,

    /// Complexity
    pub complexity: usize,

    /// Functions
    pub functions: Vec<String>,

    /// Classes
    pub classes: Vec<String>,

    /// Imports
    pub imports: Vec<String>,

    /// Last indexed time
    pub last_indexed: chrono::DateTime<chrono::Utc>,
}

/// Search result
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// File path
    pub path: PathBuf,

    /// Similarity score
    pub score: f32,

    /// File metadata
    pub metadata: FileMetadata,
}
