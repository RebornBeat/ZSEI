//! Code refactoring for ZSEI
//!
//! This module provides code refactoring functionality for the
//! Zero-Shot Bolted Embedding Indexer.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

pub mod branches;
pub mod optimizer;

use crate::core::config::Config;
use crate::errors::{Result, ZseiError};
use crate::llm::Model;
use crate::query::{CodeSnippet, QueryEngine, QueryResult};

/// Refactoring engine
pub struct RefactoringEngine {
    /// Configuration
    config: Arc<Config>,

    /// Query engine
    query_engine: Arc<QueryEngine>,

    /// LLM
    llm: Arc<dyn Model>,

    /// Active branches
    branches: RwLock<HashMap<String, RefactoringBranch>>,
}

/// Refactoring branch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactoringBranch {
    /// Branch name
    pub name: String,

    /// Branch description
    pub description: String,

    /// Query that triggered the branch
    pub query: String,

    /// Build output
    pub build_output: Option<String>,

    /// Proposed changes
    pub changes: Vec<CodeChange>,

    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Score
    pub score: f32,

    /// Score breakdown
    pub score_breakdown: HashMap<String, f32>,
}

/// Code change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChange {
    /// File path
    pub path: PathBuf,

    /// Original content
    pub original: String,

    /// Modified content
    pub modified: String,

    /// Description of the change
    pub description: String,

    /// Change type
    pub change_type: ChangeType,
}

/// Change type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ChangeType {
    /// Add code
    Add,

    /// Modify code
    Modify,

    /// Delete code
    Delete,
}

impl RefactoringEngine {
    /// Create a new refactoring engine
    pub fn new(config: Arc<Config>, query_engine: Arc<QueryEngine>, llm: Arc<dyn Model>) -> Self {
        Self {
            config,
            query_engine,
            llm,
            branches: RwLock::new(HashMap::new()),
        }
    }

    /// Create refactoring branches for a query
    pub async fn create_branches(
        &self,
        query: &str,
        build_output: Option<&str>,
        branch_count: usize,
    ) -> Result<Vec<RefactoringBranch>> {
        info!("Creating refactoring branches for query: {}", query);

        // Execute query to find relevant code
        let max_results = 20; // Fetch more results for refactoring
        let context_size = 100000; // Use large context size
        let query_result = self
            .query_engine
            .query(query, max_results, context_size)
            .await?;

        // Create branches
        let mut branches = Vec::new();

        for i in 0..branch_count {
            let branch = self
                .create_branch(&query_result, i, query, build_output)
                .await?;

            branches.push(branch.clone());

            // Store branch
            let mut branch_map = self.branches.write().await;
            branch_map.insert(branch.name.clone(), branch);
        }

        // Sort branches by score (descending)
        branches.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(branches)
    }

    /// Create a single refactoring branch
    async fn create_branch(
        &self,
        query_result: &QueryResult,
        index: usize,
        query: &str,
        build_output: Option<&str>,
    ) -> Result<RefactoringBranch> {
        let branch_name = format!("branch-{}", index + 1);

        // Create refactoring prompt
        let prompt = self.create_refactoring_prompt(query_result, query, build_output, index)?;

        // Generate refactoring plan
        let refactoring_plan = self.llm.generate(&prompt).await?;

        // Parse code changes
        let changes = self.parse_code_changes(&refactoring_plan, query_result)?;

        // Score the branch
        let (score, score_breakdown) = self.score_branch(&changes, query_result, query).await?;

        // Create branch
        let branch = RefactoringBranch {
            name: branch_name,
            description: self.extract_branch_description(&refactoring_plan),
            query: query.to_string(),
            build_output: build_output.map(String::from),
            changes,
            created_at: chrono::Utc::now(),
            score,
            score_breakdown,
        };

        Ok(branch)
    }

