//! CLI implementation for ZSEI
//!
//! This module provides the command-line interface for interacting with the
//! Zero-Shot Bolted Embedding Indexer.

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info};

pub mod commands;
pub mod handlers;
pub mod ui;

use crate::analyzers::Analyzer;
use crate::core::config::Config;
use crate::core::project::Project;
use crate::errors::Result;
use crate::indexing::Indexer;
use crate::query::QueryEngine;
use crate::refactor::RefactoringEngine;
use commands::*;

/// CLI handler struct
pub struct CliHandler {
    /// Project reference
    project: Arc<Project>,

    /// Configuration reference
    config: Arc<Config>,

    /// Analyzer reference
    analyzer: Arc<Analyzer>,

    /// Indexer reference
    indexer: Arc<Indexer>,

    /// Query engine reference
    query_engine: Arc<QueryEngine>,

    /// Refactoring engine reference
    refactoring_engine: Arc<RefactoringEngine>,
}

impl CliHandler {
    /// Create a new CLI handler
    pub fn new(
        project: Arc<Project>,
        config: Arc<Config>,
        analyzer: Arc<Analyzer>,
        indexer: Arc<Indexer>,
        query_engine: Arc<QueryEngine>,
        refactoring_engine: Arc<RefactoringEngine>,
    ) -> Self {
        Self {
            project,
            config,
            analyzer,
            indexer,
            query_engine,
            refactoring_engine,
        }
    }

    /// Handle the init command
    pub async fn handle_init(&self, args: InitArgs) -> Result<()> {
        info!("Initializing project");
        debug!("Init args: {:?}", args);

        let path = args
            .path
            .unwrap_or_else(|| self.config.project_root().to_path_buf());

        // Create project structure
        if args.force || !self.project.is_initialized()? {
            self.project.initialize(&path)?;
            info!("Project initialized at: {}", path.display());
        } else {
            info!("Project already initialized at: {}", path.display());
        }

        // Add additional paths if specified
        for additional_path in &args.additional_paths {
            self.project.add_reference_path(additional_path)?;
            info!("Added reference path: {}", additional_path.display());
        }

        Ok(())
    }

    /// Handle the analyze command
    pub async fn handle_analyze(&self, args: AnalyzeArgs) -> Result<()> {
        info!("Starting project analysis");
        debug!("Analyze args: {:?}", args);

        let paths = if args.paths.is_empty() {
            vec![self.config.project_root().to_path_buf()]
        } else {
            args.paths
        };

        let (progress_tx, progress_rx) = if args.progress {
            let (tx, rx) = mpsc::channel(100);
            (Some(tx), Some(rx))
        } else {
            (None, None)
        };

        // Set up progress bar if requested
        if let Some(rx) = progress_rx {
            tokio::spawn(ui::display_progress_bar(rx, "Analyzing".to_string()));
        }

        // Run analysis
        let result = if args.incremental {
            self.analyzer
                .analyze_incremental(&paths, progress_tx)
                .await?
        } else {
            self.analyzer.analyze_full(&paths, progress_tx).await?
        };

        // Output results
        if let Some(output_path) = args.output {
            result.save_to_file(&output_path)?;
            info!("Analysis results saved to: {}", output_path.display());
        } else {
            // Print summary to console
            ui::display_analysis_summary(&result);
        }

        Ok(())
    }

    /// Handle the index command
    pub async fn handle_index(&self, args: IndexArgs) -> Result<()> {
        info!("Starting project indexing");
        debug!("Index args: {:?}", args);

        let paths = if args.paths.is_empty() {
            vec![self.config.project_root().to_path_buf()]
        } else {
            args.paths
        };

        let (progress_tx, progress_rx) = if args.progress {
            let (tx, rx) = mpsc::channel(100);
            (Some(tx), Some(rx))
        } else {
            (None, None)
        };

        // Set up progress bar if requested
        if let Some(rx) = progress_rx {
            tokio::spawn(ui::display_progress_bar(rx, "Indexing".to_string()));
        }

        // Run indexing
        if args.incremental {
            self.indexer.index_incremental(&paths, progress_tx).await?;
        } else {
            self.indexer.index_full(&paths, progress_tx).await?;
        }

        // Save index if output path is specified
        if let Some(output_path) = args.output {
            self.indexer.save(&output_path).await?;
            info!("Index saved to: {}", output_path.display());
        } else {
            // Save to default location
            let index_path = self.config.index_path()?;
            self.indexer.save(&index_path).await?;
            info!("Index saved to: {}", index_path.display());
        }

        Ok(())
    }

