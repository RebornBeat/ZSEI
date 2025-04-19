//! Code optimization for refactoring
//!
//! This module provides code optimization functionality for refactoring.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info};

use crate::errors::{Result, ZseiError};
use crate::llm::Model;
use crate::refactor::{ChangeType, CodeChange, RefactoringBranch};

/// Optimization strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationStrategy {
    /// Correctness-focused optimization
    Correctness,

    /// Performance-focused optimization
    Performance,

    /// Readability-focused optimization
    Readability,

    /// Maintainability-focused optimization
    Maintainability,

    /// Balanced optimization
    Balanced,
}

/// Code optimizer
pub struct CodeOptimizer {
    /// LLM
    llm: Arc<dyn Model>,
}

impl CodeOptimizer {
    /// Create a new code optimizer
    pub fn new(llm: Arc<dyn Model>) -> Self {
        Self { llm }
    }

    /// Optimize a file
    pub async fn optimize_file(
        &self,
        path: &Path,
        content: &str,
        strategy: OptimizationStrategy,
    ) -> Result<CodeChange> {
        info!("Optimizing file: {}", path.display());

        // Create optimization prompt
        let prompt = self.create_optimization_prompt(path, content, strategy)?;

        // Generate optimized code
        let optimized = self.llm.generate(&prompt).await?;

        // Extract code section
        let modified = self.extract_code_from_response(&optimized)?;

        // Create code change
        let change = CodeChange {
            path: path.to_path_buf(),
            original: content.to_string(),
            modified,
            description: self.extract_description_from_response(&optimized)?,
            change_type: ChangeType::Modify,
        };

        Ok(change)
    }

    /// Optimize a refactoring branch
    pub async fn optimize_branch(
        &self,
        branch: &RefactoringBranch,
        strategy: OptimizationStrategy,
    ) -> Result<RefactoringBranch> {
        info!("Optimizing branch: {}", branch.name);

        // Clone branch
        let mut optimized_branch = branch.clone();
        optimized_branch.name = format!("{}-optimized", branch.name);
        optimized_branch.description =
            format!("{} (optimized for {:?})", branch.description, strategy);

        // Optimize each change
        let mut optimized_changes = Vec::new();

        for change in &branch.changes {
            // Only optimize modifications
            if change.change_type == ChangeType::Modify {
                let optimized_change = self
                    .optimize_file(&change.path, &change.modified, strategy)
                    .await?;
                optimized_changes.push(optimized_change);
            } else {
                optimized_changes.push(change.clone());
            }
        }

        optimized_branch.changes = optimized_changes;

        Ok(optimized_branch)
    }

    /// Create an optimization prompt
    fn create_optimization_prompt(
        &self,
        path: &Path,
        content: &str,
        strategy: OptimizationStrategy,
    ) -> Result<String> {
        let mut prompt = String::new();

        // Add file information
        prompt.push_str(&format!("File: {}\n\n", path.display()));

        // Add code content
        prompt.push_str("Original code:\n```\n");
        prompt.push_str(content);
        prompt.push_str("\n```\n\n");

        // Add optimization strategy
        prompt.push_str(&format!("Optimization strategy: {:?}\n\n", strategy));

        // Add strategy-specific instructions
        match strategy {
            OptimizationStrategy::Correctness => {
                prompt.push_str("Instructions:\n");
                prompt.push_str("1. Fix any potential bugs or errors in the code\n");
                prompt.push_str("2. Ensure error handling is comprehensive\n");
                prompt.push_str("3. Check for edge cases and handle them appropriately\n");
                prompt.push_str("4. Verify that all operations are safe and secure\n");
                prompt.push_str("5. Maintain the same functionality while improving correctness\n");
            }
            OptimizationStrategy::Performance => {
                prompt.push_str("Instructions:\n");
                prompt.push_str("1. Optimize for execution speed and memory usage\n");
                prompt.push_str("2. Eliminate unnecessary operations and allocations\n");
                prompt.push_str("3. Improve algorithm efficiency where possible\n");
                prompt.push_str("4. Consider parallelization opportunities\n");
                prompt.push_str("5. Maintain correctness while improving performance\n");
            }
            OptimizationStrategy::Readability => {
                prompt.push_str("Instructions:\n");
                prompt.push_str("1. Improve variable and function naming for clarity\n");
                prompt.push_str("2. Add comments for complex logic\n");
                prompt.push_str("3. Simplify complex expressions and control flow\n");
                prompt.push_str("4. Ensure consistent formatting and style\n");
                prompt.push_str("5. Maintain functionality while improving readability\n");
            }
            OptimizationStrategy::Maintainability => {
                prompt.push_str("Instructions:\n");
                prompt.push_str("1. Improve code structure and organization\n");
                prompt.push_str("2. Extract reusable functions for common operations\n");
                prompt.push_str("3. Apply appropriate design patterns\n");
                prompt.push_str("4. Reduce code duplication\n");
                prompt.push_str("5. Maintain functionality while improving maintainability\n");
            }
            OptimizationStrategy::Balanced => {
                prompt.push_str("Instructions:\n");
                prompt.push_str(
                    "1. Balance correctness, performance, readability, and maintainability\n",
                );
                prompt.push_str("2. Fix any obvious bugs or issues\n");
                prompt.push_str("3. Make moderate improvements to performance\n");
                prompt.push_str("4. Improve readability where it doesn't hurt performance\n");
                prompt.push_str("5. Enhance maintainability while preserving functionality\n");
            }
        }

        // Add response format
        prompt.push_str(
            "\nPlease provide the optimized code along with a description of the changes made.\n",
        );
        prompt.push_str("Format your response as follows:\n\n");
        prompt.push_str("DESCRIPTION: [description of the changes made]\n\n");
        prompt.push_str("OPTIMIZED CODE:\n```\n[optimized code here]\n```\n");

        Ok(prompt)
    }