    /// Create a refactoring prompt
    fn create_refactoring_prompt(
        &self,
        query_result: &QueryResult,
        query: &str,
        build_output: Option<&str>,
        branch_index: usize,
    ) -> Result<String> {
        let mut prompt = String::new();

        // Add query
        prompt.push_str(&format!("Query: {}\n\n", query));

        // Add build output if available
        if let Some(output) = build_output {
            prompt.push_str("Build Output:\n");
            prompt.push_str(output);
            prompt.push_str("\n\n");
        }

        // Add code snippets
        prompt.push_str("Code Snippets:\n\n");

        for (i, snippet) in query_result.code_snippets.iter().enumerate() {
            prompt.push_str(&format!("File {}: {}\n", i + 1, snippet.path.display()));

            if let Some(language) = &snippet.language {
                prompt.push_str(&format!("Language: {}\n", language));
            }

            prompt.push_str("```\n");
            prompt.push_str(&snippet.content);
            prompt.push_str("\n```\n\n");
        }

        // Add instructions
        prompt.push_str("Instructions:\n");
        prompt.push_str("1. Analyze the code and identify improvements based on the query\n");
        prompt.push_str("2. Create a refactoring plan with specific changes\n");

        // Customize branch behavior based on index
        match branch_index % 3 {
            0 => {
                prompt.push_str("3. Focus on fixing errors and improving correctness\n");
            }
            1 => {
                prompt.push_str("3. Focus on improving code readability and maintainability\n");
            }
            2 => {
                prompt.push_str("3. Focus on optimizing performance and efficiency\n");
            }
            _ => {}
        }

        prompt.push_str("4. Provide the modified code for each file that needs changes\n\n");

        // Add response format
        prompt.push_str("Response Format:\n");
        prompt.push_str("DESCRIPTION: [Overall description of the refactoring approach]\n\n");
        prompt.push_str("For each file that needs changes:\n");
        prompt.push_str("FILE: [file path]\n");
        prompt.push_str("CHANGE TYPE: [Add/Modify/Delete]\n");
        prompt.push_str("DESCRIPTION: [Description of the changes]\n");
        prompt.push_str("MODIFIED CODE:\n```\n[Complete modified file content]\n```\n\n");

        Ok(prompt)
    }

    /// Parse code changes from refactoring plan
    fn parse_code_changes(
        &self,
        refactoring_plan: &str,
        query_result: &QueryResult,
    ) -> Result<Vec<CodeChange>> {
        let mut changes = Vec::new();

        // Find file blocks
        let file_blocks: Vec<&str> = refactoring_plan.split("FILE:").skip(1).collect();

        for block in file_blocks {
            // Extract file path
            let path_end = block.find('\n').unwrap_or(block.len());
            let path_str = block[..path_end].trim();
            let path = PathBuf::from(path_str);

            // Extract change type
            let change_type_start = block.find("CHANGE TYPE:").unwrap_or(0);
            let change_type_end = if change_type_start > 0 {
                block[change_type_start..]
                    .find('\n')
                    .map(|pos| change_type_start + pos)
                    .unwrap_or(block.len())
            } else {
                0
            };

            let change_type = if change_type_start > 0 {
                let change_type_str =
                    block[change_type_start + "CHANGE TYPE:".len()..change_type_end].trim();
                match change_type_str {
                    "Add" => ChangeType::Add,
                    "Delete" => ChangeType::Delete,
                    _ => ChangeType::Modify,
                }
            } else {
                ChangeType::Modify
            };

            // Extract description
            let desc_start = block.find("DESCRIPTION:").unwrap_or(0);
            let desc_end = if desc_start > 0 {
                block[desc_start..]
                    .find("MODIFIED CODE:")
                    .map(|pos| desc_start + pos)
                    .unwrap_or(block.len())
            } else {
                0
            };

            let description = if desc_start > 0 && desc_end > desc_start {
                block[desc_start + "DESCRIPTION:".len()..desc_end]
                    .trim()
                    .to_string()
            } else {
                String::new()
            };

            // Extract modified code
            let code_start = block.find("MODIFIED CODE:").unwrap_or(0);
            let modified = if code_start > 0 {
                let code_text = &block[code_start + "MODIFIED CODE:".len()..];

                // Extract code between triple backticks
                if let (Some(start), Some(end)) = (code_text.find("```"), code_text.rfind("```")) {
                    code_text[start + 3..end].trim().to_string()
                } else {
                    code_text.trim().to_string()
                }
            } else {
                String::new()
            };

            // Get original content
            let original = query_result
                .code_snippets
                .iter()
                .find(|s| s.path == path)
                .map(|s| s.content.clone())
                .unwrap_or_else(|| {
                    // Try to read from file if not in snippets
                    fs::read_to_string(&path).unwrap_or_default()
                });

            changes.push(CodeChange {
                path,
                original,
                modified,
                description,
                change_type,
            });
        }

        Ok(changes)
    }

