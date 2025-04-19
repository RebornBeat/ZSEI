//! Query processing for ZSEI
//!
//! This module provides query processing functionality for the
//! Zero-Shot Bolted Embedding Indexer.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info};

pub mod engine;
pub mod prompt;

use crate::analyzers::common::{CodeGraph, Dependency, DependencyType};
use crate::core::config::Config;
use crate::embedding::EmbeddingType;
use crate::errors::{Result, ZseiError};
use crate::indexing::{Indexer, SearchResult};
use crate::llm::Model;

/// Query context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryContext {
    /// Query text
    pub query: String,

    /// Query type
    pub query_type: QueryType,

    /// Additional parameters
    pub parameters: HashMap<String, String>,

    /// Maximum context size
    pub max_context_size: usize,
}

/// Query type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryType {
    /// Free text query
    Text,

    /// Code search query
    Code,

    /// Error fixing query
    Error,

    /// Refactoring query
    Refactor,
}

/// Query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Original query
    pub query: String,

    /// Response text
    pub response: String,

    /// Relevant code snippets
    pub code_snippets: Vec<CodeSnippet>,

    /// Dependency graph
    pub dependency_graph: Option<CodeGraph>,

    /// Suggestions
    pub suggestions: Vec<Suggestion>,
}

impl QueryResult {
    /// Save the query result to a file
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| ZseiError::Query(format!("Failed to create directory: {}", e)))?;
        }

        // Determine file format
        let format = if let Some(ext) = path.extension() {
            match ext.to_str() {
                Some("json") => OutputFormat::Json,
                Some("md") => OutputFormat::Markdown,
                _ => OutputFormat::Json,
            }
        } else {
            OutputFormat::Json
        };

        // Generate output
        let content = match format {
            OutputFormat::Json => serde_json::to_string_pretty(self).map_err(|e| {
                ZseiError::Query(format!("Failed to serialize query result: {}", e))
            })?,
            OutputFormat::Markdown => self.to_markdown(),
        };

        // Write to file
        fs::write(path, content)
            .map_err(|e| ZseiError::Query(format!("Failed to write query result: {}", e)))?;

        Ok(())
    }

    /// Convert the query result to markdown
    fn to_markdown(&self) -> String {
        let mut markdown = String::new();

        // Add query
        markdown.push_str(&format!("# Query\n\n{}\n\n", self.query));

        // Add response
        markdown.push_str(&format!("# Response\n\n{}\n\n", self.response));

        // Add code snippets
        if !self.code_snippets.is_empty() {
            markdown.push_str("# Relevant Code Snippets\n\n");

            for snippet in &self.code_snippets {
                markdown.push_str(&format!("## {}\n\n", snippet.path.display()));

                // Add language hint for syntax highlighting
                let language = snippet.language.as_deref().unwrap_or("");
                markdown.push_str(&format!("```{}\n{}\n```\n\n", language, snippet.content));

                // Add relationships
                if !snippet.relationships.is_empty() {
                    markdown.push_str("### Relationships\n\n");

                    for (rel_type, targets) in &snippet.relationships {
                        markdown.push_str(&format!("- {}: {}\n", rel_type, targets.join(", ")));
                    }

                    markdown.push_str("\n");
                }
            }
        }

        // Add suggestions
        if !self.suggestions.is_empty() {
            markdown.push_str("# Suggestions\n\n");

            for suggestion in &self.suggestions {
                markdown.push_str(&format!("## {}\n\n", suggestion.title));
                markdown.push_str(&format!("{}\n\n", suggestion.description));

                if let Some(code) = &suggestion.code {
                    markdown.push_str("```\n");
                    markdown.push_str(code);
                    markdown.push_str("\n```\n\n");
                }
            }
        }

        markdown
    }
}

/// Code snippet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSnippet {
    /// File path
    pub path: PathBuf,

    /// Programming language
    pub language: Option<String>,

    /// Code content
    pub content: String,

    /// Relationships
    pub relationships: HashMap<String, Vec<String>>,

    /// Similarity score
    pub score: f32,
}

/// Suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    /// Suggestion title
    pub title: String,

    /// Suggestion description
    pub description: String,

    /// Code suggestion
    pub code: Option<String>,

    /// File path
    pub path: Option<PathBuf>,
}

/// Output format
#[derive(Debug, Clone, Copy)]
enum OutputFormat {
    /// JSON format
    Json,

    /// Markdown format
    Markdown,
}

/// Query engine
pub struct QueryEngine {
    /// Configuration
    config: Arc<Config>,

