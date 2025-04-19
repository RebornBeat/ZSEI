//! Code analyzers for ZSEI
//!
//! This module provides code analyzers for different programming languages.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info};

pub mod common;
pub mod rust;

use crate::core::config::Config;
use crate::core::project::{Project, ProjectStructure};
use crate::errors::{Result, ZseiError};
use common::{AnalysisResult, CodeGraph, Dependency, DependencyType, FileAnalysis};

/// Progress update
#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    /// Current item
    pub current: usize,

    /// Total items
    pub total: usize,

    /// Current item name
    pub current_item: String,

    /// Status message
    pub message: String,
}

/// Analyzer trait
#[async_trait]
pub trait LanguageAnalyzer: Send + Sync {
    /// Get the language name
    fn language_name(&self) -> &'static str;

    /// Get supported file extensions
    fn supported_extensions(&self) -> &[&'static str];

    /// Check if a file is supported
    fn is_supported(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            if let Some(ext_str) = ext.to_str() {
                return self.supported_extensions().contains(&ext_str);
            }
        }
        false
    }

    /// Analyze a file
    async fn analyze_file(&self, path: &Path) -> Result<FileAnalysis>;

    /// Extract dependencies from a file
    async fn extract_dependencies(&self, path: &Path) -> Result<Vec<Dependency>>;
}

/// Main analyzer struct
pub struct Analyzer {
    /// Configuration
    config: Arc<Config>,

    /// Language analyzers
    language_analyzers: Vec<Box<dyn LanguageAnalyzer>>,
}

impl Analyzer {
    /// Create a new analyzer
    pub fn new(config: Arc<Config>) -> Self {
        let mut language_analyzers: Vec<Box<dyn LanguageAnalyzer>> = Vec::new();

        // Add Rust analyzer
        language_analyzers.push(Box::new(rust::RustAnalyzer::new(config.clone())));

        // Add more language analyzers here as they are implemented

        Self {
            config,
            language_analyzers,
        }
    }

    /// Get the appropriate language analyzer for a file
    pub fn get_analyzer_for_file(&self, path: &Path) -> Option<&dyn LanguageAnalyzer> {
        for analyzer in &self.language_analyzers {
            if analyzer.is_supported(path) {
                return Some(analyzer.as_ref());
            }
        }
        None
    }

    /// Analyze files (full analysis)
    pub async fn analyze_full(
        &self,
        paths: &[PathBuf],
        progress_tx: Option<mpsc::Sender<ProgressUpdate>>,
    ) -> Result<AnalysisResult> {
        info!("Starting full analysis");

        let project = Project::new(self.config.clone())?;
        let files = self.collect_files_to_analyze(paths, false)?;

        debug!("Found {} files to analyze", files.len());

        self.analyze_files(files, progress_tx).await
    }

    /// Analyze files (incremental analysis)
    pub async fn analyze_incremental(
        &self,
        paths: &[PathBuf],
        progress_tx: Option<mpsc::Sender<ProgressUpdate>>,
    ) -> Result<AnalysisResult> {
        info!("Starting incremental analysis");

        let project = Project::new(self.config.clone())?;
        let files = self.collect_files_to_analyze(paths, true)?;

        debug!("Found {} files to analyze", files.len());

        self.analyze_files(files, progress_tx).await
    }

    /// Collect files to analyze
    fn collect_files_to_analyze(
        &self,
        paths: &[PathBuf],
        incremental: bool,
    ) -> Result<Vec<PathBuf>> {
        let project = Project::new(self.config.clone())?;

        let mut files = if paths.is_empty() {
            // Get all files in the project
            project.get_files_to_analyze(incremental)?
        } else {
            // Collect files from the specified paths
            let mut files = Vec::new();

            for path in paths {
                if path.is_file() {
                    if self.is_supported_file(path) {
                        files.push(path.to_path_buf());
                    }
                } else if path.is_dir() {
                    // Walk directory and collect files
                    let mut dir_files = self.collect_files_from_dir(path)?;
                    files.append(&mut dir_files);
                }
            }

            files
        };

        // Apply incremental filtering if requested
        if incremental {
            files = project.get_files_to_analyze(true)?;
        }

        Ok(files)
    }

    /// Check if a file is supported by any analyzer
    fn is_supported_file(&self, path: &Path) -> bool {
        for analyzer in &self.language_analyzers {
            if analyzer.is_supported(path) {
                return true;
            }
        }
        false
    }

    /// Collect files from a directory
    fn collect_files_from_dir(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        for entry in ignore::Walk::new(dir) {
            match entry {
                Ok(entry) => {
                    let path = entry.path().to_path_buf();
                    if path.is_file() && self.is_supported_file(&path) {
                        files.push(path);
                    }
                }
                Err(err) => {
                    debug!("Error walking directory: {}", err);
                }
            }
        }

        Ok(files)
    }

    /// Analyze files and build dependency graph
    async fn analyze_files(
        &self,
        files: Vec<PathBuf>,
        progress_tx: Option<mpsc::Sender<ProgressUpdate>>,
    ) -> Result<AnalysisResult> {
        let mut file_analyses = Vec::new();
        let mut dependencies = Vec::new();
        let total_files = files.len();

        for (i, path) in files.iter().enumerate() {
            // Send progress update
            if let Some(tx) = &progress_tx {
                let update = ProgressUpdate {
                    current: i + 1,
                    total: total_files,
                    current_item: path.display().to_string(),
                    message: format!("Analyzing {}", path.display()),
                };

                if let Err(e) = tx.send(update).await {
                    debug!("Failed to send progress update: {}", e);
                }
            }

            // Get analyzer for file
            if let Some(analyzer) = self.get_analyzer_for_file(path) {
                debug!("Analyzing file: {}", path.display());

                // Analyze file
                match analyzer.analyze_file(path).await {
                    Ok(file_analysis) => {
                        file_analyses.push(file_analysis);
                    }
                    Err(e) => {
                        debug!("Error analyzing file {}: {}", path.display(), e);
                    }
                }

                // Extract dependencies
                match analyzer.extract_dependencies(path).await {
                    Ok(file_deps) => {
                        dependencies.extend(file_deps);
                    }
                    Err(e) => {
                        debug!(
                            "Error extracting dependencies from {}: {}",
                            path.display(),
                            e
                        );
                    }
                }
            }
        }

        // Build dependency graph
        let graph = CodeGraph::new(&dependencies);

        // Build analysis result
        let result = AnalysisResult {
            file_analyses,
            dependencies,
            graph,
            project_structure: None, // Will be filled later if needed
        };

        Ok(result)
    }
}
