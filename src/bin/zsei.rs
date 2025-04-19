//! # ZSEI CLI
//!
//! This binary provides the command-line interface for the
//! Zero-Shot Bolted Embedding Indexer.

use clap::Parser;
use color_eyre::eyre::{Result, WrapErr};
use std::path::PathBuf;
use tracing::info;
use zsei::cli::commands::{Cli, Commands};
use zsei::core::config::Config;
use zsei::llm::LlmFactory;
use zsei::utils::logger;
use zsei::Zsei;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize error handler
    color_eyre::install()?;

    // Always display banner first
    zsei::cli::ui::display_banner();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Initialize logger
    let log_level = if cli.verbose { "debug" } else { "info" };
    logger::init(log_level)?;

    info!("ZSEI v{}", Zsei::version());

    // Load configuration
    let mut config = if let Some(config_path) = &cli.config {
        Config::from_file(config_path)
            .wrap_err_with(|| format!("Failed to load config from {}", config_path.display()))?
    } else {
        // Use current directory as project root if not specified
        let project_root = cli
            .project
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        // Try to load config from default location or create new
        let config_path = project_root.join(".zsei").join("config.toml");
        if config_path.exists() {
            Config::from_file(&config_path).unwrap_or_else(|_| {
                let mut config = Config::default();
                config.set_project_root(project_root);
                config
            })
        } else {
            let mut config = Config::default();
            config.set_project_root(project_root);
            config
        }
    };

    // If model path is not set yet, try to select one
    if config.llm.model_path.is_none() {
        match LlmFactory::list_available_models() {
            Ok(models) if !models.is_empty() => {
                println!("Available models:");
                for (i, (name, path)) in models.iter().enumerate() {
                    println!("  {}. {} ({})", i + 1, name, path.display());
                }

                // Set the model path
                if let Ok(selected_path) = LlmFactory::select_model_interactive() {
                    config.llm.model_path = Some(selected_path);
                    println!("Selected model: {:?}", config.llm.model_path);
                }
            }
            _ => {
                // Only show warning for commands that need a model
                match cli.command {
                    Commands::Query(_) | Commands::Refactor(_) | Commands::Run(_) => {
                        println!("Warning: No models found. Some features may not work.");
                    }
                    _ => {} // Other commands don't need a model
                }
            }
        }
    }

    // Apply CLI-provided model paths if specified
    if let Commands::Run(args) = &cli.command {
        // If the model path was provided via CLI, update the config
        if let Some(model_path) = &args.model {
            config.llm.model_path = Some(model_path.clone());
        }

        // If phase-specific model paths were provided, configure them
        if let Some(phase1_path) = &args.phase1_model {
            // Create phase1 LLM config if it doesn't exist
            if config.phase1_llm.is_none() {
                config.phase1_llm = Some(config.llm.clone());
            }

            // Update the model path
            if let Some(phase1_llm) = &mut config.phase1_llm {
                phase1_llm.model_path = Some(phase1_path.clone());
            }
        }

        if let Some(phase2_path) = &args.phase2_model {
            // Create phase2 LLM config if it doesn't exist
            if config.phase2_llm.is_none() {
                config.phase2_llm = Some(config.llm.clone());
            }

            // Update the model path
            if let Some(phase2_llm) = &mut config.phase2_llm {
                phase2_llm.model_path = Some(phase2_path.clone());
            }
        }
    }

    // Initialize ZSEI system with the configured LLMs
    let zsei = Zsei::new(config).await?;

    // Create CLI handler
    let cli_handler = zsei.create_cli_handler().await?;

    // Execute the appropriate command
    match cli.command {
        Commands::Init(args) => {
            cli_handler.handle_init(args).await?;
        }
        Commands::Analyze(args) => {
            cli_handler.handle_analyze(args).await?;
        }
        Commands::Index(args) => {
            cli_handler.handle_index(args).await?;
        }
        Commands::Query(args) => {
            cli_handler.handle_query(args).await?;
        }
        Commands::Refactor(args) => {
            cli_handler.handle_refactor(args).await?;
        }
        Commands::Run(args) => {
            cli_handler.handle_run(args).await?;
        }
    }

    Ok(())
}
