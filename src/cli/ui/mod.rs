//! CLI UI components for ZSEI
//!
//! This module provides UI components for the CLI interface.

use colored::*;
use dialoguer::{Confirm, Input, MultiSelect, Select};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tokio::sync::mpsc;

use crate::analyzers::ProgressUpdate;
use crate::errors::{Result, ZseiError};
use crate::query::QueryResult;
use crate::refactor::RefactoringBranch;

/// Display a progress bar
pub async fn display_progress_bar(mut rx: mpsc::Receiver<ProgressUpdate>, message: String) {
    let pb = ProgressBar::new(100);

    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
            .expect("Invalid progress bar template")
            .progress_chars("##-"),
    );

    pb.set_message(message);

    while let Some(update) = rx.recv().await {
        let percentage = (update.current as f64 / update.total as f64 * 100.0) as u64;
        pb.set_position(percentage);
        pb.set_message(update.message);
        pb.set_length(update.total as u64);
    }

    pb.finish_with_message("Complete".green().to_string());
}

/// Display an analysis summary
pub fn display_analysis_summary(result: &crate::analyzers::common::AnalysisResult) {
    println!("\n{}", "Analysis Summary".bold().green());
    println!("─────────────────────────────────");

    // File statistics
    println!(
        "Analyzed {} files",
        result.file_analyses.len().to_string().bold()
    );

    let languages: std::collections::HashMap<_, _> = result
        .file_analyses
        .iter()
        .map(|fa| &fa.language)
        .fold(std::collections::HashMap::new(), |mut acc, lang| {
            *acc.entry(lang).or_insert(0) += 1;
            acc
        });

    println!("Languages:");
    for (lang, count) in languages {
        println!("  - {}: {}", lang, count);
    }

    // Function statistics
    let function_count: usize = result
        .file_analyses
        .iter()
        .map(|fa| fa.functions.len())
        .sum();
    println!("Functions: {}", function_count.to_string().bold());

    // Dependency statistics
    println!(
        "Dependencies: {}",
        result.dependencies.len().to_string().bold()
    );

    // Print dependency types
    let dep_types: std::collections::HashMap<_, _> = result
        .dependencies
        .iter()
        .map(|d| &d.dependency_type)
        .fold(std::collections::HashMap::new(), |mut acc, dep_type| {
            *acc.entry(dep_type).or_insert(0) += 1;
            acc
        });

    println!("Dependency types:");
    for (dep_type, count) in dep_types {
        println!("  - {:?}: {}", dep_type, count);
    }

    println!("─────────────────────────────────\n");
}

/// Display query results
pub fn display_query_results(result: &QueryResult) {
    println!("\n{}", "Query Results".bold().green());
    println!("─────────────────────────────────");

    // Print query
    println!("{}: {}", "Query".bold().blue(), result.query);

    // Print response
    println!("\n{}", "Response".bold().blue());
    println!("{}", result.response);

    // Print code snippets
    if !result.code_snippets.is_empty() {
        println!("\n{}", "Relevant Code Snippets".bold().blue());
        println!("Found {} relevant files:", result.code_snippets.len());

        for (i, snippet) in result.code_snippets.iter().enumerate() {
            println!(
                "{}. {} (score: {:.2})",
                (i + 1).to_string().bold(),
                snippet.path.display().to_string().cyan(),
                snippet.score
            );
        }
    }

    // Print suggestions
    if !result.suggestions.is_empty() {
        println!("\n{}", "Suggestions".bold().blue());

        for (i, suggestion) in result.suggestions.iter().enumerate() {
            println!(
                "{}. {}",
                (i + 1).to_string().bold(),
                suggestion.title.bold()
            );
            println!("   {}", suggestion.description);

            if let Some(path) = &suggestion.path {
                println!("   File: {}", path.display().to_string().cyan());
            }
        }
    }

    println!("─────────────────────────────────\n");
}

