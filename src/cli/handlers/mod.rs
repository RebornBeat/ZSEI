//! CLI command handlers for ZSEI
//!
//! This module provides handlers for the CLI commands.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info};

use crate::analyzers::{Analyzer, ProgressUpdate};
use crate::cli::commands::{AnalyzeArgs, IndexArgs, InitArgs, QueryArgs, RefactorArgs, RunArgs};
use crate::cli::ui;
use crate::core::config::Config;
use crate::core::project::Project;
use crate::errors::{Result, ZseiError};
use crate::indexing::Indexer;
use crate::query::QueryEngine;
use crate::refactor::RefactoringEngine;

/// Init command handler
pub async fn handle_init(args: InitArgs, config: Arc<Config>, project: Arc<Project>) -> Result<()> {
    ui::display_banner();
    ui::display_phase_header("Initialization Phase");

    info!("Initializing project");
    debug!("Init args: {:?}", args);

    let path = args
        .path
        .unwrap_or_else(|| config.project_root().to_path_buf());

    // Create project structure
    if args.force || !project.is_initialized()? {
        project.initialize(&path)?;
        ui::display_success(&format!("Project initialized at: {}", path.display()));
    } else {
        ui::display_info(&format!(
            "Project already initialized at: {}",
            path.display()
        ));
    }

    // Add additional paths if specified
    for additional_path in &args.additional_paths {
        project.add_reference_path(additional_path)?;
        ui::display_info(&format!(
            "Added reference path: {}",
            additional_path.display()
        ));
    }

    ui::display_success("Initialization complete!");
    Ok(())
}

/// Analyze command handler
pub async fn handle_analyze(
    args: AnalyzeArgs,
    config: Arc<Config>,
    analyzer: Arc<Analyzer>,
) -> Result<()> {
    ui::display_banner();
    ui::display_phase_header("Analysis Phase");

    info!("Starting project analysis");
    debug!("Analyze args: {:?}", args);

    let start_time = Instant::now();

    let paths = if args.paths.is_empty() {
        vec![config.project_root().to_path_buf()]
    } else {
        args.paths
    };

    let (progress_tx, progress_rx) = if args.progress {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
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
        analyzer.analyze_incremental(&paths, progress_tx).await?
    } else {
        analyzer.analyze_full(&paths, progress_tx).await?
    };

    // Calculate elapsed time
    let elapsed = start_time.elapsed();
    ui::display_info(&format!(
        "Analysis completed in {}",
        crate::utils::format_duration(elapsed)
    ));

    // Output results
    if let Some(output_path) = args.output {
        result.save_to_file(&output_path)?;
        ui::display_success(&format!(
            "Analysis results saved to: {}",
            output_path.display()
        ));
    } else {
        // Print summary to console
        ui::display_analysis_summary(&result);
    }

    Ok(())
}

/// Index command handler
pub async fn handle_index(
    args: IndexArgs,
    config: Arc<Config>,
    indexer: Arc<Indexer>,
) -> Result<()> {
    ui::display_banner();
    ui::display_phase_header("Indexing Phase");

    info!("Starting project indexing");
    debug!("Index args: {:?}", args);

    let start_time = Instant::now();

    let paths = if args.paths.is_empty() {
        vec![config.project_root().to_path_buf()]
    } else {
        args.paths
    };

    let (progress_tx, progress_rx) = if args.progress {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
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
        indexer.index_incremental(&paths, progress_tx).await?;
    } else {
        indexer.index_full(&paths, progress_tx).await?;
    }

    // Calculate elapsed time
    let elapsed = start_time.elapsed();
    ui::display_info(&format!(
        "Indexing completed in {}",
        crate::utils::format_duration(elapsed)
    ));

    // Save index
    if let Some(output_path) = args.output {
        indexer.save(&output_path).await?;
        ui::display_success(&format!("Index saved to: {}", output_path.display()));
    } else {
        // Save to default location
        let index_path = config.index_path()?;
        indexer.save(&index_path).await?;
        ui::display_success(&format!("Index saved to: {}", index_path.display()));
    }

    Ok(())
}

