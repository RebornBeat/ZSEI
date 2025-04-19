//! Code embedding generation
//!
//! This module provides code embedding generation for the Zero-Shot
//! Bolted Embedding Indexer.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info};

use crate::analyzers::common::{Class, FileAnalysis, Function, Import, Variable};
use crate::core::config::EmbeddingConfig;
use crate::embedding::{Embedding, EmbeddingGenerator, EmbeddingMetadata, EmbeddingType};
use crate::errors::{Result, ZseiError};
use crate::llm::prompt::PromptManager;
use crate::llm::Model;

/// Code embedding generator
pub struct CodeEmbeddingGenerator {
    /// LLM for enhanced embedding generation
    llm: Arc<dyn Model>,

    /// Configuration
    config: EmbeddingConfig,

    /// Prompt manager
    prompt_manager: PromptManager,
}

impl CodeEmbeddingGenerator {
    /// Create a new code embedding generator
    pub fn new(llm: Arc<dyn Model>, config: EmbeddingConfig) -> Self {
        Self {
            llm,
            config,
            prompt_manager: PromptManager::new(),
        }
    }

    /// Generate syntactic features from code analysis
    async fn generate_syntactic_features(&self, analysis: &FileAnalysis) -> Result<Vec<f32>> {
        // Extract syntactic features from code analysis
        debug!(
            "Generating syntactic features for {}",
            analysis.path.display()
        );

        // Feature vector size allocation
        let syntactic_dim = self.config.dimension / 4;
        let mut features = Vec::with_capacity(syntactic_dim);

        // 1. Language features
        let language_features = self.extract_language_features(analysis)?;
        features.extend(language_features);

        // 2. Structure features
        let structure_features = self.extract_structure_features(analysis)?;
        features.extend(structure_features);

        // 3. Complexity features
        let complexity_features = self.extract_complexity_features(analysis)?;
        features.extend(complexity_features);

        // 4. Import features
        let import_features = self.extract_import_features(analysis)?;
        features.extend(import_features);

        // Ensure the vector has the correct dimension
        self.pad_or_truncate(&mut features, syntactic_dim)?;

        Ok(features)
    }

    /// Generate semantic features from code analysis
    async fn generate_semantic_features(&self, analysis: &FileAnalysis) -> Result<Vec<f32>> {
        // Use LLM to extract semantic features
        debug!(
            "Generating semantic features for {}",
            analysis.path.display()
        );

        // Create prompt for semantic analysis
        let mut variables = HashMap::new();
        variables.insert(
            "code".to_string(),
            analysis.content.clone().unwrap_or_default(),
        );
        variables.insert("language".to_string(), analysis.language.clone());

        let prompt = self
            .prompt_manager
            .create_prompt("code_analysis", &variables)
            .map_err(|e| ZseiError::Embedding(format!("Failed to create prompt: {}", e)))?;

        // Generate embedding from LLM
        let semantic_dim = self.config.dimension / 4;
        let embedding = self.llm.embed(&prompt).await?;

        // Ensure the vector has the correct dimension
        let mut features = embedding;
        self.pad_or_truncate(&mut features, semantic_dim)?;

        Ok(features)
    }

    /// Generate structural features from code analysis
    async fn generate_structural_features(&self, analysis: &FileAnalysis) -> Result<Vec<f32>> {
        // Extract structural features from code analysis
        debug!(
            "Generating structural features for {}",
            analysis.path.display()
        );

        // Feature vector size allocation
        let structural_dim = self.config.dimension / 4;
        let mut features = Vec::with_capacity(structural_dim);

        // 1. Function features
        let function_features = self.extract_function_features(analysis)?;
        features.extend(function_features);

        // 2. Class features
        let class_features = self.extract_class_features(analysis)?;
        features.extend(class_features);

        // 3. Variable features
        let variable_features = self.extract_variable_features(analysis)?;
        features.extend(variable_features);

        // Ensure the vector has the correct dimension
        self.pad_or_truncate(&mut features, structural_dim)?;

        Ok(features)
    }

    /// Generate relationship features from code analysis
    async fn generate_relationship_features(&self, analysis: &FileAnalysis) -> Result<Vec<f32>> {
        // Extract relationship features from code analysis
        debug!(
            "Generating relationship features for {}",
            analysis.path.display()
        );

        // Feature vector size allocation
        let relationship_dim = self.config.dimension / 4;
        let mut features = Vec::with_capacity(relationship_dim);

        // 1. Import relationship features
        let import_rel_features = self.extract_import_relationship_features(analysis)?;
        features.extend(import_rel_features);

        // 2. Function relationship features
        let function_rel_features = self.extract_function_relationship_features(analysis)?;
        features.extend(function_rel_features);

        // Ensure the vector has the correct dimension
        self.pad_or_truncate(&mut features, relationship_dim)?;

        Ok(features)
    }

