//! LLM integration for ZSEI
//!
//! This module provides integration with Language Models for the
//! Zero-Shot Embedding Indexer.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub mod models;
pub mod prompt;

use crate::errors::{Result, ZseiError};

/// LLM configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Model type
    pub model_type: ModelType,

    /// Path to the model
    pub model_path: Option<PathBuf>,

    /// Model parameters
    pub parameters: ModelParameters,
}

impl Default for LlmConfig {
    fn default() -> Self {
        let mut config = Self {
            model_type: ModelType::PhiMini,
            model_path: None,
            parameters: ModelParameters::default(),
        };

        // Auto-detect model path
        config.model_path = config.find_model_path();

        config
    }
}

impl LlmConfig {
    /// Look for model in standard locations
    pub fn find_model_path(&self) -> Option<PathBuf> {
        // Try env variable first
        if let Ok(path) = std::env::var("ZSEI_MODEL_PATH") {
            let model_path = PathBuf::from(path);
            if Self::is_valid_model_dir(&model_path) {
                return Some(model_path);
            }
        }

        // Try standard locations
        let standard_locations = [
            // Current directory
            PathBuf::from("./models/phi-4-mini"),
            // Application directory
            dirs::data_local_dir().map(|p| p.join("zsei/models/phi-4-mini")).unwrap_or_default(),
            // Home directory
            dirs::home_dir().map(|p| p.join("zsei/models/phi-4-mini")).unwrap_or_default(),
            // Executable directory
            std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.join("models/phi-4-mini"))).unwrap_or_default(),
            // Custom path - adjust as needed
            PathBuf::from("/home/rebornbeat/Projects/zsei/models/phi-4-mini"),
        ];

        // Check each location
        for path in standard_locations {
            if Self::is_valid_model_dir(&path) {
                return Some(path);
            }
        }

        None
    }

    /// Check if a directory contains a valid model
    fn is_valid_model_dir(path: &Path) -> bool {
        path.join("model.onnx").exists() &&
        path.join("tokenizer.json").exists()
    }

    /// Update configuration with auto-detected model
    pub fn with_auto_detected_model(mut self) -> Self {
        if self.model_path.is_none() {
            self.model_path = self.find_model_path();
        }
        self
    }
}

/// Model types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelType {
    /// Phi-4 Mini model
    PhiMini,

    /// Custom model
    Custom(String),
}

/// Model parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelParameters {
    /// Temperature
    pub temperature: f32,

    /// Top-p
    pub top_p: f32,

    /// Top-k
    pub top_k: u32,

    /// Maximum tokens
    pub max_tokens: usize,

    /// Whether to use GPU
    pub use_gpu: bool,
}

impl Default for ModelParameters {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            max_tokens: 4096,
            use_gpu: false,
        }
    }
}

/// LLM model trait
#[async_trait]
pub trait Model: Send + Sync {
    /// Generate text from a prompt
    async fn generate(&self, prompt: &str) -> Result<String>;

    /// Generate text from a prompt with parameters
    async fn generate_with_parameters(
        &self,
        prompt: &str,
        parameters: ModelParameters,
    ) -> Result<String>;

    /// Generate embeddings for text
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
}

/// LLM factory
pub struct LlmFactory;

impl LlmFactory {
    /// Create a new LLM model
    pub async fn create_model(config: LlmConfig) -> Result<Arc<dyn Model>> {
        // Apply auto-detection if no path is provided
        let config = config.with_auto_detected_model();

        match config.model_type {
            ModelType::PhiMini => {
                let model_path = config.model_path.ok_or_else(|| {
                    ZseiError::Llm("Model path is required for Phi-Mini".to_string())
                })?;

                println!("Using model at path: {:?}", model_path);
                let model = models::phi::PhiModel::new(model_path, config.parameters)?;
                Ok(Arc::new(model))
            }
            ModelType::Custom(model_name) => {
                Err(ZseiError::Llm(format!("Custom model {} not supported yet", model_name)))
            }
        }
    }

    /// Get a list of available models
    pub fn list_available_models() -> Result<Vec<(String, PathBuf)>> {
        let mut models = Vec::new();

        // Search standard locations
        let search_paths = [
            PathBuf::from("./models"),
            dirs::data_local_dir().map(|p| p.join("zsei/models")).unwrap_or_default(),
            dirs::home_dir().map(|p| p.join("zsei/models")).unwrap_or_default(),
            std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.join("models"))).unwrap_or_default(),
        ];

        for search_path in &search_paths {
            if !search_path.exists() {
                continue;
            }

            if let Ok(entries) = std::fs::read_dir(search_path) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        if let Ok(file_type) = entry.file_type() {
                            if file_type.is_dir() {
                                let model_dir = entry.path();
                                let model_name = entry.file_name().to_string_lossy().to_string();

                                // Check if it's a valid model directory
                                if LlmConfig::is_valid_model_dir(&model_dir) {
                                    models.push((model_name, model_dir));
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(models)
    }

    /// Select a model interactively
    pub fn select_model_interactive() -> Result<PathBuf> {
        use dialoguer::Select;

        let available_models = Self::list_available_models()?;

        if available_models.is_empty() {
            return Err(ZseiError::Llm("No models found. Please download a model first.".to_string()));
        }

        if available_models.len() == 1 {
            // Only one model, use it automatically
            return Ok(available_models[0].1.clone());
        }

        // Prepare options for display
        let options: Vec<String> = available_models.iter()
            .map(|(name, path)| format!("{} ({})", name, path.display()))
            .collect();

        let selection = Select::new()
            .with_prompt("Select a model")
            .items(&options)
            .default(0)
            .interact()
            .map_err(|e| ZseiError::Other(format!("Failed to get selection: {}", e)))?;

        Ok(available_models[selection].1.clone())
    }
}
