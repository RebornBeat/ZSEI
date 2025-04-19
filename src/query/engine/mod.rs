//! Query engine implementation
//!
//! This module provides the core query engine functionality for processing
//! queries and generating responses.

use std::sync::Arc;
use std::collections::HashMap;
use tracing::{debug, info};

use crate::core::config::Config;
use crate::embedding::EmbeddingType;
use crate::errors::{Result, ZseiError};
use crate::indexing::{Indexer, SearchResult};
use crate::llm::{Model, prompt::PromptManager};
use crate::query::{CodeSnippet, QueryContext, QueryResult, QueryType, Suggestion};

/// Query engine implementation
pub struct QueryEngineImpl {
    /// Configuration
    config: Arc<Config>,

    /// Indexer
    indexer: Arc<Indexer>,

    /// LLM
    llm: Arc<dyn Model>,

    /// Prompt manager
    prompt_manager: PromptManager,
}

impl QueryEngineImpl {
    /// Create a new query engine implementation
    pub fn new(
        config: Arc<Config>,
        indexer: Arc<Indexer>,
        llm: Arc<dyn Model>,
    ) -> Self {
        Self {
            config,
            indexer,
            llm,
            prompt_manager: PromptManager::new(),
        }
    }

    /// Process a query
    pub async fn process_query(
        &self,
        query_text: &str,
        max_results: usize,
        max_context_size: usize,
    ) -> Result<QueryResult> {
        info!("Processing query: {}", query_text);

        // Determine query type
        let query_type = self.determine_query_type(query_text);
        debug!("Determined query type: {:?}", query_type);

        // Create query context
        let context = QueryContext {
            query: query_text.to_string(),
            query_type: query_type.clone(),
            parameters: HashMap::new(),
            max_context_size,
        };

        // Search for relevant code
        let search_results = self.search_code(&context, max_results).await?;
        debug!("Found {} relevant files", search_results.len());

        // Extract code snippets
        let code_snippets = self.extract_code_snippets(&search_results).await?;

        // Create dependency graph
        let dependency_graph = self.build_dependency_graph(&search_results, &code_snippets).await?;

        // Generate response
        let response = self.generate_response(&context, &code_snippets, &dependency_graph).await?;

        // Generate suggestions if applicable
        let suggestions = match query_type {
            QueryType::Error | QueryType::Refactor => {
                self.generate_suggestions(&context, &code_snippets, &dependency_graph).await?
            }
            _ => Vec::new(),
        };

        // Create query result
        let result = QueryResult {
            query: query_text.to_string(),
            response,
            code_snippets,
            dependency_graph: Some(dependency_graph),
            suggestions,
        };

        Ok(result)
    }

    /// Determine the query type
    fn determine_query_type(&self, query: &str) -> QueryType {
        // Simple heuristic based on keywords
        if query.contains("error") || query.contains("bug") || query.contains("fix") || query.contains("panic") {
            QueryType::Error
        } else if query.contains("refactor") || query.contains("optimize") || query.contains("improve") || query.contains("cleanup") {
            QueryType::Refactor
        } else if query.contains("code") || query.contains("function") || query.contains("class") || query.contains("struct") {
            QueryType::Code
        } else {
            QueryType::Text
        }
    }

    /// Search for relevant code
    async fn search_code(&self, context: &QueryContext, max_results: usize) -> Result<Vec<SearchResult>> {
        debug!("Searching for code relevant to: {}", context.query);

        // Determine embedding type based on query type
        let embedding_type = match context.query_type {
            QueryType::Code | QueryType::Error | QueryType::Refactor => EmbeddingType::Code,
            QueryType::Text => EmbeddingType::Code, // Default to code for now
        };

        // Search for relevant code
        let search_results = self.indexer.search(&context.query, embedding_type, max_results).await?;

        Ok(search_results)
    }