    /// Extract language features
    fn extract_language_features(&self, analysis: &FileAnalysis) -> Result<Vec<f32>> {
        let mut features = Vec::new();

        // One-hot encoding for language
        match analysis.language.as_str() {
            "Rust" => features.extend_from_slice(&[1.0, 0.0, 0.0, 0.0, 0.0]),
            "Python" => features.extend_from_slice(&[0.0, 1.0, 0.0, 0.0, 0.0]),
            "JavaScript" => features.extend_from_slice(&[0.0, 0.0, 1.0, 0.0, 0.0]),
            "TypeScript" => features.extend_from_slice(&[0.0, 0.0, 0.0, 1.0, 0.0]),
            _ => features.extend_from_slice(&[0.0, 0.0, 0.0, 0.0, 1.0]),
        }

        Ok(features)
    }

    /// Extract structure features
    fn extract_structure_features(&self, analysis: &FileAnalysis) -> Result<Vec<f32>> {
        let mut features = Vec::new();

        // File size (normalized)
        let loc = analysis.metrics.loc as f32;
        features.push((loc / 1000.0).min(1.0)); // Normalize to 0-1

        // Function ratio
        let function_ratio = if loc > 0.0 {
            analysis.functions.len() as f32 / loc
        } else {
            0.0
        };
        features.push(function_ratio * 100.0); // Scale for better distribution

        // Class ratio
        let class_ratio = if loc > 0.0 {
            analysis.classes.len() as f32 / loc
        } else {
            0.0
        };
        features.push(class_ratio * 100.0);

        // Comment ratio
        let comment_ratio = if loc > 0.0 {
            analysis.metrics.comment_lines as f32 / loc
        } else {
            0.0
        };
        features.push(comment_ratio);

        // Import count (normalized)
        features.push((analysis.imports.len() as f32 / 50.0).min(1.0));

        Ok(features)
    }

    /// Extract complexity features
    fn extract_complexity_features(&self, analysis: &FileAnalysis) -> Result<Vec<f32>> {
        let mut features = Vec::new();

        // Overall complexity
        features.push((analysis.metrics.complexity as f32 / 100.0).min(1.0));

        // Maintainability index (normalized)
        features.push(analysis.metrics.maintainability_index as f32 / 100.0);

        // Average function complexity
        let avg_complexity = if !analysis.functions.is_empty() {
            analysis
                .functions
                .iter()
                .map(|f| f.metrics.complexity)
                .sum::<usize>() as f32
                / analysis.functions.len() as f32
        } else {
            0.0
        };
        features.push((avg_complexity / 10.0).min(1.0));

        // Max function complexity
        let max_complexity = analysis
            .functions
            .iter()
            .map(|f| f.metrics.complexity)
            .max()
            .unwrap_or(0) as f32;
        features.push((max_complexity / 20.0).min(1.0));

        // Average function size
        let avg_function_size = if !analysis.functions.is_empty() {
            analysis
                .functions
                .iter()
                .map(|f| f.metrics.loc)
                .sum::<usize>() as f32
                / analysis.functions.len() as f32
        } else {
            0.0
        };
        features.push((avg_function_size / 50.0).min(1.0));

        Ok(features)
    }

    /// Extract import features
    fn extract_import_features(&self, analysis: &FileAnalysis) -> Result<Vec<f32>> {
        let mut features = Vec::new();

        // Import count
        features.push((analysis.imports.len() as f32 / 50.0).min(1.0));

        // External vs internal imports
        let (external_count, internal_count) = self.count_import_types(&analysis.imports);

        let external_ratio = if !analysis.imports.is_empty() {
            external_count as f32 / analysis.imports.len() as f32
        } else {
            0.0
        };
        features.push(external_ratio);

        let internal_ratio = if !analysis.imports.is_empty() {
            internal_count as f32 / analysis.imports.len() as f32
        } else {
            0.0
        };
        features.push(internal_ratio);

        Ok(features)
    }