/// Display refactoring changes
pub fn display_refactoring_changes(branch: &RefactoringBranch) {
    println!("\n{}", "Refactoring Changes".bold().green());
    println!("─────────────────────────────────");

    // Print branch info
    println!("{}: {}", "Branch".bold().blue(), branch.name);
    println!("{}: {}", "Description".bold().blue(), branch.description);
    println!("{}: {:.2}", "Score".bold().blue(), branch.score);

    // Print changes
    println!("\n{}", "Changes".bold().blue());
    println!("Found {} changes:", branch.changes.len());

    for (i, change) in branch.changes.iter().enumerate() {
        println!(
            "{}. {} ({})",
            (i + 1).to_string().bold(),
            change.path.display().to_string().cyan(),
            format!("{:?}", change.change_type).magenta()
        );

        println!("   {}", change.description);

        // Print diff stats
        let original_lines = change.original.lines().count();
        let modified_lines = change.modified.lines().count();
        let diff = modified_lines as isize - original_lines as isize;

        let diff_str = if diff > 0 {
            format!("+{} lines", diff).green()
        } else if diff < 0 {
            format!("{} lines", diff).red()
        } else {
            "no line change".yellow()
        };

        println!(
            "   Diff: {} ({} -> {})",
            diff_str, original_lines, modified_lines
        );
    }

    println!("─────────────────────────────────\n");
}

/// Get an interactive query
pub fn get_interactive_query() -> Result<String> {
    let query: String = Input::new()
        .with_prompt("Enter your query")
        .interact()
        .map_err(|e| ZseiError::Other(format!("Failed to get input: {}", e)))?;

    Ok(query)
}

/// Get an interactive query or exit
pub fn get_interactive_query_or_exit() -> Result<Option<String>> {
    println!("\nEnter a new query (or 'exit' to quit):");

    let query: String = Input::new()
        .with_prompt(">")
        .interact()
        .map_err(|e| ZseiError::Other(format!("Failed to get input: {}", e)))?;

    if query.eq_ignore_ascii_case("exit") {
        Ok(None)
    } else {
        Ok(Some(query))
    }
}

/// Confirm an action
pub fn confirm(message: &str) -> Result<bool> {
    Confirm::new()
        .with_prompt(message)
        .interact()
        .map_err(|e| ZseiError::Other(format!("Failed to get confirmation: {}", e)))
}

/// Select a refactoring branch
pub fn select_refactoring_branch(branches: &[RefactoringBranch]) -> Result<RefactoringBranch> {
    let options: Vec<String> = branches
        .iter()
        .map(|b| format!("{} (score: {:.2})", b.name, b.score))
        .collect();

    let selection = Select::new()
        .with_prompt("Select a branch")
        .items(&options)
        .default(0)
        .interact()
        .map_err(|e| ZseiError::Other(format!("Failed to get selection: {}", e)))?;

    Ok(branches[selection].clone())
}

/// Get a file path
pub fn get_file_path(prompt: &str) -> Result<PathBuf> {
    let path: String = Input::new()
        .with_prompt(prompt)
        .interact()
        .map_err(|e| ZseiError::Other(format!("Failed to get input: {}", e)))?;

    Ok(PathBuf::from(path))
}

