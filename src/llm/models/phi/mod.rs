//! Phi-4 Mini LLM integration
//!
//! This module provides integration with the Phi-4 Mini language model.

use async_trait::async_trait;
use ort::execution_providers::CUDAExecutionProviderOptions;
use ort::{Environment, ExecutionProvider, GraphOptimizationLevel, Session, SessionBuilder, Value};
use rand::distr::weighted::WeightedIndex;
use rand::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokenizers::Tokenizer;
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::errors::{Result, ZseiError};
use crate::llm::{Model, ModelParameters};

/// Phi-4 Mini model
pub struct PhiModel {
    /// Model path
    model_path: PathBuf,

    /// Model parameters
    parameters: ModelParameters,

    /// Tokenizer
    tokenizer: Arc<PhiTokenizer>,

    /// Model pipeline
    pipeline: Arc<RwLock<PhiPipeline>>,
}

/// Phi tokenizer
struct PhiTokenizer {
    /// Inner tokenizer
    inner: Tokenizer,

    /// Tokenizer path
    tokenizer_path: PathBuf,

    /// Special tokens processor
    special_tokens_processor: Option<SpecialTokensProcessor>,
}

/// Special tokens processor
struct SpecialTokensProcessor {
    /// Special tokens map
    tokens: HashMap<String, String>,
}

/// Phi pipeline
struct PhiPipeline {
    /// Session
    session: Session,

    /// Environment
    env: Arc<Environment>,
}

impl PhiModel {
    /// Create a new Phi model
    pub fn new(model_path: PathBuf, parameters: ModelParameters) -> Result<Self> {
        info!("Loading Phi-4 Mini model from: {}", model_path.display());

        // Check if model exists
        if !model_path.exists() {
            return Err(ZseiError::Llm(format!(
                "Model path does not exist: {}",
                model_path.display()
            )));
        }

        // Load tokenizer
        let tokenizer_path = model_path.join("tokenizer.json");
        debug!("Loading tokenizer from: {}", tokenizer_path.display());
        let tokenizer = PhiTokenizer::new(tokenizer_path)?;

        // Create environment
        let mut env_builder = Environment::builder()
            .with_name("phi")
            .with_log_level(ort::LoggingLevel::Warning);

        // Enable CUDA if requested
        if parameters.use_gpu {
            debug!("Enabling GPU for Phi model");
            let providers: Vec<ExecutionProvider> = vec![ExecutionProvider::CUDA(
                CUDAExecutionProviderOptions::default(),
            )];
            env_builder = env_builder.with_execution_providers(&providers);
        }

        let env = Arc::new(env_builder.build()?);

        // Load model
        let model_file = model_path.join("model.onnx");
        debug!("Loading model from: {}", model_file.display());

        let mut session_builder = SessionBuilder::new(&env)?;

        // Configure session
        session_builder = session_builder
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?;

        let session = session_builder.with_model_from_file(model_file)?;

        // Create pipeline
        let pipeline = PhiPipeline { session, env };

        Ok(Self {
            model_path,
            parameters,
            tokenizer: Arc::new(tokenizer),
            pipeline: Arc::new(RwLock::new(pipeline)),
        })
    }

    /// Preprocess input
    async fn preprocess(&self, prompt: &str) -> Result<Vec<i64>> {
        // Properly format the prompt for Phi-4
        let formatted_prompt = if prompt.contains("<|im_start|") {
            // Already formatted
            prompt.to_string()
        } else {
            // Add Phi-4 prompt format
            format!(
                "<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n",
                prompt
            )
        };

        // Encode input
        let encoded = self.tokenizer.encode(&formatted_prompt)?;

        debug!("Encoded input tokens: {}", encoded.len());
        Ok(encoded)
    }