    /// Extract function features
    fn extract_function_features(&self, analysis: &FileAnalysis) -> Result<Vec<f32>> {
        let mut features = Vec::new();

        // Function count
        features.push((analysis.functions.len() as f32 / 50.0).min(1.0));

        // Public vs private functions
        let public_count = analysis.functions.iter().filter(|f| f.is_public).count();
        let public_ratio = if !analysis.functions.is_empty() {
            public_count as f32 / analysis.functions.len() as f32
        } else {
            0.0
        };
        features.push(public_ratio);

        // Average parameters per function
        let avg_params = if !analysis.functions.is_empty() {
            analysis
                .functions
                .iter()
                .map(|f| f.parameters.len())
                .sum::<usize>() as f32
                / analysis.functions.len() as f32
        } else {
            0.0
        };
        features.push((avg_params / 5.0).min(1.0));

        // Average function size
        let avg_size = if !analysis.functions.is_empty() {
            analysis
                .functions
                .iter()
                .map(|f| f.metrics.loc)
                .sum::<usize>() as f32
                / analysis.functions.len() as f32
        } else {
            0.0
        };
        features.push((avg_size / 50.0).min(1.0));

        Ok(features)
    }

    /// Extract class features
    fn extract_class_features(&self, analysis: &FileAnalysis) -> Result<Vec<f32>> {
        let mut features = Vec::new();

        // Class count
        features.push((analysis.classes.len() as f32 / 20.0).min(1.0));

        // Public vs private classes
        let public_count = analysis.classes.iter().filter(|c| c.is_public).count();
        let public_ratio = if !analysis.classes.is_empty() {
            public_count as f32 / analysis.classes.len() as f32
        } else {
            0.0
        };
        features.push(public_ratio);

        // Average methods per class
        let avg_methods = if !analysis.classes.is_empty() {
            analysis
                .classes
                .iter()
                .map(|c| c.methods.len())
                .sum::<usize>() as f32
                / analysis.classes.len() as f32
        } else {
            0.0
        };
        features.push((avg_methods / 10.0).min(1.0));

        // Average properties per class
        let avg_props = if !analysis.classes.is_empty() {
            analysis
                .classes
                .iter()
                .map(|c| c.properties.len())
                .sum::<usize>() as f32
                / analysis.classes.len() as f32
        } else {
            0.0
        };
        features.push((avg_props / 10.0).min(1.0));

        // Average inheritance depth
        let avg_depth = if !analysis.classes.is_empty() {
            analysis
                .classes
                .iter()
                .map(|c| c.metrics.inheritance_depth)
                .sum::<usize>() as f32
                / analysis.classes.len() as f32
        } else {
            0.0
        };
        features.push((avg_depth / 3.0).min(1.0));

        Ok(features)
    }

    /// Extract variable features
    fn extract_variable_features(&self, analysis: &FileAnalysis) -> Result<Vec<f32>> {
        let mut features = Vec::new();

        // Variable count
        features.push((analysis.variables.len() as f32 / 100.0).min(1.0));

        // Typed vs untyped variables
        let typed_count = analysis
            .variables
            .iter()
            .filter(|v| v.var_type.is_some())
            .count();
        let typed_ratio = if !analysis.variables.is_empty() {
            typed_count as f32 / analysis.variables.len() as f32
        } else {
            0.0
        };
        features.push(typed_ratio);

        // Public vs private variables
        let public_count = analysis.variables.iter().filter(|v| v.is_public).count();
        let public_ratio = if !analysis.variables.is_empty() {
            public_count as f32 / analysis.variables.len() as f32
        } else {
            0.0
        };
        features.push(public_ratio);

        Ok(features)
    }

    /// Extract import relationship features
    fn extract_import_relationship_features(&self, analysis: &FileAnalysis) -> Result<Vec<f32>> {
        let mut features = Vec::new();

        // Import density
        let import_density = if analysis.metrics.loc > 0 {
            analysis.imports.len() as f32 / analysis.metrics.loc as f32
        } else {
            0.0
        };
        features.push(import_density * 100.0);

        // Import uniqueness
        let unique_imports = analysis
            .imports
            .iter()
            .map(|i| i.path.clone())
            .collect::<std::collections::HashSet<_>>()
            .len();

        let import_uniqueness = if !analysis.imports.is_empty() {
            unique_imports as f32 / analysis.imports.len() as f32
        } else {
            0.0
        };
        features.push(import_uniqueness);

        Ok(features)
    }

    /// Extract function relationship features
    fn extract_function_relationship_features(&self, analysis: &FileAnalysis) -> Result<Vec<f32>> {
        let mut features = Vec::new();

        // Function density
        let function_density = if analysis.metrics.loc > 0 {
            analysis.functions.len() as f32 / analysis.metrics.loc as f32
        } else {
            0.0
        };
        features.push(function_density * 100.0);

        // Function name uniqueness
        let unique_functions = analysis
            .functions
            .iter()
            .map(|f| f.name.clone())
            .collect::<std::collections::HashSet<_>>()
            .len();

        let function_uniqueness = if !analysis.functions.is_empty() {
            unique_functions as f32 / analysis.functions.len() as f32
        } else {
            0.0
        };
        features.push(function_uniqueness);

        Ok(features)
    }

