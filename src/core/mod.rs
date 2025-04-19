//! Core components for ZSEI
//!
//! This module provides the core components for the Zero-Shot
//! Bolted Embedding Indexer, including configuration management and project
//! structure handling.

pub mod config;
pub mod project;

use std::sync::Arc;

use crate::analyzers::Analyzer;
use crate::errors::Result;
use config::Config;
use project::Project;

/// Initialize the core system
pub async fn initialize(config: Arc<Config>) -> Result<(Arc<Project>, Arc<Analyzer>)> {
    // Create project
    let project = Project::new(config.clone())?;
    let project = Arc::new(project);

    // Initialize project if needed
    if !project.is_initialized()? {
        project.initialize(config.project_root())?;
    }

    // Create analyzer
    let analyzer = Arc::new(Analyzer::new(config.clone()));

    Ok((project, analyzer))
}