    /// Indexer
    indexer: Arc<Indexer>,

    /// LLM
    llm: Arc<dyn Model>,
}

impl QueryEngine {
    /// Create a new query engine
    pub fn new(config: Arc<Config>, indexer: Arc<Indexer>, llm: Arc<dyn Model>) -> Self {
        Self {
            config,
            indexer,
            llm,
        }
    }

    /// Execute a query
    pub async fn query(
        &self,
        query: &str,
        max_results: usize,
        max_context_size: usize,
    ) -> Result<QueryResult> {
        info!("Executing query: {}", query);

        // Determine query type
        let query_type = self.determine_query_type(query);
        debug!("Determined query type: {:?}", query_type);

        // Create query context
        let context = QueryContext {
            query: query.to_string(),
            query_type: query_type.clone(),
            parameters: HashMap::new(),
            max_context_size,
        };

        // Search for relevant code
        let embedding_type = match query_type {
            QueryType::Code | QueryType::Error | QueryType::Refactor => EmbeddingType::Code,
            QueryType::Text => EmbeddingType::Code, // Default to code for now
        };

        let search_results = self
            .indexer
            .search(query, embedding_type, max_results)
            .await?;
        debug!("Found {} relevant files", search_results.len());

        // Extract code snippets
        let code_snippets = self.extract_code_snippets(&search_results).await?;

        // Create dependency graph
        let dependency_graph = self.build_dependency_graph(&search_results).await?;

        // Generate response
        let response = self
            .generate_response(&context, &code_snippets, &dependency_graph)
            .await?;

        // Create suggestions
        let suggestions = self
            .generate_suggestions(&context, &code_snippets, &dependency_graph)
            .await?;

        // Create query result
        let result = QueryResult {
            query: query.to_string(),
            response,
            code_snippets,
            dependency_graph: Some(dependency_graph),
            suggestions,
        };

        Ok(result)
    }

    /// Determine the query type
    fn determine_query_type(&self, query: &str) -> QueryType {
        // Simple heuristic for now
        if query.contains("error") || query.contains("bug") || query.contains("fix") {
            QueryType::Error
        } else if query.contains("refactor")
            || query.contains("optimize")
            || query.contains("improve")
        {
            QueryType::Refactor
        } else if query.contains("code") || query.contains("function") || query.contains("class") {
            QueryType::Code
        } else {
            QueryType::Text
        }
    }

    /// Extract code snippets from search results
    async fn extract_code_snippets(
        &self,
        search_results: &[SearchResult],
    ) -> Result<Vec<CodeSnippet>> {
        let mut snippets = Vec::new();

        for result in search_results {
            // Read file content
            let content = match fs::read_to_string(&result.path) {
                Ok(content) => content,
                Err(e) => {
                    debug!("Failed to read file {}: {}", result.path.display(), e);
                    continue;
                }
            };

            // Extract relationships
            let relationships = self.extract_relationships(result);

            // Create code snippet
            let snippet = CodeSnippet {
                path: result.path.clone(),
                language: Some(result.metadata.language.clone()),
                content,
                relationships,
                score: result.score,
            };

            snippets.push(snippet);
        }

        Ok(snippets)
    }

    /// Extract relationships from a search result
    fn extract_relationships(&self, result: &SearchResult) -> HashMap<String, Vec<String>> {
        let mut relationships = HashMap::new();

        // Add functions
        if !result.metadata.functions.is_empty() {
            relationships.insert("Functions".to_string(), result.metadata.functions.clone());
        }

        // Add classes
        if !result.metadata.classes.is_empty() {
            relationships.insert("Classes".to_string(), result.metadata.classes.clone());
        }

        // Add imports
        if !result.metadata.imports.is_empty() {
            relationships.insert("Imports".to_string(), result.metadata.imports.clone());
        }

        relationships
    }

    /// Build a dependency graph from search results
    async fn build_dependency_graph(&self, search_results: &[SearchResult]) -> Result<CodeGraph> {
        // For now, create a simple graph with files as nodes
        let mut dependencies = Vec::new();

        for result in search_results {
            // Add import dependencies
            for import in &result.metadata.imports {
                dependencies.push(Dependency {
                    source: result.path.clone(),
                    target: PathBuf::from(import),
                    dependency_type: DependencyType::Import,
                    line: None,
                    info: None,
                });
            }
        }

        Ok(CodeGraph::new(&dependencies))
    }