    /// Extract branch description from refactoring plan
    fn extract_branch_description(&self, refactoring_plan: &str) -> String {
        let desc_start = refactoring_plan.find("DESCRIPTION:").unwrap_or(0);
        let desc_end = if desc_start > 0 {
            refactoring_plan[desc_start..]
                .find("FILE:")
                .map(|pos| desc_start + pos)
                .unwrap_or(refactoring_plan.len())
        } else {
            0
        };

        if desc_start > 0 && desc_end > desc_start {
            refactoring_plan[desc_start + "DESCRIPTION:".len()..desc_end]
                .trim()
                .to_string()
        } else {
            "Refactoring branch".to_string()
        }
    }

    /// Score a refactoring branch
    async fn score_branch(
        &self,
        changes: &[CodeChange],
        query_result: &QueryResult,
        query: &str,
    ) -> Result<(f32, HashMap<String, f32>)> {
        let mut score_breakdown = HashMap::new();

        // Score based on number of changes (0-10 points)
        let change_count = changes.len() as f32;
        let change_score = (10.0 * (1.0 - (-change_count / 5.0).exp())).min(10.0);
        score_breakdown.insert("change_count".to_string(), change_score);

        // Score based on relevance to query (0-30 points)
        let relevance_score = self.score_relevance(changes, query).await?;
        score_breakdown.insert("relevance".to_string(), relevance_score);

        // Score based on code quality (0-30 points)
        let quality_score = self.score_code_quality(changes).await?;
        score_breakdown.insert("code_quality".to_string(), quality_score);

        // Score based on diff size (0-20 points)
        let diff_score = self.score_diff_size(changes);
        score_breakdown.insert("diff_size".to_string(), diff_score);

        // Score based on description quality (0-10 points)
        let desc_score = self.score_descriptions(changes);
        score_breakdown.insert("descriptions".to_string(), desc_score);

        // Calculate total score
        let total_score: f32 = score_breakdown.values().sum();

        // Normalize to 0-1 range
        let normalized_score = total_score / 100.0;

        Ok((normalized_score, score_breakdown))
    }

    /// Score based on relevance to query
    async fn score_relevance(&self, changes: &[CodeChange], query: &str) -> Result<f32> {
        // Create prompt to evaluate relevance
        let mut prompt = String::new();
        prompt.push_str(&format!("Query: {}\n\n", query));
        prompt.push_str("Code Changes:\n\n");

        for change in changes {
            prompt.push_str(&format!("File: {}\n", change.path.display()));
            prompt.push_str(&format!("Description: {}\n", change.description));
            prompt.push_str("Original:\n```\n");
            prompt.push_str(&change.original);
            prompt.push_str("\n```\n\n");
            prompt.push_str("Modified:\n```\n");
            prompt.push_str(&change.modified);
            prompt.push_str("\n```\n\n");
        }

        prompt.push_str(
            "Rate how relevant these changes are to the query on a scale of 0-30, where:\n",
        );
        prompt.push_str("0-10: Barely addresses the query\n");
        prompt.push_str("11-20: Partially addresses the query\n");
        prompt.push_str("21-30: Fully addresses the query\n\n");
        prompt.push_str("Provide only a number as your answer.\n");

        // Get relevance score
        let response = self.llm.generate(&prompt).await?;
        let score = self.extract_score(&response, 30.0)?;

        Ok(score)
    }

    /// Score based on code quality
    async fn score_code_quality(&self, changes: &[CodeChange]) -> Result<f32> {
        if changes.is_empty() {
            return Ok(0.0);
        }

        // Create prompt to evaluate code quality
        let mut prompt = String::new();
        prompt.push_str("Evaluate the quality of the following code changes:\n\n");

        for change in changes {
            prompt.push_str(&format!("File: {}\n", change.path.display()));
            prompt.push_str("Modified Code:\n```\n");
            prompt.push_str(&change.modified);
            prompt.push_str("\n```\n\n");
        }

        prompt.push_str("Rate the code quality on a scale of 0-30, where:\n");
        prompt.push_str("0-10: Poor quality (confusing, buggy, or inconsistent)\n");
        prompt.push_str("11-20: Average quality (functional but could be improved)\n");
        prompt.push_str("21-30: Excellent quality (clean, maintainable, and efficient)\n\n");
        prompt.push_str("Provide only a number as your answer.\n");

        // Get quality score
        let response = self.llm.generate(&prompt).await?;
        let score = self.extract_score(&response, 30.0)?;

        Ok(score)
    }