    /// Generate output tokens based on input tokens
    async fn generate_output(
        &self,
        input_ids: Vec<i64>,
        params: &ModelParameters,
    ) -> Result<Vec<i64>> {
        // Get pipeline reference with RwLock FIRST - before creating any non-Send resources
        let pipeline = self.pipeline.read().await;

        let mut output_ids = input_ids.clone();
        let mut current_length = output_ids.len();

        // Generate tokens up to max_tokens
        while current_length < params.max_tokens {
            // Create input data vector
            let input_data: Vec<i64> = output_ids.clone();

            // Create a dynamic dimension array (IxDyn)
            let input_shape_usize = vec![1, output_ids.len()];
            let array_data = ndarray::Array::from_vec(input_data);
            let array_reshaped = match array_data.into_shape(input_shape_usize) {
                Ok(arr) => arr,
                Err(e) => return Err(ZseiError::Llm(format!("Failed to reshape array: {}", e))),
            };
            let cow_array = ndarray::CowArray::from(array_reshaped);

            // Create memory info AFTER acquiring the pipeline
            let memory_info = ort::MemoryInfo::new(
                ort::AllocationDevice::CPU,
                0,
                ort::AllocatorType::Arena,
                ort::MemType::Default,
            )?;

            // Get the allocator from the session using the safe method
            let allocator_ptr = pipeline.session.allocator();

            // Create the tensor value with the correct types
            let input_values = vec![ort::Value::from_array(allocator_ptr, &cow_array)?];

            // Run inference
            let outputs = pipeline.session.run(input_values)?;

            // Get logits from output tensor
            let output_tensor = outputs[0].try_extract::<f32>()?;

            // Get tensor dimensions
            let view = output_tensor.view();
            let dims = view.shape();

            // Check that shape matches expectations
            if dims.len() != 3 {
                return Err(ZseiError::Llm(format!(
                    "Unexpected logits shape: {:?}",
                    dims
                )));
            }

            // Get dimensions for processing
            let batch_size = dims[0]; // Should be 1
            let seq_length = dims[1];
            let vocab_size = dims[2];

            // Get last token's logits
            let batch_idx = 0;
            let seq_idx = current_length - 1;

            // Extract logits for the last token
            let mut last_token_logits = Vec::with_capacity(vocab_size);
            for i in 0..vocab_size {
                // Access the 3D tensor directly
                last_token_logits.push(view[[batch_idx, seq_idx, i]]);
            }

            // Apply temperature scaling
            let mut token_logits = last_token_logits.clone();
            if params.temperature > 0.0 {
                for logit in &mut token_logits {
                    *logit /= params.temperature;
                }
            }

            // Apply top-k sampling
            let mut logits_with_indices: Vec<(usize, f32)> = token_logits
                .iter()
                .enumerate()
                .map(|(i, &v)| (i, v))
                .collect();

            // Sort by value in descending order
            logits_with_indices
                .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            // Take top-k
            let k = params.top_k as usize;
            if logits_with_indices.len() > k {
                logits_with_indices.truncate(k);
            }

            // Extract indices and values
            let top_k_indices: Vec<usize> =
                logits_with_indices.iter().map(|(idx, _)| *idx).collect();
            let top_k_values: Vec<f32> = logits_with_indices.iter().map(|(_, val)| *val).collect();

            // Apply softmax to get probabilities
            let max_val = top_k_values
                .iter()
                .copied()
                .fold(f32::NEG_INFINITY, f32::max);

            let exp_sum: f32 = top_k_values.iter().map(|&x| (x - max_val).exp()).sum();

            let probs: Vec<f32> = top_k_values
                .iter()
                .map(|&x| (x - max_val).exp() / exp_sum)
                .collect();

            // Apply top-p sampling
            let mut cumulative_prob = 0.0;
            let mut top_p_indices = Vec::new();
            let mut top_p_probs = Vec::new();

            for (i, &prob) in probs.iter().enumerate() {
                if cumulative_prob < params.top_p {
                    top_p_indices.push(top_k_indices[i]);
                    top_p_probs.push(prob);
                    cumulative_prob += prob;
                } else {
                    break;
                }
            }

            // Ensure we have at least one token
            if top_p_indices.is_empty() {
                top_p_indices.push(top_k_indices[0]);
                top_p_probs.push(1.0);
            }

            // Sample from the distribution
            let mut rng = rand::thread_rng();
            let dist = WeightedIndex::new(&top_p_probs)
                .map_err(|e| ZseiError::Llm(format!("Failed to create distribution: {}", e)))?;

            let selected_idx = top_p_indices[dist.sample(&mut rng)];
            let next_token = selected_idx as i64;

            // Check for EOS token
            if next_token == self.tokenizer.eos_token_id() {
                break;
            }

            // Add token to output
            output_ids.push(next_token);
            current_length = output_ids.len();

            // Safety check to prevent endless generation
            if current_length >= params.max_tokens {
                break;
            }
        }

        Ok(output_ids)
    }

