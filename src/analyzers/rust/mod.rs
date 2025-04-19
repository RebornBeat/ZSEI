//! Rust code analyzer
//!
//! This module provides a comprehensive analyzer for Rust code, extracting
//! structural information, relationships, and metrics using tree-sitter.

use async_trait::async_trait;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info};
use tree_sitter::{Parser, Query, QueryCursor, QueryMatch, StreamingIterator, Tree};

use crate::analyzers::common::{
    CodeMetrics, Dependency, DependencyType, FileAnalysis, Function, FunctionMetrics, Import,
    Parameter, Variable,
};
use crate::analyzers::LanguageAnalyzer;
use crate::core::config::Config;
use crate::errors::{Result, ZseiError};

/// Rust analyzer implementation providing comprehensive code analysis
/// for Rust source files using tree-sitter.
pub struct RustAnalyzer {
    /// Configuration options
    config: Arc<Config>,

    /// Tree-sitter parser instance
    parser: Parser,

    /// Collection of pre-compiled queries for Rust code analysis
    queries: RustQueries,
}

/// Collection of tree-sitter queries for extracting different
/// aspects of Rust code structure and relationships.
struct RustQueries {
    /// Query for extracting function definitions
    function_query: Query,

    /// Query for extracting import statements
    import_query: Query,

    /// Query for extracting struct definitions
    struct_query: Query,

    /// Query for extracting enum definitions
    enum_query: Query,

    /// Query for extracting trait definitions
    trait_query: Query,

    /// Query for extracting impl blocks
    impl_query: Query,

    /// Query for extracting function calls
    function_call_query: Query,

    /// Query for extracting type references
    type_reference_query: Query,
}