    /// Extract code from response
    fn extract_code_from_response(&self, response: &str) -> Result<String> {
        // Find code section
        let code_start = response.find("```");
        let code_end = response.rfind("```");

        if let (Some(start), Some(end)) = (code_start, code_end) {
            // Extract code between triple backticks
            let code_with_backticks = &response[start..=end];

            // Remove backticks and language identifier if present
            let code_content = code_with_backticks
                .trim_start_matches("```")
                .trim_end_matches("```");

            // Remove language identifier if present (first line)
            let first_newline = code_content.find('\n');
            let code = if let Some(pos) = first_newline {
                let first_line = &code_content[..pos];
                if !first_line.contains(' ') && first_line.len() < 20 {
                    // Likely a language identifier
                    code_content[pos + 1..].trim()
                } else {
                    code_content.trim()
                }
            } else {
                code_content.trim()
            };

            Ok(code.to_string())
        } else {
            // Fallback: return everything after "OPTIMIZED CODE:"
            if let Some(pos) = response.find("OPTIMIZED CODE:") {
                let code = response[pos + "OPTIMIZED CODE:".len()..].trim();

                // Remove code blocks if present
                let code = code
                    .trim_start_matches("```")
                    .trim_end_matches("```")
                    .trim();

                Ok(code.to_string())
            } else {
                // Last resort: return the whole response
                Err(ZseiError::Refactor(
                    "Could not extract code from response".to_string(),
                ))
            }
        }
    }

    /// Extract description from response
    fn extract_description_from_response(&self, response: &str) -> Result<String> {
        // Find description section
        if let Some(desc_start) = response.find("DESCRIPTION:") {
            let desc_text = &response[desc_start + "DESCRIPTION:".len()..];

            // Find end of description (start of code section or end of response)
            let desc_end = desc_text.find("OPTIMIZED CODE:").unwrap_or(desc_text.len());

            let description = desc_text[..desc_end].trim();
            Ok(description.to_string())
        } else {
            // No description found
            Ok("Optimized code".to_string())
        }
    }

    /// Analyze optimization opportunities in code
    pub async fn analyze_optimization_opportunities(
        &self,
        path: &Path,
        content: &str,
    ) -> Result<Vec<OptimizationOpportunity>> {
        info!(
            "Analyzing optimization opportunities for: {}",
            path.display()
        );

        // Create analysis prompt
        let prompt = self.create_analysis_prompt(path, content)?;

        // Generate analysis
        let analysis = self.llm.generate(&prompt).await?;

        // Parse opportunities
        self.parse_optimization_opportunities(&analysis)
    }

