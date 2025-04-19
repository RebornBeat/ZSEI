//! Prompt templates and management for ZSEI
//!
//! This module provides prompt templates and management for the LLM integration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Prompt template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    /// Template name
    pub name: String,

    /// Template description
    pub description: String,

    /// Template text with placeholders (e.g., {variable})
    pub template: String,

    /// Optional system prompt
    pub system_prompt: Option<String>,

    /// Required variables
    pub required_variables: Vec<String>,

    /// Optional variables with default values
    pub optional_variables: HashMap<String, String>,
}

impl PromptTemplate {
    /// Create a new prompt template
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        template: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            template: template.into(),
            system_prompt: None,
            required_variables: Vec::new(),
            optional_variables: HashMap::new(),
        }
    }

    /// Set system prompt
    pub fn with_system_prompt(mut self, system_prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(system_prompt.into());
        self
    }

    /// Add required variable
    pub fn with_required_variable(mut self, variable: impl Into<String>) -> Self {
        self.required_variables.push(variable.into());
        self
    }

    /// Add optional variable with default value
    pub fn with_optional_variable(
        mut self,
        variable: impl Into<String>,
        default_value: impl Into<String>,
    ) -> Self {
        self.optional_variables
            .insert(variable.into(), default_value.into());
        self
    }

    /// Fill template with variables
    pub fn fill(&self, variables: &HashMap<String, String>) -> Result<String, String> {
        // Check required variables
        for var in &self.required_variables {
            if !variables.contains_key(var) {
                return Err(format!("Missing required variable: {}", var));
            }
        }

        // Start with template
        let mut result = self.template.clone();

        // Replace variables
        for (var, value) in variables {
            result = result.replace(&format!("{{{}}}", var), value);
        }

        // Replace optional variables with defaults if not provided
        for (var, default_value) in &self.optional_variables {
            if !variables.contains_key(var) {
                result = result.replace(&format!("{{{}}}", var), default_value);
            }
        }

        Ok(result)
    }

    /// Get the full prompt (with system prompt if available)
    pub fn get_full_prompt(&self, variables: &HashMap<String, String>) -> Result<String, String> {
        let filled_template = self.fill(variables)?;

        if let Some(system_prompt) = &self.system_prompt {
            Ok(format!("{}\n\n{}", system_prompt, filled_template))
        } else {
            Ok(filled_template)
        }
    }
}

/// Standard prompt templates
pub struct StandardPrompts;

impl StandardPrompts {
    /// Code analysis prompt
    pub fn code_analysis() -> PromptTemplate {
        PromptTemplate::new(
            "code_analysis",
            "Analyze code to extract semantic information and relationships",
            "Analyze the following code to understand its structure, functionality, and relationships.\n\n```{language}\n{code}\n```\n\nProvide a detailed analysis including:\n1. Main functionality and purpose\n2. Key components and their relationships\n3. Important functions/classes and their roles\n4. Dependencies and interactions\n5. Any patterns or architectural principles used",
        )
        .with_required_variable("code")
        .with_required_variable("language")
        .with_system_prompt("You are a code analysis expert. Your task is to analyze code and extract semantic information about its structure, functionality, and relationships.")
    }

    /// Code optimization prompt
    pub fn code_optimization() -> PromptTemplate {
        PromptTemplate::new(
            "code_optimization",
            "Suggest optimizations for the given code",
            "Optimize the following code:\n\n```{language}\n{code}\n```\n\n{optimization_instructions}\n\nProvide optimized code along with an explanation of changes made and their benefits.",
        )
        .with_required_variable("code")
        .with_required_variable("language")
        .with_optional_variable("optimization_instructions", "Focus on improving performance, readability, and maintainability.")
        .with_system_prompt("You are a code optimization expert. Your task is to suggest improvements to make the code more efficient, readable, and maintainable while preserving its functionality.")
    }

    /// Error fixing prompt
    pub fn error_fixing() -> PromptTemplate {
        PromptTemplate::new(
            "error_fixing",
            "Fix errors in the given code",
            "Fix the following code that has compilation errors:\n\n```{language}\n{code}\n```\n\nError details:\n{error_details}\n\nProvide fixed code and explain the changes made to resolve each error.",
        )
        .with_required_variable("code")
        .with_required_variable("language")
        .with_required_variable("error_details")
        .with_system_prompt("You are a debugging expert. Your task is to fix errors in the code while preserving its intended functionality.")
    }

