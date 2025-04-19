use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// Zero-Shot Bolted Embedding Indexer (ZSEI)
///
/// A tool for analyzing, indexing, and optimizing code with AI assistance.
#[derive(Parser, Debug)]
#[command(name = "zsei")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Path to the configuration file
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Path to the project root directory
    #[arg(short, long, value_name = "DIRECTORY")]
    pub project: Option<PathBuf>,

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Subcommands
    #[command(subcommand)]
    pub command: Commands,
}

/// CLI subcommands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new project
    Init(InitArgs),

    /// Analyze a project
    Analyze(AnalyzeArgs),

    /// Index a project for searching
    Index(IndexArgs),

    /// Query the indexed project
    Query(QueryArgs),

    /// Refactor and optimize code
    Refactor(RefactorArgs),

    /// Run the full analysis-refactor loop
    Run(RunArgs),
}

/// Arguments for the init command
#[derive(Args, Debug)]
pub struct InitArgs {
    /// Path to the project root directory
    #[arg(short, long, value_name = "DIRECTORY")]
    pub path: Option<PathBuf>,

    /// Additional project directories to index
    #[arg(short, long, value_name = "DIRECTORIES")]
    pub additional_paths: Vec<PathBuf>,

    /// Force re-initialization even if already initialized
    #[arg(short, long)]
    pub force: bool,
}

/// Arguments for the analyze command
#[derive(Args, Debug)]
pub struct AnalyzeArgs {
    /// Specific files or directories to analyze
    #[arg(value_name = "PATHS")]
    pub paths: Vec<PathBuf>,

    /// Programming languages to analyze (defaults to all)
    #[arg(short, long, value_name = "LANGUAGES")]
    pub languages: Vec<String>,

    /// Output file for the analysis report
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Show progress during analysis
    #[arg(long)]
    pub progress: bool,

    /// Run incremental analysis only on changed files
    #[arg(short, long)]
    pub incremental: bool,
}

/// Arguments for the index command
#[derive(Args, Debug)]
pub struct IndexArgs {
    /// Specific files or directories to index
    #[arg(value_name = "PATHS")]
    pub paths: Vec<PathBuf>,

    /// Programming languages to index (defaults to all)
    #[arg(short, long, value_name = "LANGUAGES")]
    pub languages: Vec<String>,

    /// Output directory for the index
    #[arg(short, long, value_name = "DIRECTORY")]
    pub output: Option<PathBuf>,

    /// Show progress during indexing
    #[arg(long)]
    pub progress: bool,

    /// Run incremental indexing only on changed files
    #[arg(short, long)]
    pub incremental: bool,
}

/// Arguments for the query command
#[derive(Args, Debug)]
pub struct QueryArgs {
    /// The query to run
    #[arg(value_name = "QUERY")]
    pub query: Option<String>,

    /// Run in interactive mode
    #[arg(short, long)]
    pub interactive: bool,

    /// Path to the query file
    #[arg(short, long, value_name = "FILE")]
    pub file: Option<PathBuf>,

    /// Maximum number of results to return
    #[arg(short, long, default_value = "10")]
    pub max_results: usize,

    /// Output file for the query results
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Maximum context size for the query (in tokens)
    #[arg(long, default_value = "100000")]
    pub context_size: usize,
}

/// Arguments for the refactor command
#[derive(Args, Debug)]
pub struct RefactorArgs {
    /// The query or issue to address
    #[arg(value_name = "QUERY")]
    pub query: Option<String>,

    /// Path to the query file
    #[arg(short, long, value_name = "FILE")]
    pub file: Option<PathBuf>,

    /// Number of branch versions to create
    #[arg(short, long, default_value = "5")]
    pub branches: usize,

    /// Apply changes directly to the codebase
    #[arg(short, long)]
    pub apply: bool,

    /// Run code build command to verify changes
    #[arg(short, long)]
    pub build: bool,

    /// Custom build command to run
    #[arg(long, value_name = "COMMAND")]
    pub build_command: Option<String>,

    /// Create diff files instead of applying changes
    #[arg(long)]
    pub diff: bool,

    /// Output directory for diff files
    #[arg(long, value_name = "DIRECTORY")]
    pub diff_output: Option<PathBuf>,
}

/// Arguments for the run command
#[derive(Args, Debug)]
pub struct RunArgs {
    /// Path to the LLM model
    #[arg(short, long, value_name = "FILE")]
    pub model: Option<PathBuf>,

    /// Path to the Phase 1 LLM model
    #[arg(long, value_name = "FILE")]
    pub phase1_model: Option<PathBuf>,

    /// Path to the Phase 2 LLM model
    #[arg(long, value_name = "FILE")]
    pub phase2_model: Option<PathBuf>,

    /// The query or issue to address
    #[arg(value_name = "QUERY")]
    pub query: Option<String>,

    /// Path to the query file
    #[arg(short, long, value_name = "FILE")]
    pub file: Option<PathBuf>,

    /// Number of iterations to run
    #[arg(short, long)]
    pub iterations: Option<usize>,

    /// Number of branch versions to create per iteration
    #[arg(short, long, default_value = "5")]
    pub branches: usize,

    /// Apply changes directly to the codebase
    #[arg(short, long)]
    pub apply: bool,

    /// Run code build command to verify changes
    #[arg(short, long)]
    pub build: bool,

    /// Custom build command to run
    #[arg(long, value_name = "COMMAND")]
    pub build_command: Option<String>,

    /// Maximum context size for the query (in tokens)
    #[arg(long, default_value = "100000")]
    pub context_size: usize,
}