/// Query command handler
pub async fn handle_query(
    args: QueryArgs,
    config: Arc<Config>,
    query_engine: Arc<QueryEngine>,
) -> Result<()> {
    ui::display_banner();
    ui::display_phase_header("Query Phase");

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
        return Err(ZseiError::Query("No query provided".to_string()));
    };

    info!("Executing query: {}", query_text);

    let start_time = Instant::now();

    // Execute query
    let result = query_engine
        .query(&query_text, args.max_results, args.context_size)
        .await?;

    // Calculate elapsed time
    let elapsed = start_time.elapsed();
    ui::display_info(&format!(
        "Query completed in {}",
        crate::utils::format_duration(elapsed)
    ));

    // Output results
    if let Some(output_path) = args.output {
        result.save_to_file(&output_path)?;
        ui::display_success(&format!(
            "Query results saved to: {}",
            output_path.display()
        ));
    } else {
        // Display results in console
        ui::display_query_results(&result);
    }

    // If in interactive mode, continue accepting queries until exit
    if args.interactive {
        while let Some(query) = ui::get_interactive_query_or_exit()? {
            info!("Executing query: {}", query);

            let start_time = Instant::now();
            let result = query_engine
                .query(&query, args.max_results, args.context_size)
                .await?;
            let elapsed = start_time.elapsed();

            ui::display_info(&format!(
                "Query completed in {}",
                crate::utils::format_duration(elapsed)
            ));
            ui::display_query_results(&result);
        }
    }

    Ok(())
}

/// Refactor command handler
pub async fn handle_refactor(
    args: RefactorArgs,
    config: Arc<Config>,
    refactoring_engine: Arc<RefactoringEngine>,
) -> Result<()> {
    ui::display_banner();
    ui::display_phase_header("Refactoring Phase");

    debug!("Refactor args: {:?}", args);

    // Get query text
    let query_text = if let Some(query) = args.query {
        query
    } else if let Some(file_path) = args.file {
        std::fs::read_to_string(file_path)?
    } else {
        return Err(ZseiError::Refactor("No query provided".to_string()));
    };

    info!("Starting refactoring with query: {}", query_text);

    // Run build command if requested to get error output
    let build_output = if args.build {
        let build_cmd = args.build_command.clone().unwrap_or_else(|| {
            // Try to detect build command based on project
            if std::path::Path::new("Cargo.toml").exists() {
                "cargo build".to_string()
            } else if std::path::Path::new("package.json").exists() {
                "npm run build".to_string()
            } else if std::path::Path::new("CMakeLists.txt").exists() {
                "cmake --build build".to_string()
            } else {
                "make".to_string()
            }
        });

        ui::display_info(&format!("Running build command: {}", build_cmd));
        Some(ui::run_command(&build_cmd)?)
    } else {
        None
    };

    if let Some(output) = &build_output {
        ui::display_section_header("Build Output");
        println!("{}", output);
    }

    // Create refactoring branches
    ui::display_section_header(&format!("Creating {} refactoring branches", args.branches));

    let start_time = Instant::now();
    let branches = refactoring_engine
        .create_branches(&query_text, build_output.as_deref(), args.branches)
        .await?;
    let elapsed = start_time.elapsed();

    ui::display_info(&format!(
        "Branch creation completed in {}",
        crate::utils::format_duration(elapsed)
    ));

    // Display branch summaries and let user choose
    ui::display_section_header("Selecting Branch");
    let selected_branch = ui::select_refactoring_branch(&branches)?;
    info!("Selected branch: {}", selected_branch.name);

    // Apply changes or create diff files
    if args.apply {
        ui::display_section_header("Applying Changes");
        refactoring_engine.apply_branch(&selected_branch).await?;
        ui::display_success(&format!(
            "Applied changes from branch: {}",
            selected_branch.name
        ));

        // Run build to verify changes if requested
        if args.build {
            let build_cmd = args.build_command.clone().unwrap_or_else(|| {
                if std::path::Path::new("Cargo.toml").exists() {
                    "cargo build".to_string()
                } else if std::path::Path::new("package.json").exists() {
                    "npm run build".to_string()
                } else if std::path::Path::new("CMakeLists.txt").exists() {
                    "cmake --build build".to_string()
                } else {
                    "make".to_string()
                }
            });

            ui::display_info(&format!(
                "Running build command to verify changes: {}",
                build_cmd
            ));
            let output = ui::run_command(&build_cmd)?;
            ui::display_command_output(&output);
        }
    } else if args.diff {
        let diff_output_dir = args
            .diff_output
            .unwrap_or_else(|| config.project_root().join(".zsei").join("diffs"));

        ui::display_section_header("Creating Diff Files");
        refactoring_engine
            .create_diff_files(&selected_branch, &diff_output_dir)
            .await?;
        ui::display_success(&format!(
            "Created diff files in: {}",
            diff_output_dir.display()
        ));
    } else {
        // Just display the changes
        ui::display_refactoring_changes(&selected_branch);
    }

    ui::display_success("Refactoring complete!");
    Ok(())
}