    /// Refactoring prompt
    pub fn refactoring() -> PromptTemplate {
        PromptTemplate::new(
            "refactoring",
            "Refactor code to improve structure and design",
            "Refactor the following code to improve its design:\n\n```{language}\n{code}\n```\n\n{refactoring_instructions}\n\nProvide refactored code with explanations of the design improvements.",
        )
        .with_required_variable("code")
        .with_required_variable("language")
        .with_optional_variable("refactoring_instructions", "Focus on improving code structure, reducing complexity, and applying appropriate design patterns.")
        .with_system_prompt("You are a code refactoring expert. Your task is to restructure code to improve its design while preserving its functionality.")
    }

    /// Documentation generation prompt
    pub fn documentation() -> PromptTemplate {
        PromptTemplate::new(
            "documentation",
            "Generate documentation for the given code",
            "Generate documentation for the following code:\n\n```{language}\n{code}\n```\n\nProvide comprehensive documentation including:\n1. Overall purpose and context\n2. Function/class/module descriptions\n3. Parameter descriptions\n4. Return value descriptions\n5. Usage examples",
        )
        .with_required_variable("code")
        .with_required_variable("language")
        .with_system_prompt("You are a documentation expert. Your task is to create clear, comprehensive documentation that helps users understand and use the code correctly.")
    }

    /// Test generation prompt
    pub fn test_generation() -> PromptTemplate {
        PromptTemplate::new(
            "test_generation",
            "Generate tests for the given code",
            "Generate tests for the following code:\n\n```{language}\n{code}\n```\n\nProvide comprehensive tests including:\n1. Unit tests for individual functions/methods\n2. Edge case tests\n3. Integration tests if applicable\n4. Any necessary setup/teardown logic",
        )
        .with_required_variable("code")
        .with_required_variable("language")
        .with_system_prompt("You are a testing expert. Your task is to create comprehensive tests that verify the code's functionality, handle edge cases, and maintain high test coverage.")
    }

    /// Code dependency analysis prompt
    pub fn dependency_analysis() -> PromptTemplate {
        PromptTemplate::new(
            "dependency_analysis",
            "Analyze dependencies in the given code",
            "Analyze dependencies in the following code:\n\n```{language}\n{code}\n```\n\nIdentify:\n1. External dependencies (libraries, frameworks)\n2. Internal dependencies (module imports, function calls)\n3. Implicit dependencies (assumptions about context or environment)\n4. Dependency relationships and hierarchies",
        )
        .with_required_variable("code")
        .with_required_variable("language")
        .with_system_prompt("You are a dependency analysis expert. Your task is to identify and analyze all dependencies in the code, both explicit and implicit.")
    }

    /// Code duplication analysis prompt
    pub fn duplication_analysis() -> PromptTemplate {
        PromptTemplate::new(
            "duplication_analysis",
            "Identify duplicated code that could be consolidated",
            "Analyze the following code to identify duplication:\n\n```{language}\n{code}\n```\n\nIdentify instances of duplicated or highly similar code that could be consolidated or abstracted. For each instance, suggest how to refactor the code to eliminate the duplication.",
        )
        .with_required_variable("code")
        .with_required_variable("language")
        .with_system_prompt("You are a code consolidation expert. Your task is to identify duplicated logic or patterns and suggest ways to eliminate redundancy while improving maintainability.")
    }

    /// Cross-file relationship analysis prompt
    pub fn cross_file_analysis() -> PromptTemplate {
        PromptTemplate::new(
            "cross_file_analysis",
            "Analyze relationships between code in different files",
            "Analyze the relationships between code in these files:\n\n{file_snippets}\n\nIdentify:\n1. Dependencies between files\n2. Data flow across files\n3. Interface boundaries\n4. Cohesion and coupling aspects\n5. Potential refactorings to improve organization",
        )
        .with_required_variable("file_snippets")
        .with_system_prompt("You are a cross-file code analysis expert. Your task is to analyze how code in different files interacts and identify potential improvements in the organization.")
    }

    /// Architectural analysis prompt
    pub fn architectural_analysis() -> PromptTemplate {
        PromptTemplate::new(
            "architectural_analysis",
            "Analyze the architecture of a codebase",
            "Analyze the architecture of this codebase based on these representative snippets:\n\n{code_snippets}\n\nProvide insights on:\n1. Overall architectural pattern(s)\n2. Component organization and responsibilities\n3. Communication and data flow patterns\n4. Strengths and weaknesses of the current architecture\n5. Suggestions for architectural improvements",
        )
        .with_required_variable("code_snippets")
        .with_system_prompt("You are a software architecture expert. Your task is to analyze code snippets to understand and evaluate the overall architecture of the codebase.")
    }