    /// Postprocess output
    async fn postprocess(&self, output_ids: Vec<i64>, input_len: usize) -> Result<String> {
        // Get only the generated part
        let generated_ids = output_ids[input_len..].to_vec();

        // Decode output
        let output = self.tokenizer.decode(&generated_ids)?;

        // Clean up any trailing special tokens
        let clean_output = if let Some(processor) = &self.tokenizer.special_tokens_processor {
            processor.clean_up_output(&output)
        } else {
            output
        };

        Ok(clean_output)
    }
}

impl PhiTokenizer {
    /// Create a new Phi tokenizer
    fn new(tokenizer_path: PathBuf) -> Result<Self> {
        if !tokenizer_path.exists() {
            return Err(ZseiError::Llm(format!(
                "Tokenizer file not found: {}",
                tokenizer_path.display()
            )));
        }

        let inner = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| ZseiError::Llm(format!("Failed to load tokenizer: {}", e)))?;

        // Try to load special tokens
        let special_tokens_processor = Self::create_special_tokens_processor(&tokenizer_path).ok();

        Ok(Self {
            inner,
            tokenizer_path,
            special_tokens_processor,
        })
    }

    /// Create a special tokens processor
    fn create_special_tokens_processor(tokenizer_path: &Path) -> Result<SpecialTokensProcessor> {
        let special_tokens_path = tokenizer_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("special_tokens_map.json");

        let content = fs::read_to_string(&special_tokens_path)
            .map_err(|e| ZseiError::Llm(format!("Failed to read special tokens: {}", e)))?;

        let tokens: HashMap<String, String> = serde_json::from_str(&content)
            .map_err(|e| ZseiError::Llm(format!("Failed to parse special tokens: {}", e)))?;

        Ok(SpecialTokensProcessor { tokens })
    }

    /// Encode text
    fn encode(&self, text: &str) -> Result<Vec<i64>> {
        let encoding = self
            .inner
            .encode(text, false)
            .map_err(|e| ZseiError::Llm(format!("Failed to encode text: {}", e)))?;

        Ok(encoding.get_ids().iter().map(|&id| id as i64).collect())
    }

    /// Decode tokens
    fn decode(&self, tokens: &[i64]) -> Result<String> {
        // Convert i64 to u32
        let tokens_u32: Vec<u32> = tokens.iter().map(|&id| id as u32).collect();

        let text = self
            .inner
            .decode(&tokens_u32, true)
            .map_err(|e| ZseiError::Llm(format!("Failed to decode tokens: {}", e)))?;

        Ok(text)
    }

    /// Get EOS token ID
    fn eos_token_id(&self) -> i64 {
        // For Phi-4 Mini, the EOS token is typically 50256
        // but we should try to get it from the tokenizer if possible
        if let Some(processor) = &self.special_tokens_processor {
            if let Some(eos_token) = processor.tokens.get("eos_token") {
                if let Ok(encoding) = self.inner.encode(eos_token.as_str(), false) {
                    if !encoding.get_ids().is_empty() {
                        return encoding.get_ids()[0] as i64;
                    }
                }
            }
        }

        // Fallback to default EOS token ID
        50256
    }
}

impl SpecialTokensProcessor {
    /// Clean up output by removing special tokens
    fn clean_up_output(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Remove special tokens from the end
        for token in self.tokens.values() {
            if result.ends_with(token) {
                result = result[..result.len() - token.len()].to_string();
            }
        }

        // Remove Phi-4 specific tokens
        if result.contains("<|im_end|>") {
            result = result
                .split("<|im_end|>")
                .next()
                .unwrap_or(&result)
                .to_string();
        }

        result.trim().to_string()
    }
}