/// Run command handler - the main analysis-refactor loop
pub async fn handle_run(
    args: RunArgs,
    config: Arc<Config>,
    query_engine: Arc<QueryEngine>,
    refactoring_engine: Arc<RefactoringEngine>,
) -> Result<()> {
    ui::display_banner();
    ui::display_phase_header("ZSEI Analysis-Refactor Loop");

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
        ui::display_section_header(&format!(
            "Iteration {}/{}",
            iteration,
            if max_iterations == std::usize::MAX {
                "∞".to_string()
            } else {
                max_iterations.to_string()
            }
        ));

        // PHASE 1: Prompt Analysis
        ui::display_phase_header("Phase 1: Prompt Analysis");

        // Run build command if requested to get error output
        let build_output = if args.build {
            let build_cmd = args.build_command.clone().unwrap_or_else(|| {
                if std::path::Path::new("Cargo.toml").exists() {
                    "cargo build".to_string()
                } else if std::path::Path::new("package.json").exists() {
                    "npm run build".to_string()
                } else if std::path::Path::new("CMakeLists.txt").exists() {
                    "cmake --build build".to_string()
                } else {
                    "make".to_string()
                }
            });

            ui::display_info(&format!("Running build command: {}", build_cmd));
            Some(ui::run_command(&build_cmd)?)
        } else {
            None
        };

        if let Some(output) = &build_output {
            ui::display_section_header("Build Output");
            println!("{}", output);
        }

        // If we have build output, append it to the query
        let query_with_errors = if let Some(output) = &build_output {
            format!("{}\n\nBuild output:\n{}", current_query, output)
        } else {
            current_query.clone()
        };

        // Execute query to find relevant code - using Phase 1 LLM
        ui::display_info("Searching for relevant code...");
        let query_result = query_engine
            .query(&query_with_errors, 50, args.context_size)
            .await?;

        // Show results and ask user to proceed
        ui::display_query_results(&query_result);
        if !ui::confirm("Proceed to Phase 2 (Refactoring)?")? {
            if ui::confirm("Export query results and exit?")? {
                let export_path = ui::get_file_path("Enter export path:")?;
                query_result.save_to_file(&export_path)?;
                ui::display_success(&format!(
                    "Query results exported to: {}",
                    export_path.display()
                ));
            }
            break;
        }

        // PHASE 2: Refactoring
        ui::display_phase_header("Phase 2: Refactoring");

        // Create refactoring branches - using Phase 2 LLM
        ui::display_info(&format!(
            "Creating {} refactoring branches...",
            args.branches
        ));
        let branches = refactoring_engine
            .create_branches(&query_with_errors, build_output.as_deref(), args.branches)
            .await?;

        // Display branch summaries and let user choose
        ui::display_section_header("Select Refactoring Branch");
        let selected_branch = ui::select_refactoring_branch(&branches)?;
        info!("Selected branch: {}", selected_branch.name);

        // Apply changes if requested
        if args.apply {
            ui::display_section_header("Applying Changes");
            refactoring_engine.apply_branch(&selected_branch).await?;
            ui::display_success(&format!(
                "Applied changes from branch: {}",
                selected_branch.name
            ));

            // Run build to verify changes
            if args.build {
                let build_cmd = args.build_command.clone().unwrap_or_else(|| {
                    if std::path::Path::new("Cargo.toml").exists() {
                        "cargo build".to_string()
                    } else if std::path::Path::new("package.json").exists() {
                        "npm run build".to_string()
                    } else if std::path::Path::new("CMakeLists.txt").exists() {
                        "cmake --build build".to_string()
                    } else {
                        "make".to_string()
                    }
                });

                ui::display_info(&format!(
                    "Running build command to verify changes: {}",
                    build_cmd
                ));
                let new_output = ui::run_command(&build_cmd)?;
                ui::display_command_output(&new_output);

                // Update query with new build output for next iteration
                if new_output.contains("error") {
                    current_query = format!(
                        "{}\n\nPrevious iteration resolved some issues, but new build shows:\n{}",
                        initial_query, new_output
                    );

                    ui::display_info("Some errors still remain. Continuing to next iteration.");
                } else {
                    ui::display_success("Build successful! All issues resolved.");
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

    ui::display_success(&format!(
        "Analysis-refactor loop completed after {} iterations",
        iteration - 1
    ));
    Ok(())
}
