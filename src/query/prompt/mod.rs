//! Query prompt handling
//!
//! This module provides prompt handling functionality for the query engine.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::errors::{Result, ZseiError};
use crate::query::{CodeSnippet, QueryContext, QueryType};

/// Query prompt template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPromptTemplate {
    /// Template name
    pub name: String,

    /// Template for code analysis queries
    pub code_analysis_template: String,

    /// Template for error fixing queries
    pub error_fixing_template: String,

    /// Template for refactoring queries
    pub refactoring_template: String,

    /// Template for general text queries
    pub text_template: String,
}

impl Default for QueryPromptTemplate {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            code_analysis_template: DEFAULT_CODE_ANALYSIS_TEMPLATE.to_string(),
            error_fixing_template: DEFAULT_ERROR_FIXING_TEMPLATE.to_string(),
            refactoring_template: DEFAULT_REFACTORING_TEMPLATE.to_string(),
            text_template: DEFAULT_TEXT_TEMPLATE.to_string(),
        }
    }
}

/// Default template for code analysis queries
const DEFAULT_CODE_ANALYSIS_TEMPLATE: &str = r#"
Query: {{query}}

Code snippets to analyze:
{{#each code_snippets}}
File: {{path}}
Language: {{language}}
Score: {{score}}

```
{{content}}
```

{{#if relationships}}
Relationships:
{{#each relationships}}
- {{@key}}: {{this}}
{{/each}}
{{/if}}

{{/each}}

Instructions:
Please analyze the provided code snippets and answer the query. Focus on:
1. The structure and organization of the code
2. The main functionality and purpose
3. Key relationships and dependencies
4. Any notable patterns or idioms used

Provide a clear, detailed explanation that addresses the user's query directly.
"#;

/// Default template for error fixing queries
const DEFAULT_ERROR_FIXING_TEMPLATE: &str = r#"
Query: {{query}}

{{#if build_output}}
Build output:
```
{{build_output}}
```
{{/if}}

Code snippets to analyze:
{{#each code_snippets}}
File: {{path}}
Language: {{language}}
Score: {{score}}

```
{{content}}
```

{{#if relationships}}
Relationships:
{{#each relationships}}
- {{@key}}: {{this}}
{{/each}}
{{/if}}

{{/each}}

Dependencies:
{{#each dependencies}}
{{#if outgoing}}
File: {{file}}
Depends on:
{{#each outgoing}}
- {{target}} ({{type}})
{{/each}}
{{/if}}
{{#if incoming}}
Used by:
{{#each incoming}}
- {{source}} ({{type}})
{{/each}}
{{/if}}
{{/each}}

Instructions:
Please analyze the provided code snippets and identify the errors mentioned in the query.
Focus on:
1. The specific error(s) and their root causes
2. How the error affects the overall functionality
3. Clear and precise solutions to fix the issues
4. Any additional improvements that could prevent similar errors

Provide a detailed explanation of the problems and solutions, with specific code changes needed.
"#;

/// Default template for refactoring queries
const DEFAULT_REFACTORING_TEMPLATE: &str = r#"
Query: {{query}}

Code snippets to refactor:
{{#each code_snippets}}
File: {{path}}
Language: {{language}}
Score: {{score}}

```
{{content}}
```

{{#if relationships}}
Relationships:
{{#each relationships}}
- {{@key}}: {{this}}
{{/each}}
{{/if}}

{{/each}}

Dependencies:
{{#each dependencies}}
{{#if outgoing}}
File: {{file}}
Depends on:
{{#each outgoing}}
- {{target}} ({{type}})
{{/each}}
{{/if}}
{{#if incoming}}
Used by:
{{#each incoming}}
- {{source}} ({{type}})
{{/each}}
{{/if}}
{{/each}}

Instructions:
Please analyze the provided code snippets and suggest refactoring improvements.
Focus on:
1. Code quality issues and potential improvements
2. Performance optimizations
3. Better organization and structure
4. Modern idioms and best practices
5. Maintaining or improving functionality while enhancing readability and maintainability

For each suggestion, provide:
- A clear explanation of the issue
- The benefits of the proposed change
- Specific code modifications
"#;

/// Default template for general text queries
const DEFAULT_TEXT_TEMPLATE: &str = r#"
Query: {{query}}

Code snippets:
{{#each code_snippets}}
File: {{path}}
Language: {{language}}
Score: {{score}}

```
{{content}}
```

{{#if relationships}}
Relationships:
{{#each relationships}}
- {{@key}}: {{this}}
{{/each}}
{{/if}}

{{/each}}

Instructions:
Please answer the user's query based on the provided code snippets.
Provide a clear, concise, and helpful response that directly addresses the query.
"#;

/// Query prompt builder
pub struct QueryPromptBuilder {
    /// Query context
    context: QueryContext,

    /// Code snippets
    code_snippets: Vec<CodeSnippet>,

    /// Build output
    build_output: Option<String>,

    /// Dependencies
    dependencies: HashMap<String, DependencyInfo>,

    /// Prompt template
    template: QueryPromptTemplate,
}

/// Dependency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    /// File path
    pub file: String,

    /// Outgoing dependencies
    pub outgoing: Vec<DependencyEntry>,

    /// Incoming dependencies
    pub incoming: Vec<DependencyEntry>,
}

/// Dependency entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEntry {
    /// Source file
    pub source: String,

    /// Target file
    pub target: String,

    /// Dependency type
    pub r#type: String,
}

impl QueryPromptBuilder {
    /// Create a new query prompt builder
    pub fn new(
        context: QueryContext,
        code_snippets: Vec<CodeSnippet>,
    ) -> Self {
        Self {
            context,
            code_snippets,
            build_output: None,
            dependencies: HashMap::new(),
            template: QueryPromptTemplate::default(),
        }
    }

    /// Add build output
    pub fn with_build_output(mut self, build_output: String) -> Self {
        self.build_output = Some(build_output);
        self
    }

    /// Add dependencies
    pub fn with_dependencies(mut self, dependencies: HashMap<String, DependencyInfo>) -> Self {
        self.dependencies = dependencies;
        self
    }

    /// Use a custom template
    pub fn with_template(mut self, template: QueryPromptTemplate) -> Self {
        self.template = template;
        self
    }

    /// Build the prompt
    pub fn build(&self) -> Result<String> {
        let template = match self.context.query_type {
            QueryType::Code => &self.template.code_analysis_template,
            QueryType::Error => &self.template.error_fixing_template,
            QueryType::Refactor => &self.template.refactoring_template,
            QueryType::Text => &self.template.text_template,
        };

        let mut prompt = template.to_string();

        // Replace variables
        prompt = prompt.replace("{{query}}", &self.context.query);

        if let Some(build_output) = &self.build_output {
            prompt = prompt.replace("{{build_output}}", build_output);
        }

        // Replace code snippets (simplified, would use a proper template engine in production)
        let mut code_snippets_text = String::new();
        for snippet in &self.code_snippets {
            code_snippets_text.push_str(&format!("File: {}\n", snippet.path.display()));
            code_snippets_text.push_str(&format!("Language: {}\n", snippet.language.as_deref().unwrap_or("unknown")));
            code_snippets_text.push_str(&format!("Score: {:.2}\n\n", snippet.score));
            code_snippets_text.push_str("```\n");
            code_snippets_text.push_str(&snippet.content);
            code_snippets_text.push_str("\n```\n\n");

            if !snippet.relationships.is_empty() {
                code_snippets_text.push_str("Relationships:\n");
                for (rel_type, targets) in &snippet.relationships {
                    code_snippets_text.push_str(&format!("- {}: {}\n", rel_type, targets.join(", ")));
                }
                code_snippets_text.push_str("\n");
            }
        }

        prompt = prompt.replace("{{#each code_snippets}}", &code_snippets_text);

        // Replace dependencies (simplified, would use a proper template engine in production)
        let mut dependencies_text = String::new();
        for (file, info) in &self.dependencies {
            dependencies_text.push_str(&format!("File: {}\n", file));

            if !info.outgoing.is_empty() {
                dependencies_text.push_str("Depends on:\n");
                for dep in &info.outgoing {
                    dependencies_text.push_str(&format!("- {} ({})\n", dep.target, dep.r#type));
                }
            }

            if !info.incoming.is_empty() {
                dependencies_text.push_str("Used by:\n");
                for dep in &info.incoming {
                    dependencies_text.push_str(&format!("- {} ({})\n", dep.source, dep.r#type));
                }
            }

            dependencies_text.push_str("\n");
        }

        prompt = prompt.replace("{{#each dependencies}}", &dependencies_text);

        // Clean up any remaining template tags (simplified)
        prompt = prompt.replace("{{#if build_output}}", "");
        prompt = prompt.replace("{{/if}}", "");
        prompt = prompt.replace("{{#if relationships}}", "");
        prompt = prompt.replace("{{#if outgoing}}", "");
        prompt = prompt.replace("{{#if incoming}}", "");
        prompt = prompt.replace("{{/each}}", "");

        Ok(prompt)
    }
}

/// Suggestions prompt builder
pub struct SuggestionsPromptBuilder {
    /// Query context
    context: QueryContext,

    /// Code snippets
    code_snippets: Vec<CodeSnippet>,

    /// Build output
    build_output: Option<String>,
}

impl SuggestionsPromptBuilder {
    /// Create a new suggestions prompt builder
    pub fn new(
        context: QueryContext,
        code_snippets: Vec<CodeSnippet>,
    ) -> Self {
        Self {
            context,
            code_snippets,
            build_output: None,
        }
    }

    /// Add build output
    pub fn with_build_output(mut self, build_output: String) -> Self {
        self.build_output = Some(build_output);
        self
    }

    /// Build the prompt
    pub fn build(&self) -> Result<String> {
        let mut prompt = String::new();

        // Add query
        prompt.push_str(&format!("Query: {}\n\n", self.context.query));

        // Add build output if available
        if let Some(output) = &self.build_output {
            prompt.push_str("Build Output:\n");
            prompt.push_str(output);
            prompt.push_str("\n\n");
        }

        // Add code snippets
        prompt.push_str("Code snippets to improve:\n\n");

        for (i, snippet) in self.code_snippets.iter().enumerate() {
            prompt.push_str(&format!("Snippet {}:\n", i + 1));
            prompt.push_str(&format!("File: {}\n", snippet.path.display()));
            prompt.push_str(&format!("Language: {}\n", snippet.language.as_deref().unwrap_or("unknown")));
            prompt.push_str("```\n");
            prompt.push_str(&snippet.content);
            prompt.push_str("\n```\n\n");
        }

        // Add instructions based on query type
        match self.context.query_type {
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
            _ => {
                return Err(ZseiError::Query("Suggestions are only available for Error and Refactor query types".to_string()));
            }
        }

        Ok(prompt)
    }
}

/// Convert a dependency graph to dependency information
pub fn convert_dependency_graph_to_info(
    graph: &crate::analyzers::common::CodeGraph,
) -> HashMap<String, DependencyInfo> {
    let mut result = HashMap::new();

    for node in graph.nodes.values() {
        let file = node.id.to_string_lossy().to_string();

        let outgoing = graph.get_outgoing_dependencies(&node.id)
            .iter()
            .map(|edge| DependencyEntry {
                source: edge.source.to_string_lossy().to_string(),
                target: edge.target.to_string_lossy().to_string(),
                r#type: format!("{:?}", edge.edge_type),
            })
            .collect();

        let incoming = graph.get_incoming_dependencies(&node.id)
            .iter()
            .map(|edge| DependencyEntry {
                source: edge.source.to_string_lossy().to_string(),
                target: edge.target.to_string_lossy().to_string(),
                r#type: format!("{:?}", edge.edge_type),
            })
            .collect();

        result.insert(file.clone(), DependencyInfo {
            file,
            outgoing,
            incoming,
        });
    }

    result
}

/// Extract the best code snippets based on context size
pub fn extract_best_snippets(
    snippets: &[CodeSnippet],
    max_context_size: usize,
) -> Vec<CodeSnippet> {
    // Sort snippets by score (highest first)
    let mut sorted_snippets = snippets.to_vec();
    sorted_snippets.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    // Calculate total content size
    let mut current_size = 0;
    let mut result = Vec::new();

    for snippet in sorted_snippets {
        let snippet_size = snippet.content.len();

        // If adding this snippet would exceed the limit, stop
        if current_size + snippet_size > max_context_size && !result.is_empty() {
            break;
        }

        // Add snippet and update size
        result.push(snippet.clone());
        current_size += snippet_size;
    }

    result
}

/// Parse suggestions from LLM response
pub fn parse_suggestions(
    response: &str,
    code_snippets: &[CodeSnippet],
) -> Result<Vec<crate::query::Suggestion>> {
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
        let suggestion = crate::query::Suggestion {
            title,
            description,
            code,
            path: file_path,
        };

        suggestions.push(suggestion);
    }

    Ok(suggestions)
}