    /// Count external vs internal imports
    fn count_import_types(&self, imports: &[Import]) -> (usize, usize) {
        let mut external_count = 0;
        let mut internal_count = 0;

        for import in imports {
            if import.is_relative {
                internal_count += 1;
            } else {
                // Heuristic: if the import has a dot, it's likely an external package
                if import.path.contains('.') {
                    external_count += 1;
                } else {
                    internal_count += 1;
                }
            }
        }

        (external_count, internal_count)
    }

    /// Pad or truncate a vector to the desired length
    fn pad_or_truncate(&self, vector: &mut Vec<f32>, target_length: usize) -> Result<()> {
        if vector.len() < target_length {
            // Pad with zeros
            vector.resize(target_length, 0.0);
        } else if vector.len() > target_length {
            // Truncate
            vector.truncate(target_length);
        }

        Ok(())
    }

    /// Calculate content hash
    fn calculate_content_hash(&self, content: &str) -> String {
        // Import the md5 crate functions
        use md5::compute;

        // Compute the MD5 hash of the content bytes
        // The compute function handles all the digest operations internally
        let digest = compute(content.as_bytes());

        // Format the result as a hexadecimal string
        format!("{:x}", digest)
    }
}

#[async_trait]
impl EmbeddingGenerator for CodeEmbeddingGenerator {
    type Input = FileAnalysis;

    async fn generate(&self, input: &Self::Input) -> Result<Embedding> {
        info!("Generating embedding for {}", input.path.display());

        // 1. Generate feature vectors
        let syntactic_features = self.generate_syntactic_features(input).await?;
        debug!(
            "Generated syntactic features: {} dimensions",
            syntactic_features.len()
        );

        let semantic_features = self.generate_semantic_features(input).await?;
        debug!(
            "Generated semantic features: {} dimensions",
            semantic_features.len()
        );

        let structural_features = self.generate_structural_features(input).await?;
        debug!(
            "Generated structural features: {} dimensions",
            structural_features.len()
        );

        let relationship_features = self.generate_relationship_features(input).await?;
        debug!(
            "Generated relationship features: {} dimensions",
            relationship_features.len()
        );

        // 2. Combine vectors
        let mut vector = Vec::with_capacity(self.config.dimension);
        vector.extend_from_slice(&syntactic_features);
        vector.extend_from_slice(&semantic_features);
        vector.extend_from_slice(&structural_features);
        vector.extend_from_slice(&relationship_features);

        // 3. Normalize vector
        crate::embedding::utils::normalize(&mut vector);

        // 4. Create metadata
        let content_hash = if let Some(content) = &input.content {
            self.calculate_content_hash(content)
        } else {
            String::new()
        };

        let metadata = EmbeddingMetadata {
            source_path: input.path.clone(),
            content_hash,
            language: Some(input.language.clone()),
            timestamp: chrono::Utc::now(),
            attributes: self.extract_metadata_attributes(input),
        };

        // 5. Create embedding
        let embedding = Embedding {
            vector,
            embedding_type: EmbeddingType::Code,
            metadata,
        };

        Ok(embedding)
    }

    fn dimension(&self) -> usize {
        self.config.dimension
    }

    fn embedding_type(&self) -> EmbeddingType {
        EmbeddingType::Code
    }
}

impl CodeEmbeddingGenerator {
    /// Extract metadata attributes
    fn extract_metadata_attributes(&self, analysis: &FileAnalysis) -> HashMap<String, String> {
        let mut attributes = HashMap::new();

        // Add file metrics
        attributes.insert("loc".to_string(), analysis.metrics.loc.to_string());
        attributes.insert(
            "complexity".to_string(),
            analysis.metrics.complexity.to_string(),
        );
        attributes.insert(
            "maintainability".to_string(),
            analysis.metrics.maintainability_index.to_string(),
        );

        // Add function and class counts
        attributes.insert(
            "function_count".to_string(),
            analysis.functions.len().to_string(),
        );
        attributes.insert(
            "class_count".to_string(),
            analysis.classes.len().to_string(),
        );
        attributes.insert(
            "import_count".to_string(),
            analysis.imports.len().to_string(),
        );
        attributes.insert(
            "variable_count".to_string(),
            analysis.variables.len().to_string(),
        );

        // Add language
        attributes.insert("language".to_string(), analysis.language.clone());

        // Add file extension
        if let Some(ext) = analysis.path.extension() {
            if let Some(ext_str) = ext.to_str() {
                attributes.insert("extension".to_string(), ext_str.to_string());
            }
        }

        attributes
    }
}