    /// Handle the query command
    pub async fn handle_query(&self, args: QueryArgs) -> Result<()> {
        debug!("Query args: {:?}", args);

        // Get query text
        let query_text = if let Some(query) = args.query {
            query
        } else if let Some(file_path) = args.file {
            std::fs::read_to_string(file_path)?
        } else if args.interactive {
            // Interactive mode
            ui::get_interactive_query()?
        } else {
            return Err(crate::ZseiError::Query("No query provided".to_string()));
        };

        info!("Executing query: {}", query_text);

        // Execute query
        let result = self
            .query_engine
            .query(&query_text, args.max_results, args.context_size)
            .await?;

        // Output results
        if let Some(output_path) = args.output {
            result.save_to_file(&output_path)?;
            info!("Query results saved to: {}", output_path.display());
        } else {
            // Display results in console
            ui::display_query_results(&result);
        }

        // If in interactive mode, continue accepting queries until exit
        if args.interactive {
            while let Some(query) = ui::get_interactive_query_or_exit()? {
                info!("Executing query: {}", query);
                let result = self
                    .query_engine
                    .query(&query, args.max_results, args.context_size)
                    .await?;
                ui::display_query_results(&result);
            }
        }

        Ok(())
    }

    /// Handle the refactor command
    pub async fn handle_refactor(&self, args: RefactorArgs) -> Result<()> {
        debug!("Refactor args: {:?}", args);

        // Get query text
        let query_text = if let Some(query) = args.query {
            query
        } else if let Some(file_path) = args.file {
            std::fs::read_to_string(file_path)?
        } else {
            return Err(crate::ZseiError::Refactor("No query provided".to_string()));
        };

        info!("Starting refactoring with query: {}", query_text);

        // Run build command if requested to get error output
        let build_output = if args.build {
            let build_cmd = args.build_command.clone().unwrap_or_else(|| {
                // Try to detect build command based on project
                if self.project.has_file("Cargo.toml") {
                    "cargo build".to_string()
                } else if self.project.has_file("package.json") {
                    "npm run build".to_string()
                } else if self.project.has_file("CMakeLists.txt") {
                    "cmake --build build".to_string()
                } else {
                    "make".to_string()
                }
            });

            Some(ui::run_command(&build_cmd)?)
        } else {
            None
        };

        // Create refactoring branches
        info!("Creating {} refactoring branches", args.branches);
        let branches = self
            .refactoring_engine
            .create_branches(&query_text, build_output.as_deref(), args.branches)
            .await?;

        // Display branch summaries and let user choose
        let selected_branch = ui::select_refactoring_branch(&branches)?;
        info!("Selected branch: {}", selected_branch.name);

        // Apply changes or create diff files
        if args.apply {
            self.refactoring_engine
                .apply_branch(&selected_branch)
                .await?;
            info!("Applied changes from branch: {}", selected_branch.name);

            // Run build to verify changes if requested
            if args.build {
                let build_cmd = args.build_command.clone().unwrap_or_else(|| {
                    if self.project.has_file("Cargo.toml") {
                        "cargo build".to_string()
                    } else if self.project.has_file("package.json") {
                        "npm run build".to_string()
                    } else if self.project.has_file("CMakeLists.txt") {
                        "cmake --build build".to_string()
                    } else {
                        "make".to_string()
                    }
                });

                info!("Running build command to verify changes: {}", build_cmd);
                let output = ui::run_command(&build_cmd)?;
                ui::display_command_output(&output);
            }
        } else if args.diff {
            let diff_output_dir = args
                .diff_output
                .unwrap_or_else(|| self.config.project_root().join(".zsei").join("diffs"));
            self.refactoring_engine
                .create_diff_files(&selected_branch, &diff_output_dir)
                .await?;
            info!("Created diff files in: {}", diff_output_dir.display());
        } else {
            // Just display the changes
            ui::display_refactoring_changes(&selected_branch);
        }

        Ok(())
    }