impl RustAnalyzer {
    /// Create a new Rust analyzer with the given configuration.
    ///
    /// This initializes the tree-sitter parser with Rust language support
    /// and compiles all the necessary queries for Rust code analysis.
    pub fn new(config: Arc<Config>) -> Self {
        let mut parser = Parser::new();
        let language = tree_sitter_rust::LANGUAGE;
        parser
            .set_language(&language.into())
            .expect("Failed to set Rust language");

        // Create queries for different aspects of Rust code
        let function_query = Query::new(
            &tree_sitter_rust::LANGUAGE.into(),
            r#"
            (function_item
              name: (identifier) @function_name
              parameters: (parameters) @parameters
              body: (block) @body
              .
              (#match? @function_name "^[a-zA-Z_][a-zA-Z0-9_]*$"))
            "#,
        )
        .expect("Failed to create function query");

        let import_query = Query::new(
            &tree_sitter_rust::LANGUAGE.into(),
            r#"
            (use_declaration
              argument: (_) @path)
            "#,
        )
        .expect("Failed to create import query");

        let struct_query = Query::new(
            &tree_sitter_rust::LANGUAGE.into(),
            r#"
            (struct_item
              name: (type_identifier) @struct_name
              body: (field_declaration_list)? @body)
            "#,
        )
        .expect("Failed to create struct query");

        let enum_query = Query::new(
            &tree_sitter_rust::LANGUAGE.into(),
            r#"
            (enum_item
              name: (type_identifier) @enum_name
              body: (enum_variant_list) @body)
            "#,
        )
        .expect("Failed to create enum query");

        let trait_query = Query::new(
            &tree_sitter_rust::LANGUAGE.into(),
            r#"
            (trait_item
              name: (type_identifier) @trait_name
              body: (declaration_list) @body)
            "#,
        )
        .expect("Failed to create trait query");

        let impl_query = Query::new(
            &tree_sitter_rust::LANGUAGE.into(),
            r#"
            (impl_item
              trait: (type_identifier)? @trait_name
              type: (type_identifier) @type_name
              body: (declaration_list) @body)
            "#,
        )
        .expect("Failed to create impl query");

        let function_call_query = Query::new(
            &tree_sitter_rust::LANGUAGE.into(),
            r#"
            (call_expression
              function: [
                (identifier) @function_name
                (field_expression
                  field: (field_identifier) @function_name)
                (scoped_identifier
                  path: (identifier)? @module_name
                  name: (identifier) @function_name)
              ])
            "#,
        )
        .expect("Failed to create function call query");

        let type_reference_query = Query::new(
            &tree_sitter_rust::LANGUAGE.into(),
            r#"
            (type_identifier) @type_name
            "#,
        )
        .expect("Failed to create type reference query");

        let queries = RustQueries {
            function_query,
            import_query,
            struct_query,
            enum_query,
            trait_query,
            impl_query,
            function_call_query,
            type_reference_query,
        };

        Self {
            config,
            parser,
            queries,
        }
    }

    /// Parse a file into a syntax tree
    ///
    /// This reads the file content and parses it using tree-sitter to create
    /// an abstract syntax tree (AST) that can be queried for structural information.
    fn parse_file(&mut self, path: &Path) -> Result<Tree> {
        let content = fs::read_to_string(path).map_err(|e| {
            ZseiError::Analyzer(format!("Failed to read file {}: {}", path.display(), e))
        })?;

        let tree = self.parser.parse(&content, None).ok_or_else(|| {
            ZseiError::Analyzer(format!("Failed to parse file {}", path.display()))
        })?;

        Ok(tree)
    }

    /// Extract functions from the syntax tree
    ///
    /// This queries the syntax tree for function definitions and extracts
    /// comprehensive information about each function including name, signature,
    /// parameters, and metrics like complexity.
    fn extract_functions(&self, tree: &Tree, content: &str) -> Result<Vec<Function>> {
        let mut functions = Vec::new();
        let mut query_cursor = QueryCursor::new();

        let mut matches = query_cursor.matches(
            &self.queries.function_query,
            tree.root_node(),
            content.as_bytes(),
        );

        // Process each match using the correct tree-sitter pattern
        while let Some(match_) = matches.next() {
            let mut function_name_node = None;
            let mut parameters_node = None;
            let mut body_node = None;

            // Process each capture in the match
            for capture in match_.captures {
                let capture_name =
                    self.queries.function_query.capture_names()[capture.index as usize];

                if capture_name == "function_name" {
                    function_name_node = Some(capture.node);
                } else if capture_name == "parameters" {
                    parameters_node = Some(capture.node);
                } else if capture_name == "body" {
                    body_node = Some(capture.node);
                }
            }

            // Extract function information if we have all required nodes
            if let (Some(name_node), Some(params_node), Some(body_node)) =
                (function_name_node, parameters_node, body_node)
            {
                let name = content[name_node.byte_range()].to_string();
                let params_str = &content[params_node.byte_range()];
                let signature = format!("fn {}({})", name, params_str);
                let body = content[body_node.byte_range()].to_string();

                let start_line = name_node.start_position().row;
                let end_line = body_node.end_position().row;

                // Extract parameters
                let parameters = self.extract_parameters(params_node, content)?;

                // Calculate function metrics
                let loc = body.lines().count();
                let complexity = self.calculate_cyclomatic_complexity(&body);
                let cognitive_complexity = self.calculate_cognitive_complexity(&body);

                // Extract return type if present
                let return_type = self.extract_return_type(name_node.parent().unwrap(), content);

                // Create function object with all extracted information
                let parameter_count = parameters.len(); // Get the count before moving
                let function = Function {
                    name,
                    signature,
                    start_line,
                    end_line,
                    body: Some(body),
                    return_type,
                    parameters, // This still moves parameters
                    is_public: self.is_node_public(name_node, content),
                    metrics: FunctionMetrics {
                        loc,
                        complexity,
                        parameter_count, // Use the count we saved earlier
                        cognitive_complexity,
                    },
                };

                functions.push(function);
            }
        }

        Ok(functions)
    }

    /// Extract the return type of a function
    ///
    /// This analyzes the function node to find and extract its return type.
    fn extract_return_type(
        &self,
        function_node: tree_sitter::Node<'_>,
        content: &str,
    ) -> Option<String> {
        for i in 0..function_node.child_count() {
            if let Some(child) = function_node.child(i) {
                if child.kind() == "return_type" {
                    // Once we find the return type, get its child which is the actual type
                    if let Some(type_node) = child.child(0) {
                        return Some(content[type_node.byte_range()].to_string());
                    }
                }
            }
        }

        None
    }

    /// Extract parameters from a parameters node
    ///
    /// This analyzes the parameter list node to extract individual parameter information
    /// including names, types, and default values.
    fn extract_parameters(
        &self,
        params_node: tree_sitter::Node<'_>,
        content: &str,
    ) -> Result<Vec<Parameter>> {
        let mut parameters = Vec::new();

        // Find all parameter nodes
        for i in 0..params_node.child_count() {
            if let Some(child) = params_node.child(i) {
                if child.kind() == "parameter" {
                    let mut param_name = None;
                    let mut param_type = None;

                    // Find parameter name and type
                    for j in 0..child.child_count() {
                        if let Some(param_child) = child.child(j) {
                            if param_child.kind() == "identifier" {
                                param_name = Some(content[param_child.byte_range()].to_string());
                            } else if param_child.kind() == "type_identifier"
                                || param_child.kind() == "reference_type"
                            {
                                param_type = Some(content[param_child.byte_range()].to_string());
                            }
                        }
                    }

                    // If we found a parameter name, create a Parameter object
                    if let Some(name) = param_name {
                        parameters.push(Parameter {
                            name,
                            param_type,
                            default_value: None, // Rust doesn't have default parameter values
                            position: i,
                        });
                    }
                }
            }
        }

        Ok(parameters)
    }

    /// Extract imports from the syntax tree
    ///
    /// This queries the syntax tree for import statements (use declarations)
    /// and extracts information about each import.
    fn extract_imports(&self, tree: &Tree, content: &str) -> Result<Vec<Import>> {
        let mut imports = Vec::new();
        let mut query_cursor = QueryCursor::new();

        let mut matches = query_cursor.matches(
            &self.queries.import_query,
            tree.root_node(),
            content.as_bytes(),
        );

        // Process each match using the correct tree-sitter pattern
        while let Some(match_) = matches.next() {
            let mut path_node = None;

            // Process each capture in the match
            for capture in match_.captures {
                let capture_name =
                    self.queries.import_query.capture_names()[capture.index as usize];

                if capture_name == "path" {
                    path_node = Some(capture.node);
                }
            }

            // If we found a path node, create an Import object
            if let Some(node) = path_node {
                let path = content[node.byte_range()].to_string();
                let line = node.start_position().row;

                // Determine if this is a relative import
                let is_relative = path.starts_with("super::")
                    || path.starts_with("self::")
                    || path.starts_with("crate::");

                // Extract the module name if possible
                let name = path.split("::").last().map(String::from);

                imports.push(Import {
                    path,
                    name,
                    line,
                    is_relative,
                });
            }
        }

        Ok(imports)
    }

    /// Extract variables from the syntax tree
    ///
    /// This walks the syntax tree looking for variable declarations (let statements)
    /// and extracts information about each variable.
    fn extract_variables(&self, tree: &Tree, content: &str) -> Result<Vec<Variable>> {
        let mut variables = Vec::new();
        let root_node = tree.root_node();

        // Look for let statements throughout the tree
        for node in self.iter_tree(&root_node) {
            if node.kind() == "let_declaration" {
                let mut var_name = None;
                let mut var_type = None;
                let mut init_value = None;

                // Extract pattern (variable name)
                if let Some(pattern) = node.child_by_field_name("pattern") {
                    if pattern.kind() == "identifier" {
                        var_name = Some(content[pattern.byte_range()].to_string());
                    }
                }

                // Extract type
                if let Some(type_node) = node.child_by_field_name("type") {
                    var_type = Some(content[type_node.byte_range()].to_string());
                }

                // Extract initialization value
                if let Some(value) = node.child_by_field_name("value") {
                    init_value = Some(content[value.byte_range()].to_string());
                }

                // If we found a variable name, create a Variable object
                if let Some(name) = var_name {
                    variables.push(Variable {
                        name,
                        var_type,
                        line: node.start_position().row,
                        is_public: false, // Local variables are never public
                        init_value,
                    });
                }
            } else if node.kind() == "const_item" || node.kind() == "static_item" {
                // Also capture const and static declarations
                let mut var_name = None;
                let mut var_type = None;
                let mut init_value = None;

                // Extract name
                for i in 0..node.child_count() {
                    if let Some(child) = node.child(i) {
                        if child.kind() == "identifier" {
                            var_name = Some(content[child.byte_range()].to_string());
                        } else if child.kind() == "type_identifier"
                            || child.kind() == "primitive_type"
                        {
                            var_type = Some(content[child.byte_range()].to_string());
                        } else if child.kind() == "=" {
                            // Next sibling should be the value
                            if let Some(value_node) = node.child(i + 1) {
                                init_value = Some(content[value_node.byte_range()].to_string());
                            }
                        }
                    }
                }

                // If we found a name, create a Variable object
                if let Some(name) = var_name {
                    variables.push(Variable {
                        name,
                        var_type,
                        line: node.start_position().row,
                        is_public: self.is_node_public(node, content),
                        init_value,
                    });
                }
            }
        }

        Ok(variables)
    }

    /// Calculate cyclomatic complexity for a code block
    ///
    /// This analyzes code to determine the cyclomatic complexity,
    /// a quantitative measure of the number of linearly independent paths
    /// through the code.
    fn calculate_cyclomatic_complexity(&self, body: &str) -> usize {
        // Count control flow keywords and operators that create branches
        let control_flow_patterns = [
            "if ", "else ", "match ", "for ", "while ", "loop", "&&", "||",
            "?",        // Error propagation creates branches
            "return",   // Early returns create branches
            "break",    // Loop breaks create branches
            "continue", // Loop continues create branches
        ];

        let mut complexity = 1; // Base complexity

        for pattern in control_flow_patterns {
            let count = body.matches(pattern).count();
            complexity += count;
        }

        // Handle closures which can also add complexity
        complexity += body.matches("|").count() / 2; // Pairs of "|" likely indicate closures

        complexity
    }

    /// Calculate cognitive complexity for a code block
    ///
    /// This analyzes code to determine cognitive complexity,
    /// a measure of how difficult the code is to understand.
    fn calculate_cognitive_complexity(&self, body: &str) -> usize {
        // Start with zero complexity
        let mut complexity = 0;

        // Basic control flow structures
        complexity += body.matches("if ").count();
        complexity += body.matches("else ").count();
        complexity += body.matches("match ").count() * 2; // Match is more complex
        complexity += body.matches("for ").count();
        complexity += body.matches("while ").count();
        complexity += body.matches("loop").count();

        // Logical operators that require mental context tracking
        complexity += body.matches("&&").count();
        complexity += body.matches("||").count();

        // Error handling complexity
        complexity += body.matches("Result<").count();
        complexity += body.matches("Option<").count();
        complexity += body.matches("?").count();

        // Function calls and method chaining
        complexity += body.matches("(").count(); // Function calls
        complexity += body.matches(".").count() / 2; // Method chaining (divide by 2 for rough estimate)

        // Nesting increases complexity
        let lines: Vec<&str> = body.lines().collect();
        let mut nesting_level = 0;

        for line in lines {
            let opening_braces = line.matches('{').count();
            let closing_braces = line.matches('}').count();

            // Update nesting level
            nesting_level += opening_braces as isize - closing_braces as isize;

            // Each control flow at higher nesting increases complexity more
            if line.contains("if ")
                || line.contains("else ")
                || line.contains("match ")
                || line.contains("for ")
                || line.contains("while ")
                || line.contains("loop")
            {
                // Add complexity based on nesting level
                complexity += nesting_level as usize;
            }
        }

        complexity
    }

    /// Check if a node is public (has a pub modifier)
    ///
    /// This determines whether a declaration (function, struct, etc.)
    /// is publicly visible or private.
    fn is_node_public(&self, node: tree_sitter::Node<'_>, content: &str) -> bool {
        // If node is not the main declaration, go to parent
        let declaration_node = if node.kind() == "identifier" || node.kind() == "type_identifier" {
            match node.parent() {
                Some(parent) => parent,
                None => return false,
            }
        } else {
            node
        };

        // Check for "pub" modifier
        for i in 0..declaration_node.child_count() {
            if let Some(child) = declaration_node.child(i) {
                if child.kind() == "visibility_modifier" {
                    let visibility = content[child.byte_range()].to_string();
                    return visibility.contains("pub");
                }
            }
        }

        false
    }

    /// Iterator over all nodes in a tree
    ///
    /// This provides a way to walk through all nodes in the syntax tree
    /// to find items that might not be directly accessible via queries.
    fn iter_tree<'a>(
        &self,
        node: &'a tree_sitter::Node,
    ) -> impl Iterator<Item = tree_sitter::Node<'a>> {
        struct NodeIterator<'a> {
            stack: Vec<tree_sitter::Node<'a>>,
        }

