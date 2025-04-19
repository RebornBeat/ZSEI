//! Embedding generation for ZSEI
//!
//! This module provides embedding generation for the Zero-Shot
//! Bolted Embedding Indexer.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub mod code;
// Future modalities will be added as separate modules:
// pub mod image;
// pub mod audio;
// pub mod video;

use crate::core::config::Config;
use crate::errors::{Result, ZseiError};
use crate::llm::Model;

/// Embedding type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmbeddingType {
    /// Code embedding
    Code,

    /// Image embedding
    Image,

    /// Audio embedding
    Audio,

    /// Video embedding
    Video,
}

/// Embedding with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    /// Embedding vector
    pub vector: Vec<f32>,

    /// Embedding type
    pub embedding_type: EmbeddingType,

    /// Embedding metadata
    pub metadata: EmbeddingMetadata,
}

/// Embedding metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingMetadata {
    /// Source path
    pub source_path: PathBuf,

    /// Source content hash
    pub content_hash: String,

    /// Source language
    pub language: Option<String>,

    /// Creation timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Additional attributes
    pub attributes: HashMap<String, String>,
}

/// Embedding generator trait
#[async_trait]
pub trait EmbeddingGenerator: Send + Sync {
    /// Input type
    type Input;

    /// Generate embeddings for the input
    async fn generate(&self, input: &Self::Input) -> Result<Embedding>;

    /// Get the embedding dimension
    fn dimension(&self) -> usize;

    /// Get the embedding type
    fn embedding_type(&self) -> EmbeddingType;
}

/// Embedding factory
pub struct EmbeddingFactory {
    /// LLM for enhanced embedding generation
    llm: Arc<dyn Model>,

    /// Configuration
    config: Arc<Config>,
}

impl EmbeddingFactory {
    /// Create a new embedding factory
    pub fn new(llm: Arc<dyn Model>, config: Arc<Config>) -> Self {
        Self { llm, config }
    }

    /// Create a code embedding generator
    pub fn create_code_generator(
        &self,
    ) -> Box<dyn EmbeddingGenerator<Input = crate::analyzers::common::FileAnalysis>> {
        Box::new(code::CodeEmbeddingGenerator::new(
            self.llm.clone(),
            self.config.embedding.clone(),
        ))
    }

    // Add more generator creation methods for other modalities as they are implemented
    // pub fn create_image_generator(&self) -> Box<dyn EmbeddingGenerator<Input=crate::analyzers::common::ImageAnalysis>> {
    //     Box::new(image::ImageEmbeddingGenerator::new(
    //         self.llm.clone(),
    //         self.config.embedding.clone(),
    //     ))
    // }
}

/// Utility functions for embeddings
pub mod utils {
    use super::*;

    /// Calculate cosine similarity between two embeddings
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
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

    /// Calculate euclidean distance between two embeddings
    pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return f32::MAX;
        }

        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f32>()
            .sqrt()
    }

    /// Normalize an embedding vector to unit length
    pub fn normalize(vector: &mut [f32]) {
        let norm: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm > 0.0 {
            for x in vector.iter_mut() {
                *x /= norm;
            }
        }
    }

    /// Create a combined embedding from multiple embeddings
    pub fn combine_embeddings(
        embeddings: &[Embedding],
        weights: Option<&[f32]>,
    ) -> Result<Vec<f32>> {
        if embeddings.is_empty() {
            return Err(ZseiError::Embedding("No embeddings to combine".to_string()));
        }

        let dimension = embeddings[0].vector.len();

        // Check that all embeddings have the same dimension
        for embedding in embeddings {
            if embedding.vector.len() != dimension {
                return Err(ZseiError::Embedding(format!(
                    "Embedding dimension mismatch: expected {}, found {}",
                    dimension,
                    embedding.vector.len()
                )));
            }
        }

        // Get weights
        let weights = if let Some(w) = weights {
            if w.len() != embeddings.len() {
                return Err(ZseiError::Embedding(format!(
                    "Weight count mismatch: got {} weights for {} embeddings",
                    w.len(),
                    embeddings.len()
                )));
            }
            w
        } else {
            // Equal weights if none provided
            &vec![1.0 / embeddings.len() as f32; embeddings.len()]
        };

        // Create combined vector
        let mut combined = vec![0.0; dimension];

        for (i, embedding) in embeddings.iter().enumerate() {
            let weight = weights[i];

            for (j, &value) in embedding.vector.iter().enumerate() {
                combined[j] += value * weight;
            }
        }

        // Normalize
        normalize(&mut combined);

        Ok(combined)
    }
}