#[async_trait]
impl Model for PhiModel {
    async fn generate(&self, prompt: &str) -> Result<String> {
        self.generate_with_parameters(prompt, self.parameters.clone())
            .await
    }

    async fn generate_with_parameters(
        &self,
        prompt: &str,
        parameters: ModelParameters,
    ) -> Result<String> {
        debug!("Generating text with Phi model");

        // Preprocess
        let input_ids = self.preprocess(prompt).await?;
        let input_len = input_ids.len();

        // Generate
        let output_ids = self.generate_output(input_ids, &parameters).await?;

        // Postprocess
        let output = self.postprocess(output_ids, input_len).await?;

        Ok(output)
    }

    /// Generate embeddings for text
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        debug!("Generating embeddings with Phi model");

        // Preprocess the text to get input tokens
        let input_ids = self.preprocess(text).await?;

        // Get pipeline reference with RwLock FIRST - before creating memory_info
        let pipeline = self.pipeline.read().await;

        // Create input data vector and prepare array
        let input_data: Vec<i64> = input_ids.clone();
        let input_shape_usize = vec![1, input_ids.len()];
        let array_data = ndarray::Array::from_vec(input_data);
        let array_reshaped = match array_data.into_shape(input_shape_usize) {
            Ok(arr) => arr,
            Err(e) => return Err(ZseiError::Llm(format!("Failed to reshape array: {}", e))),
        };
        let cow_array = ndarray::CowArray::from(array_reshaped);

        // Get memory info and allocator
        let memory_info = ort::MemoryInfo::new(
            ort::AllocationDevice::CPU,
            0,
            ort::AllocatorType::Arena,
            ort::MemType::Default,
        )?;

        // Get the allocator from the session - this is a safe method
        let allocator_ptr = pipeline.session.allocator();

        // Create the tensor value with the correct types
        let input_values = vec![ort::Value::from_array(allocator_ptr, &cow_array)?];

        // Run inference
        let outputs = pipeline.session.run(input_values)?;

        // Get embedding data from output tensor
        let output_tensor = outputs[0].try_extract::<f32>()?;

        // Get dimensions
        let view = output_tensor.view();
        let dims = view.shape();

        // Validate dimensions
        if dims.len() < 2 {
            return Err(ZseiError::Llm(
                "Unexpected output shape for embeddings".to_string(),
            ));
        }

        // Determine embedding dimension based on output shape
        // The exact processing depends on your model's output format
        let embedding_dim: usize;
        let seq_len: usize;

        if dims.len() == 2 {
            // Format: [batch_size, embedding_dim]
            embedding_dim = dims[1];
            seq_len = 1;
        } else if dims.len() == 3 {
            // Format: [batch_size, seq_len, embedding_dim]
            embedding_dim = dims[2];
            seq_len = dims[1];
        } else {
            return Err(ZseiError::Llm(format!(
                "Unsupported embedding tensor shape: {:?}",
                dims
            )));
        }

        // Create embedding by extracting values
        let mut embedding = Vec::with_capacity(embedding_dim);

        if dims.len() == 2 {
            // Directly copy embedding from output
            for i in 0..embedding_dim {
                embedding.push(view[[0, i]]);
            }
        } else {
            // Average across sequence dimension
            for i in 0..embedding_dim {
                let mut sum = 0.0;
                for j in 0..seq_len {
                    sum += view[[0, j, i]];
                }
                embedding.push(sum / (seq_len as f32));
            }
        }

        // Normalize embedding to unit length
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 1e-6 {
            for x in &mut embedding {
                *x /= norm;
            }
        }

        Ok(embedding)
    }
}

impl From<ort::OrtError> for ZseiError {
    fn from(err: ort::OrtError) -> Self {
        ZseiError::Llm(format!("ONNX Runtime error: {}", err))
    }
}

// Add appropriate From implementations for other error types
impl From<tokenizers::Error> for ZseiError {
    fn from(err: tokenizers::Error) -> Self {
        ZseiError::Llm(format!("Tokenizer error: {}", err))
    }
}