        impl<'a> Iterator for NodeIterator<'a> {
            type Item = tree_sitter::Node<'a>;

            fn next(&mut self) -> Option<Self::Item> {
                if let Some(node) = self.stack.pop() {
                    // Add all children to stack in reverse order
                    for i in (0..node.child_count()).rev() {
                        if let Some(child) = node.child(i) {
                            self.stack.push(child);
                        }
                    }
                    Some(node)
                } else {
                    None
                }
            }
        }

        NodeIterator { stack: vec![*node] }
    }

    /// Extract dependencies from a syntax tree
    ///
    /// This analyzes the code to find all dependencies including imports,
    /// function calls, and type references to create a complete dependency graph.
    fn extract_dependencies_from_tree(
        &self,
        tree: &Tree,
        content: &str,
        source_path: &Path,
        project_root: &Path,
    ) -> Result<Vec<Dependency>> {
        let mut dependencies = Vec::new();

        // Extract import dependencies
        let imports = self.extract_imports(tree, content)?;
        for import in imports {
            // Convert import path to file path
            let target_path = self.resolve_import_path(&import.path, source_path, project_root)?;

            dependencies.push(Dependency {
                source: source_path.to_path_buf(),
                target: target_path,
                dependency_type: DependencyType::Import,
                line: Some(import.line),
                info: Some(format!("Import: {}", import.path)),
            });
        }

        // Extract function call dependencies
        let mut query_cursor = QueryCursor::new();
        let mut matches = query_cursor.matches(
            &self.queries.function_call_query,
            tree.root_node(),
            content.as_bytes(),
        );

        while let Some(match_) = matches.next() {
            let mut function_name_node = None;
            let mut module_name_node = None;

            // Process each capture in the match
            for capture in match_.captures {
                let capture_name =
                    self.queries.function_call_query.capture_names()[capture.index as usize];

                if capture_name == "function_name" {
                    function_name_node = Some(capture.node);
                } else if capture_name == "module_name" {
                    module_name_node = Some(capture.node);
                }
            }

            // Process function calls
            if let Some(func_node) = function_name_node {
                let function_name = content[func_node.byte_range()].to_string();
                let module_name = module_name_node.map(|n| content[n.byte_range()].to_string());

                // If we have a module name, try to resolve it to a file
                if let Some(module) = &module_name {
                    match self.resolve_module_path(module, source_path, project_root) {
                        Ok(target_path) => {
                            dependencies.push(Dependency {
                                source: source_path.to_path_buf(),
                                target: target_path,
                                dependency_type: DependencyType::FunctionCall,
                                line: Some(func_node.start_position().row),
                                info: Some(format!("Function call: {}::{}", module, function_name)),
                            });
                        }
                        Err(e) => {
                            debug!(
                                "Could not resolve module '{}' for function call '{}': {}",
                                module, function_name, e
                            );
                        }
                    }
                } else {
                    // For local function calls, we still record them but don't change the target
                    dependencies.push(Dependency {
                        source: source_path.to_path_buf(),
                        target: source_path.to_path_buf(), // Same file for local calls
                        dependency_type: DependencyType::FunctionCall,
                        line: Some(func_node.start_position().row),
                        info: Some(format!("Local function call: {}", function_name)),
                    });
                }
            }
        }

        // Extract type reference dependencies
        let mut matches = query_cursor.matches(
            &self.queries.type_reference_query,
            tree.root_node(),
            content.as_bytes(),
        );

        while let Some(match_) = matches.next() {
            let mut type_name_node = None;

            // Process each capture in the match
            for capture in match_.captures {
                let capture_name =
                    self.queries.type_reference_query.capture_names()[capture.index as usize];

                if capture_name == "type_name" {
                    type_name_node = Some(capture.node);
                }
            }

            // Process type references
            if let Some(type_node) = type_name_node {
                let type_name = content[type_node.byte_range()].to_string();

                // Skip primitive types
                if [
                    "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128",
                    "usize", "f32", "f64", "bool", "char", "str", "String",
                ]
                .contains(&type_name.as_str())
                {
                    continue;
                }

                // Try to resolve the type to a file
                // This is a best-effort approach as fully resolving types requires
                // a complete type system understanding
                if let Ok(target_path) =
                    self.resolve_type_path(&type_name, source_path, project_root)
                {
                    dependencies.push(Dependency {
                        source: source_path.to_path_buf(),
                        target: target_path,
                        dependency_type: DependencyType::TypeUsage,
                        line: Some(type_node.start_position().row),
                        info: Some(format!("Type reference: {}", type_name)),
                    });
                }
            }
        }

        // Extract struct and trait dependencies
        self.extract_struct_trait_dependencies(
            tree,
            content,
            source_path,
            project_root,
            &mut dependencies,
        )?;

        Ok(dependencies)
    }

    /// Extract struct and trait dependencies
    ///
    /// This finds dependencies related to struct definitions, trait implementations,
    /// and trait bounds to build a more complete dependency graph.
    fn extract_struct_trait_dependencies(
        &self,
        tree: &Tree,
        content: &str,
        source_path: &Path,
        project_root: &Path,
        dependencies: &mut Vec<Dependency>,
    ) -> Result<()> {
        // Extract struct implementations and trait implementations
        let mut query_cursor = QueryCursor::new();

        // Process impl blocks
        let mut matches = query_cursor.matches(
            &self.queries.impl_query,
            tree.root_node(),
            content.as_bytes(),
        );

        while let Some(match_) = matches.next() {
            let mut trait_name_node = None;
            let mut type_name_node = None;

            // Process each capture in the match
            for capture in match_.captures {
                let capture_name = self.queries.impl_query.capture_names()[capture.index as usize];

                if capture_name == "trait_name" {
                    trait_name_node = Some(capture.node);
                } else if capture_name == "type_name" {
                    type_name_node = Some(capture.node);
                }
            }

            // If we have both trait and type, it's a trait implementation
            if let (Some(trait_node), Some(type_node)) = (trait_name_node, type_name_node) {
                let trait_name = content[trait_node.byte_range()].to_string();
                let type_name = content[type_node.byte_range()].to_string();

                // Try to resolve the trait to a file
                if let Ok(trait_path) =
                    self.resolve_type_path(&trait_name, source_path, project_root)
                {
                    dependencies.push(Dependency {
                        source: source_path.to_path_buf(),
                        target: trait_path,
                        dependency_type: DependencyType::Implementation,
                        line: Some(trait_node.start_position().row),
                        info: Some(format!(
                            "Trait implementation: {} for {}",
                            trait_name, type_name
                        )),
                    });
                }

                // Try to resolve the type to a file
                if let Ok(type_path) = self.resolve_type_path(&type_name, source_path, project_root)
                {
                    dependencies.push(Dependency {
                        source: source_path.to_path_buf(),
                        target: type_path,
                        dependency_type: DependencyType::Implementation,
                        line: Some(type_node.start_position().row),
                        info: Some(format!("Type implementation: {}", type_name)),
                    });
                }
            }
            // If we only have the type, it's a method implementation
            else if let Some(type_node) = type_name_node {
                let type_name = content[type_node.byte_range()].to_string();

                // Try to resolve the type to a file
                if let Ok(type_path) = self.resolve_type_path(&type_name, source_path, project_root)
                {
                    dependencies.push(Dependency {
                        source: source_path.to_path_buf(),
                        target: type_path,
                        dependency_type: DependencyType::Implementation,
                        line: Some(type_node.start_position().row),
                        info: Some(format!("Implementation for: {}", type_name)),
                    });
                }
            }
        }

        // Process struct definitions to look for field types
        let mut matches = query_cursor.matches(
            &self.queries.struct_query,
            tree.root_node(),
            content.as_bytes(),
        );

        while let Some(match_) = matches.next() {
            let mut struct_name_node = None;
            let mut body_node = None;

            // Process each capture in the match
            for capture in match_.captures {
                let capture_name =
                    self.queries.struct_query.capture_names()[capture.index as usize];

                if capture_name == "struct_name" {
                    struct_name_node = Some(capture.node);
                } else if capture_name == "body" {
                    body_node = Some(capture.node);
                }
            }

            // If we have a struct body, extract field types
            if let (Some(_struct_node), Some(body_node)) = (struct_name_node, body_node) {
                // Process each field in the struct
                for i in 0..body_node.child_count() {
                    if let Some(field) = body_node.child(i) {
                        if field.kind() == "field_declaration" {
                            // Extract field type
                            for j in 0..field.child_count() {
                                if let Some(child) = field.child(j) {
                                    if child.kind() == "type_identifier" {
                                        let type_name = content[child.byte_range()].to_string();

                                        // Try to resolve the field type to a file
                                        if let Ok(type_path) = self.resolve_type_path(
                                            &type_name,
                                            source_path,
                                            project_root,
                                        ) {
                                            dependencies.push(Dependency {
                                                source: source_path.to_path_buf(),
                                                target: type_path,
                                                dependency_type: DependencyType::TypeUsage,
                                                line: Some(child.start_position().row),
                                                info: Some(format!("Field type: {}", type_name)),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Process enum definitions for variant types
        let mut matches = query_cursor.matches(
            &self.queries.enum_query,
            tree.root_node(),
            content.as_bytes(),
        );

        while let Some(match_) = matches.next() {
            let mut enum_name_node = None;
            let mut body_node = None;

            // Process each capture in the match
            for capture in match_.captures {
                let capture_name = self.queries.enum_query.capture_names()[capture.index as usize];

                if capture_name == "enum_name" {
                    enum_name_node = Some(capture.node);
                } else if capture_name == "body" {
                    body_node = Some(capture.node);
                }
            }

            // If we have an enum body, extract variant types
            if let (Some(_enum_node), Some(body_node)) = (enum_name_node, body_node) {
                // Process each variant in the enum
                for i in 0..body_node.child_count() {
                    if let Some(variant) = body_node.child(i) {
                        if variant.kind() == "enum_variant" {
                            // Look for tuple or struct variants with type references
                            for j in 0..variant.child_count() {
                                if let Some(child) = variant.child(j) {
                                    if child.kind() == "type_identifier" {
                                        let type_name = content[child.byte_range()].to_string();

                                        // Try to resolve the variant type to a file
                                        if let Ok(type_path) = self.resolve_type_path(
                                            &type_name,
                                            source_path,
                                            project_root,
                                        ) {
                                            dependencies.push(Dependency {
                                                source: source_path.to_path_buf(),
                                                target: type_path,
                                                dependency_type: DependencyType::TypeUsage,
                                                line: Some(child.start_position().row),
                                                info: Some(format!(
                                                    "Enum variant type: {}",
                                                    type_name
                                                )),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Resolve an import path to a file path
    ///
    /// This converts Rust module path syntax (e.g., 'std::fs::File')
    /// to actual file paths within the project structure.
    fn resolve_import_path(
        &self,
        import_path: &str,
        source_path: &Path,
        project_root: &Path,
    ) -> Result<PathBuf> {
        // Handle standard library imports
        if ["std", "core", "alloc"].contains(&import_path.split("::").next().unwrap_or("")) {
            // Standard library - use a special path
            return Ok(PathBuf::from("rust")
                .join("std")
                .join(import_path.replace("::", "/")));
        }

        // Look for Cargo.toml to find crate root
        let mut current_dir = source_path.parent().unwrap_or(Path::new(""));
        let mut cargo_toml_path = None;

        while current_dir.starts_with(project_root) {
            let cargo_path = current_dir.join("Cargo.toml");
            if cargo_path.exists() {
                cargo_toml_path = Some(cargo_path);
                break;
            }

            if let Some(parent) = current_dir.parent() {
                current_dir = parent;
            } else {
                break;
            }
        }

        // Determine crate root
        let crate_root = if let Some(ref cargo_path) = cargo_toml_path {
            cargo_path.parent().unwrap_or(project_root)
        } else {
            project_root
        };

        // Handle crate-relative paths
        let parts: Vec<&str> = import_path.split("::").collect();

        if parts.is_empty() {
            return Err(ZseiError::Analyzer(format!(
                "Invalid import path: {}",
                import_path
            )));
        }

        // Check for external crate
        let is_external = {
            if let Some(ref cargo_path) = cargo_toml_path {
                // Parse Cargo.toml to get dependencies
                if let Ok(content) = fs::read_to_string(cargo_path) {
                    content.contains(&format!("name = \"{}\"", parts[0]))
                        || content.contains(&format!("\"{} ", parts[0]))
                } else {
                    false
                }
            } else {
                false
            }
        };

        if is_external {
            // External crate - use a special path
            Ok(PathBuf::from("external").join(parts.join("/")))
        } else {
            // Try different potential paths
            let mut potential_paths = Vec::new();

            // Special case for 'crate' paths
            if import_path.starts_with("crate::") {
                // Remove 'crate::' prefix and navigate from crate root
                let path_without_prefix = import_path.replacen("crate::", "", 1);
                let module_path = path_without_prefix.replace("::", "/");

                // Check for module as file
                potential_paths.push(crate_root.join("src").join(format!("{}.rs", module_path)));

                // Check for module as directory with mod.rs
                potential_paths.push(crate_root.join("src").join(&module_path).join("mod.rs"));
            }
            // Special case for 'self' or 'super' paths
            else if import_path.starts_with("self::") || import_path.starts_with("super::") {
                let source_dir = source_path.parent().unwrap_or(Path::new(""));

                if import_path.starts_with("self::") {
                    // Navigate from current directory
                    let path_without_prefix = import_path.replacen("self::", "", 1);
                    let module_path = path_without_prefix.replace("::", "/");

                    potential_paths.push(source_dir.join(format!("{}.rs", module_path)));
                    potential_paths.push(source_dir.join(&module_path).join("mod.rs"));
                } else {
                    // Navigate from parent directory
                    let path_without_prefix = import_path.replacen("super::", "", 1);
                    let module_path = path_without_prefix.replace("::", "/");

                    if let Some(parent_dir) = source_dir.parent() {
                        potential_paths.push(parent_dir.join(format!("{}.rs", module_path)));
                        potential_paths.push(parent_dir.join(&module_path).join("mod.rs"));
                    }
                }
            }
            // Regular paths
            else {
                // Check for src/lib.rs or src/main.rs
                potential_paths.push(crate_root.join("src").join("lib.rs"));
                potential_paths.push(crate_root.join("src").join("main.rs"));

                // Check for module file
                let module_path = parts[0]; // First part of the path
                potential_paths.push(crate_root.join("src").join(format!("{}.rs", module_path)));

                // Check for module directory with mod.rs
                potential_paths.push(crate_root.join("src").join(module_path).join("mod.rs"));

                // Check for nested modules
                if parts.len() > 1 {
                    let full_module_path = parts.join("/");
                    potential_paths.push(
                        crate_root
                            .join("src")
                            .join(format!("{}.rs", full_module_path)),
                    );
                    potential_paths.push(
                        crate_root
                            .join("src")
                            .join(&full_module_path)
                            .join("mod.rs"),
                    );
                }
            }

            // Try to find matching path
            for path in potential_paths {
                if path.exists() {
                    return Ok(path);
                }
            }

            // If not found, return a best guess path
            let module_path = parts.join("/");
            Ok(crate_root.join("src").join(format!("{}.rs", module_path)))
        }
    }

    /// Resolve a module path to a file path
    ///
    /// This converts module identifiers in function calls (e.g., 'module::function()')
    /// to file paths for tracking dependencies.
    fn resolve_module_path(
        &self,
        module_name: &str,
        source_path: &Path,
        project_root: &Path,
    ) -> Result<PathBuf> {
        let source_dir = source_path.parent().unwrap_or(Path::new(""));

        // Try module.rs in the same directory
        let module_path = source_dir.join(format!("{}.rs", module_name));
        if module_path.exists() {
            return Ok(module_path);
        }

        // Try module/mod.rs
        let module_dir_path = source_dir.join(module_name).join("mod.rs");
        if module_dir_path.exists() {
            return Ok(module_dir_path);
        }

        // Try to find the module in standard library or external crates
        let import_path = format!("{}::", module_name);
        self.resolve_import_path(&import_path, source_path, project_root)
    }

    /// Resolve a type path to a file path
    ///
    /// This attempts to find where a type is defined within the project
    /// to map type dependencies correctly.
    fn resolve_type_path(
        &self,
        type_name: &str,
        source_path: &Path,
        project_root: &Path,
    ) -> Result<PathBuf> {
        // Try to find type definition in the same file
        // Types that are defined in the same file don't create external dependencies
        // Since we can't easily check this from the analyzer, we'll assume types
        // following certain naming patterns might be local types

        // If type name contains '::', it's likely a path to another module
        if type_name.contains("::") {
            return self.resolve_import_path(type_name, source_path, project_root);
        }

        // Check if the type might be from std library or other common libraries
        let std_types = [
            "String", "Vec", "HashMap", "BTreeMap", "HashSet", "BTreeSet", "Option", "Result",
            "Box", "Rc", "Arc", "Cell", "RefCell", "Mutex", "RwLock", "Cow", "PathBuf", "Path",
        ];

        if std_types.contains(&type_name) {
            return Ok(PathBuf::from("rust").join("std"));
        }

        // Best effort: search for the type in the current directory
        let source_dir = source_path.parent().unwrap_or(Path::new(""));

        // Try to find a file named after the type (common Rust convention)
        let snake_case = to_snake_case(type_name);
        if !snake_case.is_empty() {
            let type_file = source_dir.join(format!("{}.rs", snake_case));
            if type_file.exists() {
                return Ok(type_file);
            }
        }

        // If not found, return the source file itself as the best guess
        // This is not always correct but better than no dependency
        Ok(source_path.to_path_buf())
    }

    /// Calculate maintainability index
    fn calculate_maintainability_index(
        &self,
        loc: usize,
        comment_lines: usize,
        complexity: usize,
    ) -> f64 {
        // Halstead volume approximation based on code size
        let halstead_volume = (loc as f64) * 3.0; // simplified approximation

        // Comment ratio
        let comment_ratio = if loc > 0 {
            comment_lines as f64 / loc as f64
        } else {
            0.0
        };

        // Standard maintainability index formula
        // MI = 171 - 5.2 * ln(V) - 0.23 * G - 16.2 * ln(LOC) + 50 * sin(sqrt(2.4 * CM))
        let mi = 171.0
            - 5.2 * halstead_volume.ln()
            - 0.23 * complexity as f64
            - 16.2 * (loc as f64).ln()
            + 50.0 * (2.4 * comment_ratio).sqrt().sin();

        // Normalize to 0-100 scale
        (mi * 100.0 / 171.0).min(100.0).max(0.0)
    }
}

/// Convert CamelCase to snake_case
///
/// This is a utility function to help convert type names to potential file names.
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_is_upper = true; // Treat first char specially

    for c in s.chars() {
        if c.is_uppercase() {
            if !prev_is_upper && !result.is_empty() {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
            prev_is_upper = true;
        } else {
            result.push(c);
            prev_is_upper = false;
        }
    }

    result
}

#[async_trait]
impl LanguageAnalyzer for RustAnalyzer {
    fn language_name(&self) -> &'static str {
        "Rust"
    }

    fn supported_extensions(&self) -> &[&'static str] {
        &["rs"]
    }

    async fn analyze_file(&self, path: &Path) -> Result<FileAnalysis> {
        info!("Analyzing Rust file: {}", path.display());
        let content = fs::read_to_string(path).map_err(|e| {
            ZseiError::Analyzer(format!("Failed to read file {}: {}", path.display(), e))
        })?;

        // Clone parser as it's not thread-safe
        let mut parser = Parser::new();
        let language = tree_sitter_rust::LANGUAGE;
        parser
            .set_language(&language.into())
            .expect("Failed to set Rust language");

        // Parse file
        let tree = parser.parse(content.as_bytes(), None).ok_or_else(|| {
            ZseiError::Analyzer(format!("Failed to parse file {}", path.display()))
        })?;

        // Extract code elements
        let functions = self.extract_functions(&tree, &content)?;
        let imports = self.extract_imports(&tree, &content)?;
        let variables = self.extract_variables(&tree, &content)?;

        // Calculate metrics
        let loc = content.lines().count();
        let comment_lines = content
            .lines()
            .filter(|l| l.trim().starts_with("//") || l.trim().starts_with("/*"))
            .count();

        // Calculate aggregate complexity
        let complexity = functions.iter().map(|f| f.metrics.complexity).sum();

        // Calculate maintainability index
        let maintainability_index =
            self.calculate_maintainability_index(loc, comment_lines, complexity);

        let metrics = CodeMetrics {
            loc,
            comment_lines,
            function_count: functions.len(),
            class_count: 0, // Rust doesn't have classes, but we could count structs
            import_count: imports.len(),
            variable_count: variables.len(),
            complexity,
            maintainability_index,
        };

        debug!(
            "Analysis complete: {} functions, {} imports, {} variables",
            functions.len(),
            imports.len(),
            variables.len()
        );

        Ok(FileAnalysis {
            path: path.to_path_buf(),
            language: "Rust".to_string(),
            content: Some(content),
            functions,
            classes: Vec::new(), // Rust doesn't have classes in the traditional sense
            variables,
            imports,
            metrics,
        })
    }

    async fn extract_dependencies(&self, path: &Path) -> Result<Vec<Dependency>> {
        info!("Extracting dependencies from Rust file: {}", path.display());
        let content = fs::read_to_string(path).map_err(|e| {
            ZseiError::Analyzer(format!("Failed to read file {}: {}", path.display(), e))
        })?;

        // Clone parser as it's not thread-safe
        let mut parser = Parser::new();
        let language = tree_sitter_rust::LANGUAGE;
        parser
            .set_language(&language.into())
            .expect("Failed to set Rust language");

        // Parse file
        let tree = parser.parse(content.as_bytes(), None).ok_or_else(|| {
            ZseiError::Analyzer(format!("Failed to parse file {}", path.display()))
        })?;

        // Extract dependencies
        let dependencies =
            self.extract_dependencies_from_tree(&tree, &content, path, self.config.project_root())?;

        debug!(
            "Extracted {} dependencies from {}",
            dependencies.len(),
            path.display()
        );

        Ok(dependencies)
    }
}