    /// Create an analysis prompt
    fn create_analysis_prompt(&self, path: &Path, content: &str) -> Result<String> {
        let mut prompt = String::new();

        // Add file information
        prompt.push_str(&format!("File: {}\n\n", path.display()));

        // Add code content
        prompt.push_str("Code to analyze:\n```\n");
        prompt.push_str(content);
        prompt.push_str("\n```\n\n");

        // Add instructions
        prompt.push_str("Instructions:\n");
        prompt.push_str("1. Analyze the code for optimization opportunities\n");
        prompt.push_str("2. Identify potential issues in the following categories:\n");
        prompt.push_str("   - Correctness (bugs, error handling, edge cases)\n");
        prompt.push_str("   - Performance (efficiency, memory usage, algorithms)\n");
        prompt.push_str("   - Readability (naming, comments, complexity)\n");
        prompt.push_str("   - Maintainability (structure, duplication, patterns)\n");
        prompt.push_str("3. For each opportunity, provide:\n");
        prompt.push_str("   - A title describing the issue\n");
        prompt.push_str("   - The category of the issue\n");
        prompt.push_str("   - A description of the problem\n");
        prompt.push_str("   - A suggested improvement\n");
        prompt.push_str("   - The line or section where the issue occurs\n");

        // Add response format
        prompt.push_str("\nFormat your response with one opportunity per section as follows:\n\n");
        prompt.push_str("OPPORTUNITY: [title]\n");
        prompt.push_str("CATEGORY: [correctness|performance|readability|maintainability]\n");
        prompt.push_str("DESCRIPTION: [description of the problem]\n");
        prompt.push_str("SUGGESTION: [suggested improvement]\n");
        prompt.push_str("LOCATION: [line or section information]\n\n");

        Ok(prompt)
    }

    /// Parse optimization opportunities from analysis
    fn parse_optimization_opportunities(
        &self,
        analysis: &str,
    ) -> Result<Vec<OptimizationOpportunity>> {
        let mut opportunities = Vec::new();

        // Split analysis into opportunity sections
        let sections: Vec<&str> = analysis.split("OPPORTUNITY:").skip(1).collect();

        for section in sections {
            // Extract opportunity details
            let title = self
                .extract_field(section, "OPPORTUNITY:", "CATEGORY:")
                .unwrap_or_else(|| "Unnamed opportunity".to_string());

            let category_str = self
                .extract_field(section, "CATEGORY:", "DESCRIPTION:")
                .unwrap_or_else(|| "unknown".to_string())
                .to_lowercase();

            let category = match category_str.as_str() {
                "correctness" => OptimizationCategory::Correctness,
                "performance" => OptimizationCategory::Performance,
                "readability" => OptimizationCategory::Readability,
                "maintainability" => OptimizationCategory::Maintainability,
                _ => OptimizationCategory::Other(category_str),
            };

            let description = self
                .extract_field(section, "DESCRIPTION:", "SUGGESTION:")
                .unwrap_or_else(|| "No description provided".to_string());

            let suggestion = self
                .extract_field(section, "SUGGESTION:", "LOCATION:")
                .unwrap_or_else(|| "No suggestion provided".to_string());

            let location = self
                .extract_field(section, "LOCATION:", "")
                .unwrap_or_else(|| "Unknown location".to_string());

            // Create opportunity
            let opportunity = OptimizationOpportunity {
                title,
                category,
                description,
                suggestion,
                location,
            };

            opportunities.push(opportunity);
        }

        Ok(opportunities)
    }

    /// Extract a field from a section
    fn extract_field(&self, text: &str, start_marker: &str, end_marker: &str) -> Option<String> {
        let start = text
            .find(start_marker)
            .map(|pos| pos + start_marker.len())?;

        let end = if end_marker.is_empty() {
            text.len()
        } else {
            text.find(end_marker).unwrap_or(text.len())
        };

        if start < end {
            Some(text[start..end].trim().to_string())
        } else {
            None
        }
    }
}

/// Optimization opportunity
#[derive(Debug, Clone)]
pub struct OptimizationOpportunity {
    /// Title
    pub title: String,

    /// Category
    pub category: OptimizationCategory,

    /// Description
    pub description: String,

    /// Suggestion
    pub suggestion: String,

    /// Location
    pub location: String,
}

/// Optimization category
#[derive(Debug, Clone)]
pub enum OptimizationCategory {
    /// Correctness
    Correctness,

    /// Performance
    Performance,

    /// Readability
    Readability,

    /// Maintainability
    Maintainability,

    /// Other
    Other(String),
}