    /// Score based on diff size
    fn score_diff_size(&self, changes: &[CodeChange]) -> f32 {
        if changes.is_empty() {
            return 0.0;
        }

        let mut total_diff_ratio = 0.0;

        for change in changes {
            let original_len = change.original.len() as f32;

            if original_len == 0.0 {
                continue;
            }

            let modified_len = change.modified.len() as f32;
            let diff_ratio = ((modified_len - original_len).abs() / original_len).min(1.0);

            // Prefer moderate changes (not too small, not too large)
            let score = if diff_ratio < 0.01 {
                // Very small change
                diff_ratio * 1000.0
            } else if diff_ratio > 0.5 {
                // Very large change
                20.0 * (1.0 - diff_ratio)
            } else {
                // Moderate change
                20.0
            };

            total_diff_ratio += score;
        }

        (total_diff_ratio / changes.len() as f32).min(20.0)
    }

    /// Score based on description quality
    fn score_descriptions(&self, changes: &[CodeChange]) -> f32 {
        if changes.is_empty() {
            return 0.0;
        }

        let mut total_desc_score = 0.0;

        for change in changes {
            let desc_len = change.description.len() as f32;

            // Score based on description length and quality
            let desc_score = if desc_len == 0.0 {
                0.0
            } else if desc_len < 20.0 {
                // Very short description
                desc_len / 20.0 * 5.0
            } else if desc_len > 500.0 {
                // Very long description
                10.0 * (1.0 - ((desc_len - 500.0) / 1000.0).min(1.0))
            } else {
                // Good length description
                (desc_len / 100.0).min(1.0) * 10.0
            };

            total_desc_score += desc_score;
        }

        (total_desc_score / changes.len() as f32).min(10.0)
    }

    /// Extract a numeric score from a response
    fn extract_score(&self, response: &str, max_score: f32) -> Result<f32> {
        // Extract first number from response
        let score_str = response
            .chars()
            .skip_while(|c| !c.is_digit(10))
            .take_while(|c| c.is_digit(10) || *c == '.')
            .collect::<String>();

        match score_str.parse::<f32>() {
            Ok(score) => Ok(score.min(max_score)),
            Err(_) => Ok(max_score / 2.0), // Default to middle score if parsing fails
        }
    }