    /// Generate a response using LLM
    async fn generate_response(
        &self,
        context: &QueryContext,
        code_snippets: &[CodeSnippet],
        dependency_graph: &CodeGraph,
    ) -> Result<String> {
        // Create prompt for LLM
        let prompt = self.create_response_prompt(context, code_snippets, dependency_graph)?;

        // Generate response
        let response = self.llm.generate(&prompt).await?;

        Ok(response)
    }

    /// Create a prompt for generating a response
    fn create_response_prompt(
        &self,
        context: &QueryContext,
        code_snippets: &[CodeSnippet],
        dependency_graph: &CodeGraph,
    ) -> Result<String> {
        let mut prompt = String::new();

        // Add query
        prompt.push_str(&format!("Query: {}\n\n", context.query));

        // Add code snippets
        prompt.push_str("Relevant code snippets:\n\n");

        for (i, snippet) in code_snippets.iter().enumerate() {
            prompt.push_str(&format!("Snippet {}:\n", i + 1));
            prompt.push_str(&format!("File: {}\n", snippet.path.display()));
            prompt.push_str(&format!(
                "Language: {}\n",
                snippet.language.as_deref().unwrap_or("unknown")
            ));
            prompt.push_str(&format!("Score: {:.2}\n\n", snippet.score));
            prompt.push_str(&format!("```\n{}\n```\n\n", snippet.content));

            // Add relationships
            if !snippet.relationships.is_empty() {
                prompt.push_str("Relationships:\n");

                for (rel_type, targets) in &snippet.relationships {
                    prompt.push_str(&format!("- {}: {}\n", rel_type, targets.join(", ")));
                }

                prompt.push_str("\n");
            }
        }

        // Add dependencies
        prompt.push_str("Dependencies:\n\n");

        for node in dependency_graph.nodes.values() {
            let outgoing = dependency_graph.get_outgoing_dependencies(&node.id);
            let incoming = dependency_graph.get_incoming_dependencies(&node.id);

            if !outgoing.is_empty() || !incoming.is_empty() {
                prompt.push_str(&format!("File: {}\n", node.id.display()));

                if !outgoing.is_empty() {
                    prompt.push_str("Depends on:\n");
                    for edge in outgoing {
                        prompt.push_str(&format!("- {} ({})\n", edge.target.display(), edge.label));
                    }
                }

                if !incoming.is_empty() {
                    prompt.push_str("Used by:\n");
                    for edge in incoming {
                        prompt.push_str(&format!("- {} ({})\n", edge.source.display(), edge.label));
                    }
                }

                prompt.push_str("\n");
            }
        }

        // Add instructions based on query type
        match context.query_type {
            QueryType::Text => {
                prompt.push_str("Instructions: Please answer the user's query based on the provided code snippets and dependencies. Explain the code's functionality, structure, and any relevant details.\n");
            }
            QueryType::Code => {
                prompt.push_str("Instructions: Please explain the relevant code snippets, focusing on how they work and their purpose. Identify important functions, classes, and relationships between components.\n");
            }
            QueryType::Error => {
                prompt.push_str("Instructions: Please analyze the provided code snippets to identify potential errors or bugs related to the query. Suggest specific fixes or solutions to address these issues.\n");
            }
            QueryType::Refactor => {
                prompt.push_str("Instructions: Please suggest ways to refactor or optimize the provided code snippets. Focus on improving code quality, maintainability, and performance while preserving functionality.\n");
            }
        }

        Ok(prompt)
    }

    /// Generate suggestions using LLM
    async fn generate_suggestions(
        &self,
        context: &QueryContext,
        code_snippets: &[CodeSnippet],
        dependency_graph: &CodeGraph,
    ) -> Result<Vec<Suggestion>> {
        // Only generate suggestions for error and refactor queries
        if matches!(context.query_type, QueryType::Text | QueryType::Code) {
            return Ok(Vec::new());
        }

        // Create prompt for LLM
        let prompt = self.create_suggestions_prompt(context, code_snippets, dependency_graph)?;

        // Generate suggestions
        let response = self.llm.generate(&prompt).await?;

        // Parse suggestions
        let suggestions = self.parse_suggestions(&response, code_snippets)?;

        Ok(suggestions)
    }

