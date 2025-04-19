//! Vector storage for ZSEI
//!
//! This module provides vector storage functionality for the
//! Zero-Shot Bolted Embedding Indexer.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use crate::embedding::Embedding;
use crate::errors::{Result, ZseiError};

/// Vector search parameters
#[derive(Debug, Clone)]
pub struct VectorSearchParams {
    /// Maximum number of results
    pub max_results: usize,

    /// Minimum similarity score
    pub min_score: f32,
}

/// Vector search result
#[derive(Debug, Clone)]
pub struct VectorSearchResult {
    /// Embedding ID
    pub id: String,

    /// Similarity score
    pub score: f32,

    /// Embedding type
    pub embedding_type: crate::embedding::EmbeddingType,
}

/// Vector store
pub struct VectorStore {
    /// Embeddings indexed by ID
    embeddings: HashMap<String, StoredEmbedding>,
}

/// Stored embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredEmbedding {
    /// Embedding ID
    id: String,

    /// Embedding vector
    vector: Vec<f32>,

    /// Embedding type
    embedding_type: crate::embedding::EmbeddingType,

    /// Embedding dimension
    dimension: usize,
}

impl VectorStore {
    /// Create a new vector store
    pub fn new() -> Self {
        Self {
            embeddings: HashMap::new(),
        }
    }

    /// Add an embedding to the store
    pub fn add_embedding(&mut self, embedding: Embedding) -> Result<()> {
        // Extract info from embedding
        let id = embedding.metadata.content_hash.clone();
        let vector = embedding.vector;
        let embedding_type = embedding.embedding_type;
        let dimension = vector.len();

        // Create stored embedding
        let stored_embedding = StoredEmbedding {
            id: id.clone(),
            vector,
            embedding_type,
            dimension,
        };

        // Add to store
        self.embeddings.insert(id, stored_embedding);

        Ok(())
    }

    /// Search for similar embeddings
    pub fn search(
        &self,
        query: &[f32],
        params: VectorSearchParams,
    ) -> Result<Vec<VectorSearchResult>> {
        let mut results = Vec::new();

        // Calculate similarity for each embedding
        for (id, embedding) in &self.embeddings {
            let score = self.cosine_similarity(query, &embedding.vector);

            if score >= params.min_score {
                results.push(VectorSearchResult {
                    id: id.clone(),
                    score,
                    embedding_type: embedding.embedding_type.clone(),
                });
            }
        }

        // Sort by score (descending)
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit to max results
        if results.len() > params.max_results {
            results.truncate(params.max_results);
        }

        Ok(results)
    }

    /// Save the vector store to disk
    pub fn save(&self, path: &Path) -> Result<()> {
        // Serialize to binary format
        let file = File::create(path)
            .map_err(|e| ZseiError::Indexing(format!("Failed to create file: {}", e)))?;

        let writer = BufWriter::new(file);

        serde_json::to_writer(writer, &self.embeddings)
            .map_err(|e| ZseiError::Indexing(format!("Failed to serialize vector store: {}", e)))?;

        Ok(())
    }

    /// Load the vector store from disk
    pub fn load(&mut self, path: &Path) -> Result<()> {
        // Deserialize from binary format
        let file = File::open(path)
            .map_err(|e| ZseiError::Indexing(format!("Failed to open file: {}", e)))?;

        let reader = BufReader::new(file);

        self.embeddings = serde_json::from_reader(reader).map_err(|e| {
            ZseiError::Indexing(format!("Failed to deserialize vector store: {}", e))
        })?;

        Ok(())
    }

    /// Calculate cosine similarity between two vectors
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot_product / (norm_a * norm_b)
        }
    }
}