    /// Apply a branch to the codebase
    pub async fn apply_branch(&self, branch: &RefactoringBranch) -> Result<()> {
        info!("Applying branch: {}", branch.name);

        for change in &branch.changes {
            info!("Applying changes to file: {}", change.path.display());

            match change.change_type {
                ChangeType::Add | ChangeType::Modify => {
                    // Create parent directories if they don't exist
                    if let Some(parent) = change.path.parent() {
                        fs::create_dir_all(parent).map_err(|e| {
                            ZseiError::Refactor(format!("Failed to create directory: {}", e))
                        })?;
                    }

                    // Write modified content
                    fs::write(&change.path, &change.modified)
                        .map_err(|e| ZseiError::Refactor(format!("Failed to write file: {}", e)))?;
                }
                ChangeType::Delete => {
                    // Delete file
                    if change.path.exists() {
                        fs::remove_file(&change.path).map_err(|e| {
                            ZseiError::Refactor(format!("Failed to delete file: {}", e))
                        })?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Create diff files for a branch
    pub async fn create_diff_files(
        &self,
        branch: &RefactoringBranch,
        output_dir: &Path,
    ) -> Result<()> {
        info!("Creating diff files for branch: {}", branch.name);

        // Create output directory if it doesn't exist
        fs::create_dir_all(output_dir).map_err(|e| {
            ZseiError::Refactor(format!("Failed to create output directory: {}", e))
        })?;

        // Create a summary file
        let summary_path = output_dir.join(format!("{}-summary.md", branch.name));
        let mut summary = String::new();

        summary.push_str(&format!("# Refactoring Branch: {}\n\n", branch.name));
        summary.push_str(&format!("## Description\n\n{}\n\n", branch.description));
        summary.push_str(&format!("## Query\n\n{}\n\n", branch.query));

        if let Some(build_output) = &branch.build_output {
            summary.push_str("## Build Output\n\n```\n");
            summary.push_str(build_output);
            summary.push_str("\n```\n\n");
        }

        summary.push_str("## Changes\n\n");

        // Create diff files for each change
        for (i, change) in branch.changes.iter().enumerate() {
            let relative_path = change.path.to_string_lossy();
            let diff_name = format!(
                "{}-{}-{}.diff",
                branch.name,
                i,
                change
                    .path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
            );
            let diff_path = output_dir.join(&diff_name);

            // Create diff content
            let diff = self.create_diff(&change.original, &change.modified, &relative_path)?;

            // Write diff file
            fs::write(&diff_path, &diff)
                .map_err(|e| ZseiError::Refactor(format!("Failed to write diff file: {}", e)))?;

            // Add to summary
            summary.push_str(&format!(
                "### Change {}: {}\n\n",
                i + 1,
                change.path.display()
            ));
            summary.push_str(&format!("**Type**: {:?}\n\n", change.change_type));
            summary.push_str(&format!("**Description**: {}\n\n", change.description));
            summary.push_str(&format!("**Diff**: [View diff](./{diff_name})\n\n"));
        }

        // Add score information
        summary.push_str("## Scores\n\n");
        summary.push_str(&format!("**Overall Score**: {:.2}\n\n", branch.score));
        summary.push_str("**Score Breakdown**:\n\n");

        for (key, value) in &branch.score_breakdown {
            summary.push_str(&format!("- {}: {:.2}\n", key, value));
        }

        // Write summary file
        fs::write(&summary_path, summary)
            .map_err(|e| ZseiError::Refactor(format!("Failed to write summary file: {}", e)))?;

        Ok(())
    }

    /// Create a unified diff between two strings
    fn create_diff(&self, a: &str, b: &str, path: &str) -> Result<String> {
        let a_lines: Vec<&str> = a.lines().collect();
        let b_lines: Vec<&str> = b.lines().collect();

        // Create simple unified diff
        let mut diff = String::new();

        diff.push_str(&format!("--- {}\n", path));
        diff.push_str(&format!("+++ {}\n", path));

        // Find common prefix and suffix
        let mut prefix_len = 0;
        let max_prefix = std::cmp::min(a_lines.len(), b_lines.len());

        while prefix_len < max_prefix && a_lines[prefix_len] == b_lines[prefix_len] {
            prefix_len += 1;
        }

        let mut suffix_len = 0;
        let max_suffix = std::cmp::min(a_lines.len() - prefix_len, b_lines.len() - prefix_len);

        while suffix_len < max_suffix
            && a_lines[a_lines.len() - 1 - suffix_len] == b_lines[b_lines.len() - 1 - suffix_len]
        {
            suffix_len += 1;
        }

        // Calculate hunk range
        let a_start = if prefix_len > 3 { prefix_len - 3 } else { 0 };
        let a_end = if a_lines.len() > suffix_len + 3 {
            a_lines.len() - suffix_len + 3
        } else {
            a_lines.len()
        };
        let b_start = if prefix_len > 3 { prefix_len - 3 } else { 0 };
        let b_end = if b_lines.len() > suffix_len + 3 {
            b_lines.len() - suffix_len + 3
        } else {
            b_lines.len()
        };

        // Create hunk header
        diff.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            a_start + 1,
            a_end - a_start,
            b_start + 1,
            b_end - b_start
        ));

        // Add context lines before diff
        for i in a_start..prefix_len {
            diff.push_str(&format!(" {}\n", a_lines[i]));
        }

        // Add diff lines
        let a_diff_end = a_lines.len() - suffix_len;
        let b_diff_end = b_lines.len() - suffix_len;

        for i in prefix_len..a_diff_end {
            diff.push_str(&format!("-{}\n", a_lines[i]));
        }

        for i in prefix_len..b_diff_end {
            diff.push_str(&format!("+{}\n", b_lines[i]));
        }

        // Add context lines after diff
        for i in 0..suffix_len {
            let line = a_lines[a_lines.len() - suffix_len + i];
            diff.push_str(&format!(" {}\n", line));
        }

        Ok(diff)
    }

    /// Get a branch by name
    pub async fn get_branch(&self, name: &str) -> Option<RefactoringBranch> {
        let branches = self.branches.read().await;
        branches.get(name).cloned()
    }

    /// Get all branches
    pub async fn get_all_branches(&self) -> Vec<RefactoringBranch> {
        let branches = self.branches.read().await;
        branches.values().cloned().collect()
    }

    /// Delete a branch
    pub async fn delete_branch(&self, name: &str) -> Result<()> {
        let mut branches = self.branches.write().await;

        if branches.remove(name).is_some() {
            Ok(())
        } else {
            Err(ZseiError::Refactor(format!("Branch not found: {}", name)))
        }
    }
}