    /// Zero-Shot Bolted Embedding prompt
    pub fn zero_shot_embedding() -> PromptTemplate {
        PromptTemplate::new(
            "zero_shot_embedding",
            "Generate a Zero-Shot Bolted Embedding description",
            "Analyze the following content to create a Zero-Shot Bolted Embedding description.\n\n{content_type}: {content}\n\nProvide a detailed description focusing on:\n1. Core functionality and purpose\n2. Key components and their relationships\n3. Important patterns and structures\n4. Contextual connections and dependencies\n5. Unique identifying features",
        )
        .with_required_variable("content")
        .with_required_variable("content_type")
        .with_system_prompt("You are a Zero-Shot Bolted Embedding expert. Your task is to create a comprehensive description that captures the essence of the content in a way that can be effectively embedded and compared with other items.")
    }

    /// Initialization phase prompt
    pub fn initialization_phase() -> PromptTemplate {
        PromptTemplate::new(
            "initialization_phase",
            "Analyze a project for initialization",
            "You are analyzing a project for initialization. The project structure is as follows:\n\n{project_structure}\n\nBased on this structure, provide:\n1. An overview of the project architecture\n2. Key components and their responsibilities\n3. Potential areas for analysis focus\n4. Recommended metrics to track during analysis",
        )
        .with_required_variable("project_structure")
        .with_system_prompt("You are a project initialization expert. Your task is to analyze a project structure to prepare for deep analysis and optimization.")
    }

    /// Phase 1 (Prompt Analysis) prompt
    pub fn prompt_analysis_phase() -> PromptTemplate {
        PromptTemplate::new(
            "prompt_analysis_phase",
            "Analyze a query to find relevant code components",
            "Query: {query}\n\n{build_output}\n\nProject context:\n{project_context}\n\nAnalyze this query to determine:\n1. What specific code components would be most relevant\n2. Key functionality that needs to be examined\n3. Potential locations for issues or improvements\n4. Related components that might be affected",
        )
        .with_required_variable("query")
        .with_optional_variable("build_output", "")
        .with_required_variable("project_context")
        .with_system_prompt("You are a query analysis expert. Your task is to analyze a user query to identify the most relevant parts of a codebase that need to be examined or modified.")
    }

    /// Phase 2 (Code Refactoring) prompt
    pub fn code_refactoring_phase() -> PromptTemplate {
        PromptTemplate::new(
            "code_refactoring_phase",
            "Generate multiple refactoring approaches",
            "Query: {query}\n\nRelevant code:\n{code_context}\n\n{build_output}\n\nGenerate {branch_count} different approaches to refactor this code, focusing on:\n1. Approach {branch_focus}\n2. Specific changes needed for each approach\n3. Pros and cons of each approach\n4. Implementation details",
        )
        .with_required_variable("query")
        .with_required_variable("code_context")
        .with_optional_variable("build_output", "")
        .with_required_variable("branch_count")
        .with_required_variable("branch_focus")
        .with_system_prompt("You are a code refactoring expert. Your task is to generate multiple distinct approaches to refactoring code based on a user query and relevant code context.")
    }
}

/// Prompt manager
pub struct PromptManager {
    /// Templates
    templates: HashMap<String, PromptTemplate>,
}

impl PromptManager {
    /// Create a new prompt manager
    pub fn new() -> Self {
        let mut templates = HashMap::new();

        // Add standard templates
        let standard_templates = vec![
            StandardPrompts::code_analysis(),
            StandardPrompts::code_optimization(),
            StandardPrompts::error_fixing(),
            StandardPrompts::refactoring(),
            StandardPrompts::documentation(),
            StandardPrompts::test_generation(),
            StandardPrompts::dependency_analysis(),
            StandardPrompts::duplication_analysis(),
            StandardPrompts::cross_file_analysis(),
            StandardPrompts::architectural_analysis(),
            StandardPrompts::zero_shot_embedding(),
            StandardPrompts::initialization_phase(),
            StandardPrompts::prompt_analysis_phase(),
            StandardPrompts::code_refactoring_phase(),
        ];

        for template in standard_templates {
            templates.insert(template.name.clone(), template);
        }

        Self { templates }
    }

    /// Add a template
    pub fn add_template(&mut self, template: PromptTemplate) {
        self.templates.insert(template.name.clone(), template);
    }

    /// Get a template by name
    pub fn get_template(&self, name: &str) -> Option<&PromptTemplate> {
        self.templates.get(name)
    }

    /// Get all template names
    pub fn get_template_names(&self) -> Vec<String> {
        self.templates.keys().cloned().collect()
    }

    /// Create a prompt from a template
    pub fn create_prompt(
        &self,
        template_name: &str,
        variables: &HashMap<String, String>,
    ) -> Result<String, String> {
        let template = self
            .get_template(template_name)
            .ok_or_else(|| format!("Template not found: {}", template_name))?;

        template.get_full_prompt(variables)
    }
}

impl Default for PromptManager {
    fn default() -> Self {
        Self::new()
    }
}