    /// Extract code snippets from search results
    async fn extract_code_snippets(&self, search_results: &[SearchResult]) -> Result<Vec<CodeSnippet>> {
        debug!("Extracting code snippets from {} search results", search_results.len());

        let mut snippets = Vec::with_capacity(search_results.len());

        for result in search_results {
            // Read file content
            let content = match crate::utils::fs::read_file_text(&result.path) {
                Ok(content) => content,
                Err(e) => {
                    debug!("Failed to read file {}: {}", result.path.display(), e);
                    continue;
                }
            };

            // Extract relationships
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

    /// Build a dependency graph from search results and code snippets
    async fn build_dependency_graph(
        &self,
        search_results: &[SearchResult],
        code_snippets: &[CodeSnippet],
    ) -> Result<crate::analyzers::common::CodeGraph> {
        debug!("Building dependency graph");

        // For now, create a simple graph with files as nodes
        let mut dependencies = Vec::new();

        for result in search_results {
            // Add import dependencies
            for import in &result.metadata.imports {
                dependencies.push(crate::analyzers::common::Dependency {
                    source: result.path.clone(),
                    target: std::path::PathBuf::from(import),
                    dependency_type: crate::analyzers::common::DependencyType::Import,
                    line: None,
                    info: None,
                });
            }
        }

        Ok(crate::analyzers::common::CodeGraph::new(&dependencies))
    }

    /// Generate a response using LLM
    async fn generate_response(
        &self,
        context: &QueryContext,
        code_snippets: &[CodeSnippet],
        dependency_graph: &crate::analyzers::common::CodeGraph,
    ) -> Result<String> {
        debug!("Generating response for query: {}", context.query);

        // Create a prompt for the LLM
        let prompt = self.create_response_prompt(context, code_snippets, dependency_graph)?;

        debug!("Response prompt length: {} characters", prompt.len());

        // Generate response
        let response = self.llm.generate(&prompt).await?;

        Ok(response)
    }

    /// Create a prompt for generating a response
    fn create_response_prompt(
        &self,
        context: &QueryContext,
        code_snippets: &[CodeSnippet],
        dependency_graph: &crate::analyzers::common::CodeGraph,
    ) -> Result<String> {
        let mut prompt = String::new();

        // Add query
        prompt.push_str(&format!("Query: {}\n\n", context.query));

        // Add code snippets
        prompt.push_str("Relevant code snippets:\n\n");

        for (i, snippet) in code_snippets.iter().enumerate() {
            prompt.push_str(&format!("Snippet {}:\n", i + 1));
            prompt.push_str(&format!("File: {}\n", snippet.path.display()));
            prompt.push_str(&format!("Language: {}\n", snippet.language.as_deref().unwrap_or("unknown")));
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
        dependency_graph: &crate::analyzers::common::CodeGraph,
    ) -> Result<Vec<Suggestion>> {
        debug!("Generating suggestions for query: {}", context.query);

        // Create prompt for LLM
        let prompt = self.create_suggestions_prompt(context, code_snippets, dependency_graph)?;

        debug!("Suggestions prompt length: {} characters", prompt.len());

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
        dependency_graph: &crate::analyzers::common::CodeGraph,
    ) -> Result<String> {
        let mut prompt = String::new();

        // Add query
        prompt.push_str(&format!("Query: {}\n\n", context.query));

        // Add code snippets
        prompt.push_str("Code snippets to improve:\n\n");

        for (i, snippet) in code_snippets.iter().enumerate() {
            prompt.push_str(&format!("Snippet {}:\n", i + 1));
            prompt.push_str(&format!("File: {}\n", snippet.path.display()));
            prompt.push_str(&format!("Language: {}\n", snippet.language.as_deref().unwrap_or("unknown")));
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
    fn parse_suggestions(&self, response: &str, code_snippets: &[CodeSnippet]) -> Result<Vec<Suggestion>> {
        debug!("Parsing suggestions from LLM response");

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
                Some(std::path::PathBuf::from(file_str))
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
            if !title.is_empty() && !description.is_empty() {
                let suggestion = Suggestion {
                    title,
                    description,
                    code,
                    path: file_path,
                };

                suggestions.push(suggestion);
            }
        }

        debug!("Extracted {} suggestions", suggestions.len());

        Ok(suggestions)
    }
}