/// Run a command and capture output
pub fn run_command(command: &str) -> Result<String> {
    let parts: Vec<&str> = command.split_whitespace().collect();

    if parts.is_empty() {
        return Err(ZseiError::Other("Empty command".to_string()));
    }

    let cmd = parts[0];
    let args = &parts[1..];

    let output = Command::new(cmd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| ZseiError::Other(format!("Failed to run command: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let combined = if stderr.is_empty() {
        stdout
    } else if stdout.is_empty() {
        stderr
    } else {
        format!("{}\n\n{}", stdout, stderr)
    };

    Ok(combined)
}

/// Display command output
pub fn display_command_output(output: &str) {
    println!("\n{}", "Command Output".bold().blue());
    println!("─────────────────────────────────");
    println!("{}", output);
    println!("─────────────────────────────────\n");
}

/// Select multiple files
pub fn select_multiple_files(files: &[PathBuf], prompt: &str) -> Result<Vec<PathBuf>> {
    let options: Vec<String> = files.iter().map(|p| p.display().to_string()).collect();

    let indices = MultiSelect::new()
        .with_prompt(prompt)
        .items(&options)
        .interact()
        .map_err(|e| ZseiError::Other(format!("Failed to get selection: {}", e)))?;

    let selected: Vec<PathBuf> = indices.into_iter().map(|i| files[i].clone()).collect();

    Ok(selected)
}

/// Display banner
pub fn display_banner() {
    let version = env!("CARGO_PKG_VERSION");

    println!(
        "{}",
        format!(
            r#"
  ______  _____  ______  _____
 |___  / / ____||  ____||_   _|
    / / | (___  | |__     | |
   / /   \___ \ |  __|    | |
  / /________) || |____  _| |_
 /______|_____/ |______||_____|

 Zero-Shot Embedding Indexer                     v{}"#,
            version
        )
        .bright_blue()
    );
}

/// Display phase header
pub fn display_phase_header(phase: &str) {
    println!("\n{}", phase.bold().green().underline());
}

/// Display section header
pub fn display_section_header(section: &str) {
    println!("\n{}", section.bold().yellow());
}

/// Wait for user to press enter
pub fn press_enter_to_continue() -> Result<()> {
    println!("\nPress Enter to continue...");
    let _: String = Input::new()
        .with_prompt("")
        .allow_empty(true)
        .interact()
        .map_err(|e| ZseiError::Other(format!("Failed to get input: {}", e)))?;

    Ok(())
}

/// Display error
pub fn display_error(error: &str) {
    eprintln!("{}: {}", "Error".bold().red(), error);
}

/// Display warning
pub fn display_warning(warning: &str) {
    eprintln!("{}: {}", "Warning".bold().yellow(), warning);
}

/// Display success
pub fn display_success(message: &str) {
    println!("{}: {}", "Success".bold().green(), message);
}

/// Display info
pub fn display_info(message: &str) {
    println!("{}: {}", "Info".bold().blue(), message);
}

/// Show a menu with multiple options
pub fn show_menu(title: &str, options: &[&str]) -> Result<usize> {
    let selection = Select::new()
        .with_prompt(title)
        .items(options)
        .default(0)
        .interact()
        .map_err(|e| ZseiError::Other(format!("Failed to get selection: {}", e)))?;

    Ok(selection)
}

/// Get a numeric input with min/max validation
pub fn get_numeric_input<T>(prompt: &str, min: T, max: T) -> Result<T>
where
    T: std::str::FromStr + std::cmp::PartialOrd + std::fmt::Display,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    loop {
        let input: String = Input::new()
            .with_prompt(prompt)
            .interact()
            .map_err(|e| ZseiError::Other(format!("Failed to get input: {}", e)))?;

        match input.parse::<T>() {
            Ok(value) => {
                if value < min || value > max {
                    println!("Value must be between {} and {}", min, max);
                    continue;
                }
                return Ok(value);
            }
            Err(e) => {
                println!("Invalid number: {}", e);
                continue;
            }
        }
    }
}

/// Display a diff between two strings
pub fn display_diff(original: &str, modified: &str, path: &str) {
    println!("\n{}", "Diff".bold().blue());
    println!("─────────────────────────────────");

    let original_lines: Vec<&str> = original.lines().collect();
    let modified_lines: Vec<&str> = modified.lines().collect();

    println!("File: {}", path.cyan());

    // Find differences
    let mut i = 0;
    let mut j = 0;

    while i < original_lines.len() || j < modified_lines.len() {
        if i < original_lines.len()
            && j < modified_lines.len()
            && original_lines[i] == modified_lines[j]
        {
            // Same line
            i += 1;
            j += 1;
        } else {
            // Different lines - find next matching line
            let mut found_match = false;
            let search_range = 5; // Look ahead up to 5 lines

            for offset_i in 0..search_range {
                if i + offset_i >= original_lines.len() {
                    break;
                }

                for offset_j in 0..search_range {
                    if j + offset_j >= modified_lines.len() {
                        break;
                    }

                    if original_lines[i + offset_i] == modified_lines[j + offset_j] {
                        // Found match - print deleted and added lines
                        for k in 0..offset_i {
                            println!("{} {}", "-".red(), original_lines[i + k]);
                        }

                        for k in 0..offset_j {
                            println!("{} {}", "+".green(), modified_lines[j + k]);
                        }

                        i += offset_i + 1;
                        j += offset_j + 1;
                        found_match = true;
                        break;
                    }
                }

                if found_match {
                    break;
                }
            }

            if !found_match {
                // No match found - just print current line
                if i < original_lines.len() {
                    println!("{} {}", "-".red(), original_lines[i]);
                    i += 1;
                }

                if j < modified_lines.len() {
                    println!("{} {}", "+".green(), modified_lines[j]);
                    j += 1;
                }
            }
        }
    }

    println!("─────────────────────────────────\n");
}