    /// Create a prompt for generating suggestions
    fn create_suggestions_prompt(
        &self,
        context: &QueryContext,
        code_snippets: &[CodeSnippet],
        dependency_graph: &CodeGraph,
    ) -> Result<String> {
        let mut prompt = String::new();

        // Add query
        prompt.push_str(&format!("Query: {}\n\n", context.query));

        // Add code snippets
        prompt.push_str("Code snippets to improve:\n\n");

        for (i, snippet) in code_snippets.iter().enumerate() {
            prompt.push_str(&format!("Snippet {}:\n", i + 1));
            prompt.push_str(&format!("File: {}\n", snippet.path.display()));
            prompt.push_str(&format!(
                "Language: {}\n",
                snippet.language.as_deref().unwrap_or("unknown")
            ));
            prompt.push_str(&format!("```\n{}\n```\n\n", snippet.content));
        }

        // Add instructions based on query type
        match context.query_type {
            QueryType::Error => {
                prompt.push_str("Instructions: Please provide specific suggestions to fix the errors in the code. For each suggestion, include:\n");
                prompt.push_str("1. A brief title describing the fix\n");
                prompt.push_str("2. A detailed explanation of the problem and solution\n");
                prompt.push_str("3. The corrected code\n");
                prompt.push_str("4. The file path that needs modification\n\n");
                prompt.push_str("Format each suggestion as follows:\n");
                prompt.push_str("SUGGESTION TITLE: [title]\n");
                prompt.push_str("DESCRIPTION: [description]\n");
                prompt.push_str("FILE: [file path]\n");
                prompt.push_str("CODE:\n```\n[corrected code]\n```\n\n");
                prompt.push_str("Provide 2-3 specific and actionable suggestions.\n");
            }
            QueryType::Refactor => {
                prompt.push_str("Instructions: Please provide specific suggestions to refactor and improve the code. For each suggestion, include:\n");
                prompt.push_str("1. A brief title describing the improvement\n");
                prompt.push_str("2. A detailed explanation of the benefit and approach\n");
                prompt.push_str("3. The refactored code\n");
                prompt.push_str("4. The file path that needs modification\n\n");
                prompt.push_str("Format each suggestion as follows:\n");
                prompt.push_str("SUGGESTION TITLE: [title]\n");
                prompt.push_str("DESCRIPTION: [description]\n");
                prompt.push_str("FILE: [file path]\n");
                prompt.push_str("CODE:\n```\n[refactored code]\n```\n\n");
                prompt.push_str("Provide 2-3 specific and actionable suggestions.\n");
            }
            _ => {}
        }

        Ok(prompt)
    }

    /// Parse suggestions from LLM response
    fn parse_suggestions(
        &self,
        response: &str,
        code_snippets: &[CodeSnippet],
    ) -> Result<Vec<Suggestion>> {
        let mut suggestions = Vec::new();

        // Split response into suggestion blocks
        let suggestion_blocks: Vec<&str> = response.split("SUGGESTION TITLE:").skip(1).collect();

        for block in suggestion_blocks {
            // Extract title
            let title = block.lines().next().unwrap_or("").trim().to_string();

            // Extract description
            let description_start = block.find("DESCRIPTION:").unwrap_or(0);
            let description_end = block.find("FILE:").unwrap_or(block.len());
            let description = if description_start > 0 && description_end > description_start {
                block[description_start + "DESCRIPTION:".len()..description_end]
                    .trim()
                    .to_string()
            } else {
                String::new()
            };

            // Extract file path
            let file_start = block.find("FILE:").unwrap_or(0);
            let file_end = block.find("CODE:").unwrap_or(block.len());
            let file_path = if file_start > 0 && file_end > file_start {
                let file_str = block[file_start + "FILE:".len()..file_end].trim();
                Some(PathBuf::from(file_str))
            } else {
                None
            };

            // Extract code
            let code_start = block.find("CODE:").unwrap_or(0);
            let code = if code_start > 0 {
                let code_text = &block[code_start + "CODE:".len()..];

                // Extract code between triple backticks
                if let (Some(start), Some(end)) = (code_text.find("```"), code_text.rfind("```")) {
                    let code_content = &code_text[start + 3..end].trim();

                    // Remove language identifier if present
                    let first_line_end = code_content.find('\n').unwrap_or(0);
                    if first_line_end > 0 && !code_content[..first_line_end].contains(' ') {
                        Some(code_content[first_line_end + 1..].trim().to_string())
                    } else {
                        Some(code_content.to_string())
                    }
                } else {
                    Some(code_text.trim().to_string())
                }
            } else {
                None
            };

            // Create suggestion
            let suggestion = Suggestion {
                title,
                description,
                code,
                path: file_path,
            };

            suggestions.push(suggestion);
        }

        Ok(suggestions)
    }
}