    /// Handle the run command - the main analysis-refactor loop
    pub async fn handle_run(&self, args: RunArgs) -> Result<()> {
        debug!("Run args: {:?}", args);

        // Get query text
        let initial_query = if let Some(query) = args.query {
            query
        } else if let Some(file_path) = args.file {
            std::fs::read_to_string(file_path)?
        } else {
            ui::get_interactive_query()?
        };

        info!(
            "Starting analysis-refactor loop with query: {}",
            initial_query
        );

        // Initialize phase tracking
        let mut current_query = initial_query.clone();
        let mut iteration = 1;
        let max_iterations = args.iterations.unwrap_or(std::usize::MAX);

        // Main loop
        while iteration <= max_iterations {
            info!(
                "Starting iteration {}/{}",
                iteration,
                if max_iterations == std::usize::MAX {
                    "∞".to_string()
                } else {
                    max_iterations.to_string()
                }
            );

            // PHASE 1: Prompt Analysis
            info!("Phase 1: Prompt Analysis");

            // Run build command if requested to get error output
            let build_output = if args.build {
                let build_cmd = args.build_command.clone().unwrap_or_else(|| {
                    if self.project.has_file("Cargo.toml") {
                        "cargo build".to_string()
                    } else if self.project.has_file("package.json") {
                        "npm run build".to_string()
                    } else if self.project.has_file("CMakeLists.txt") {
                        "cmake --build build".to_string()
                    } else {
                        "make".to_string()
                    }
                });

                Some(ui::run_command(&build_cmd)?)
            } else {
                None
            };

            // If we have build output, append it to the query
            let query_with_errors = if let Some(output) = &build_output {
                format!("{}\n\nBuild output:\n{}", current_query, output)
            } else {
                current_query.clone()
            };

            // Execute query to find relevant code
            let query_result = self
                .query_engine
                .query(&query_with_errors, 50, args.context_size)
                .await?;

            // Show results and ask user to proceed
            ui::display_query_results(&query_result);
            if !ui::confirm("Proceed to Phase 2 (Refactoring)?")? {
                if ui::confirm("Export query results and exit?")? {
                    let export_path = ui::get_file_path("Enter export path:")?;
                    query_result.save_to_file(&export_path)?;
                    info!("Query results exported to: {}", export_path.display());
                }
                break;
            }

            // PHASE 2: Refactoring
            info!("Phase 2: Refactoring");

            // Create refactoring branches
            let branches = self
                .refactoring_engine
                .create_branches(&query_with_errors, build_output.as_deref(), args.branches)
                .await?;

            // Display branch summaries and let user choose
            let selected_branch = ui::select_refactoring_branch(&branches)?;
            info!("Selected branch: {}", selected_branch.name);

            // Apply changes if requested
            if args.apply {
                self.refactoring_engine
                    .apply_branch(&selected_branch)
                    .await?;
                info!("Applied changes from branch: {}", selected_branch.name);

                // Run build to verify changes
                if args.build {
                    let build_cmd = args.build_command.clone().unwrap_or_else(|| {
                        if self.project.has_file("Cargo.toml") {
                            "cargo build".to_string()
                        } else if self.project.has_file("package.json") {
                            "npm run build".to_string()
                        } else if self.project.has_file("CMakeLists.txt") {
                            "cmake --build build".to_string()
                        } else {
                            "make".to_string()
                        }
                    });

                    info!("Running build command to verify changes: {}", build_cmd);
                    let new_output = ui::run_command(&build_cmd)?;
                    ui::display_command_output(&new_output);

                    // Update query with new build output for next iteration
                    if new_output.contains("error") {
                        current_query = format!(
                            "{}\n\nPrevious iteration resolved some issues, but new build shows:\n{}",
                            initial_query, new_output
                        );
                    } else {
                        info!("Build successful! All issues resolved.");
                        if !ui::confirm("Continue with further optimizations?")? {
                            break;
                        }
                        current_query = format!(
                            "{}\n\nAll build errors resolved. Now focus on optimizing the code.",
                            initial_query
                        );
                    }
                }
            } else {
                // Just display the changes
                ui::display_refactoring_changes(&selected_branch);

                // Ask user if they want to continue
                if !ui::confirm("Continue to next iteration?")? {
                    break;
                }
            }

            // Allow user to update query for next iteration
            if ui::confirm("Update query for next iteration?")? {
                current_query = ui::get_interactive_query()?;
            }

            iteration += 1;
        }

        info!(
            "Analysis-refactor loop completed after {} iterations",
            iteration - 1
        );
        Ok(())
    }
}

/// Create a CliHandler with all necessary components
pub async fn create_cli_handler(
    config: Arc<Config>,
    project: Arc<Project>,
    analyzer: Arc<Analyzer>,
    indexer: Arc<Indexer>,
    query_engine: Arc<QueryEngine>,
    refactoring_engine: Arc<RefactoringEngine>,
) -> Result<CliHandler> {
    let cli_handler = CliHandler::new(
        project,
        config,
        analyzer,
        indexer,
        query_engine,
        refactoring_engine,
    );

    Ok(cli_handler)
}
