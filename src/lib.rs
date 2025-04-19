//! # Zero-Shot Bolted Embedding Indexer (ZSEI)
//!
//! A revolutionary multi-modal analysis and search framework that combines precise
//! structural analysis with Zero-Shot Bolted Embedding for comprehensive
//! understanding across code, images, audio, and video content.
//!
//! This library provides tools to analyze, index, search, and optimize code
//! with future extensibility for other modalities.

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms, clippy::all)]

use std::sync::Arc;

/// Code analyzers for different languages
pub mod analyzers;

/// CLI interface
pub mod cli;

/// Core system components
pub mod core;

/// Embedding generation
pub mod embedding;

/// Indexing and storage
pub mod indexing;

/// LLM integration
pub mod llm;

/// Query processing
pub mod query;

/// Code refactoring and optimization
pub mod refactor;

/// Utility functions
pub mod utils;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library errors
pub mod errors {
    use thiserror::Error;

    /// Main error type for the library
    #[derive(Error, Debug)]
    pub enum ZseiError {
        /// IO errors
        #[error("IO error: {0}")]
        Io(#[from] std::io::Error),

        /// Analyzer errors
        #[error("Analyzer error: {0}")]
        Analyzer(String),

        /// Embedding errors
        #[error("Embedding error: {0}")]
        Embedding(String),

        /// Indexing errors
        #[error("Indexing error: {0}")]
        Indexing(String),

        /// LLM errors
        #[error("LLM error: {0}")]
        Llm(String),

        /// Query errors
        #[error("Query error: {0}")]
        Query(String),

        /// Refactoring errors
        #[error("Refactoring error: {0}")]
        Refactor(String),

        /// Configuration errors
        #[error("Configuration error: {0}")]
        Config(String),

        /// Serialization errors
        #[error("Serialization error: {0}")]
        Serialization(#[from] serde_json::Error),

        /// Other errors
        #[error("Other error: {0}")]
        Other(String),
    }

    /// Result type alias using ZseiError
    pub type Result<T> = std::result::Result<T, ZseiError>;
}

/// Re-export commonly used types
pub use errors::{Result, ZseiError};

/// Core ZSEI system struct
pub struct Zsei {
    /// Project configuration
    pub config: Arc<core::config::Config>,

    /// Project
    pub project: Arc<core::project::Project>,

    /// Analyzer
    pub analyzer: Arc<analyzers::Analyzer>,

    /// Indexer
    pub indexer: Arc<indexing::Indexer>,

    /// Query engine (Phase 1)
    pub query_engine: Arc<query::QueryEngine>,

    /// Refactoring engine (Phase 2)
    pub refactoring_engine: Arc<refactor::RefactoringEngine>,
}

impl Zsei {
    /// Create a new ZSEI instance
    pub async fn new(config: core::config::Config) -> Result<Self> {
        let config = Arc::new(config);

        // Initialize core components
        let (project, analyzer) = core::initialize(config.clone()).await?;

        // Initialize LLMs
        let default_llm = llm::LlmFactory::create_model(config.llm.clone()).await?;

        // Initialize phase-specific LLMs if configured
        let phase1_llm = if config.phase1_llm.is_some() {
            llm::LlmFactory::create_model(config.get_phase1_llm_config().clone()).await?
        } else {
            default_llm.clone()
        };

        let phase2_llm = if config.phase2_llm.is_some() {
            llm::LlmFactory::create_model(config.get_phase2_llm_config().clone()).await?
        } else {
            default_llm.clone()
        };

        // Initialize components
        let indexer = Arc::new(indexing::Indexer::new(
            config.clone(),
            analyzer.clone(),
            default_llm.clone(),
        ));

        let query_engine = Arc::new(query::QueryEngine::new(
            config.clone(),
            indexer.clone(),
            phase1_llm,
        ));

        let refactoring_engine = Arc::new(refactor::RefactoringEngine::new(
            config.clone(),
            query_engine.clone(),
            phase2_llm,
        ));

        Ok(Self {
            config,
            project,
            analyzer,
            indexer,
            query_engine,
            refactoring_engine,
        })
    }

    /// Initialize the system with default configuration
    pub async fn init() -> Result<Self> {
        let config = core::config::Config::default();
        Self::new(config).await
    }

    /// Initialize the system from a configuration file
    pub async fn from_file(path: &std::path::Path) -> Result<Self> {
        let config = core::config::Config::from_file(path)?;
        Self::new(config).await
    }

    /// Get the system configuration
    pub fn config(&self) -> &core::config::Config {
        &self.config
    }

    /// Get the project
    pub fn project(&self) -> &Arc<core::project::Project> {
        &self.project
    }

    /// Get the analyzer
    pub fn analyzer(&self) -> &Arc<analyzers::Analyzer> {
        &self.analyzer
    }

    /// Get the indexer
    pub fn indexer(&self) -> &Arc<indexing::Indexer> {
        &self.indexer
    }

    /// Get the query engine (Phase 1)
    pub fn query_engine(&self) -> &Arc<query::QueryEngine> {
        &self.query_engine
    }

    /// Get the refactoring engine (Phase 2)
    pub fn refactoring_engine(&self) -> &Arc<refactor::RefactoringEngine> {
        &self.refactoring_engine
    }

    /// Create a CLI handler
    pub async fn create_cli_handler(&self) -> Result<cli::CliHandler> {
        cli::create_cli_handler(
            self.config.clone(),
            self.project.clone(),
            self.analyzer.clone(),
            self.indexer.clone(),
            self.query_engine.clone(),
            self.refactoring_engine.clone(),
        )
        .await
    }

    /// Version of the library
    pub fn version() -> &'static str {
        VERSION
    }
}
