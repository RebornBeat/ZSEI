//! Common analyzer components
//!
//! This module provides common components for code analyzers.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::core::project::ProjectStructure;

/// Analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// File analyses
    pub file_analyses: Vec<FileAnalysis>,

    /// Dependencies
    pub dependencies: Vec<Dependency>,

    /// Code graph
    pub graph: CodeGraph,

    /// Project structure
    pub project_structure: Option<ProjectStructure>,
}

impl AnalysisResult {
    /// Save analysis result to a file
    pub fn save_to_file(&self, path: &Path) -> crate::errors::Result<()> {
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                crate::ZseiError::Analyzer(format!("Failed to create directories: {}", e))
            })?;
        }

        // Serialize and save
        let content = serde_json::to_string_pretty(self).map_err(|e| {
            crate::ZseiError::Analyzer(format!("Failed to serialize analysis result: {}", e))
        })?;

        std::fs::write(path, content).map_err(|e| {
            crate::ZseiError::Analyzer(format!("Failed to write analysis result: {}", e))
        })?;

        Ok(())
    }

    /// Load analysis result from a file
    pub fn load_from_file(path: &Path) -> crate::errors::Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            crate::ZseiError::Analyzer(format!("Failed to read analysis result: {}", e))
        })?;

        let result = serde_json::from_str(&content).map_err(|e| {
            crate::ZseiError::Analyzer(format!("Failed to deserialize analysis result: {}", e))
        })?;

        Ok(result)
    }

    /// Get file analysis by path
    pub fn get_file_analysis(&self, path: &Path) -> Option<&FileAnalysis> {
        self.file_analyses.iter().find(|a| a.path == path)
    }

    /// Get dependencies for a file
    pub fn get_dependencies_for_file(&self, path: &Path) -> Vec<&Dependency> {
        self.dependencies
            .iter()
            .filter(|d| d.source == path)
            .collect()
    }

    /// Get dependents for a file
    pub fn get_dependents_for_file(&self, path: &Path) -> Vec<&Dependency> {
        self.dependencies
            .iter()
            .filter(|d| d.target == path)
            .collect()
    }

    /// Get all files in the analysis
    pub fn get_all_files(&self) -> HashSet<&PathBuf> {
        let mut files = HashSet::new();

        for analysis in &self.file_analyses {
            files.insert(&analysis.path);
        }

        for dep in &self.dependencies {
            files.insert(&dep.source);
            files.insert(&dep.target);
        }

        files
    }
}

/// File analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAnalysis {
    /// File path
    pub path: PathBuf,

    /// Language
    pub language: String,

    /// File content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// Functions
    pub functions: Vec<Function>,

    /// Classes
    pub classes: Vec<Class>,

    /// Variables
    pub variables: Vec<Variable>,

    /// Imports
    pub imports: Vec<Import>,

    /// Metrics
    pub metrics: CodeMetrics,
}

/// Function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    /// Function name
    pub name: String,

    /// Function signature
    pub signature: String,

    /// Start line
    pub start_line: usize,

    /// End line
    pub end_line: usize,

    /// Function body
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,

    /// Return type
    pub return_type: Option<String>,

    /// Parameters
    pub parameters: Vec<Parameter>,

    /// Whether the function is public
    pub is_public: bool,

    /// Metrics
    pub metrics: FunctionMetrics,
}

/// Parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    /// Parameter name
    pub name: String,

    /// Parameter type
    pub param_type: Option<String>,

    /// Default value
    pub default_value: Option<String>,

    /// Parameter position
    pub position: usize,
}

/// Class
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Class {
    /// Class name
    pub name: String,

    /// Start line
    pub start_line: usize,

    /// End line
    pub end_line: usize,

    /// Methods
    pub methods: Vec<Function>,

    /// Properties
    pub properties: Vec<Variable>,

    /// Base classes
    pub base_classes: Vec<String>,

    /// Whether the class is public
    pub is_public: bool,

    /// Metrics
    pub metrics: ClassMetrics,
}

/// Variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    /// Variable name
    pub name: String,

    /// Variable type
    pub var_type: Option<String>,

    /// Line number
    pub line: usize,

    /// Whether the variable is public
    pub is_public: bool,

    /// Initialization value
    pub init_value: Option<String>,
}

/// Import
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Import {
    /// Import path
    pub path: String,

    /// Import name
    pub name: Option<String>,

    /// Line number
    pub line: usize,

    /// Whether it's a relative import
    pub is_relative: bool,
}

/// Code metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeMetrics {
    /// Lines of code
    pub loc: usize,

    /// Lines of comment
    pub comment_lines: usize,

    /// Number of functions
    pub function_count: usize,

    /// Number of classes
    pub class_count: usize,

    /// Number of imports
    pub import_count: usize,

    /// Number of variables
    pub variable_count: usize,

    /// Complexity
    pub complexity: usize,

    /// Maintainability index
    pub maintainability_index: f64,
}

/// Function metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionMetrics {
    /// Lines of code
    pub loc: usize,

    /// Cyclomatic complexity
    pub complexity: usize,

    /// Parameter count
    pub parameter_count: usize,

    /// Cognitive complexity
    pub cognitive_complexity: usize,
}

/// Class metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassMetrics {
    /// Lines of code
    pub loc: usize,

    /// Number of methods
    pub method_count: usize,

    /// Number of properties
    pub property_count: usize,

    /// Inheritance depth
    pub inheritance_depth: usize,

    /// Class cohesion
    pub cohesion: f64,
}

/// Dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// Source file
    pub source: PathBuf,

    /// Target file
    pub target: PathBuf,

    /// Dependency type
    pub dependency_type: DependencyType,

    /// Line number
    pub line: Option<usize>,

    /// Additional information
    pub info: Option<String>,
}

/// Dependency type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DependencyType {
    /// Import dependency
    Import,

    /// Function call
    FunctionCall,

    /// Inheritance
    Inheritance,

    /// Implementation
    Implementation,

    /// Variable usage
    VariableUsage,

    /// Type usage
    TypeUsage,
}

/// Code graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGraph {
    /// Nodes (files)
    pub nodes: HashMap<PathBuf, GraphNode>,

    /// Edges (dependencies)
    pub edges: Vec<GraphEdge>,
}

/// Graph node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// Node ID (file path)
    pub id: PathBuf,

    /// Node label
    pub label: String,

    /// Node type
    pub node_type: String,

    /// Additional properties
    pub properties: HashMap<String, String>,
}

/// Graph edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Source node
    pub source: PathBuf,

    /// Target node
    pub target: PathBuf,

    /// Edge label
    pub label: String,

    /// Edge type
    pub edge_type: DependencyType,

    /// Edge weight
    pub weight: f64,

    /// Additional properties
    pub properties: HashMap<String, String>,
}

impl CodeGraph {
    /// Create a new code graph from dependencies
    pub fn new(dependencies: &[Dependency]) -> Self {
        let mut nodes = HashMap::new();
        let mut edges = Vec::new();

        // Create nodes for all files
        let mut file_set = HashSet::new();

        for dep in dependencies {
            file_set.insert(dep.source.clone());
            file_set.insert(dep.target.clone());
        }

        for file in file_set {
            let node = GraphNode {
                id: file.clone(),
                label: file
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown")
                    .to_string(),
                node_type: "File".to_string(),
                properties: HashMap::new(),
            };

            nodes.insert(file, node);
        }

        // Create edges for dependencies
        for dep in dependencies {
            let edge = GraphEdge {
                source: dep.source.clone(),
                target: dep.target.clone(),
                label: format!("{:?}", dep.dependency_type),
                edge_type: dep.dependency_type.clone(),
                weight: 1.0,
                properties: HashMap::new(),
            };

            edges.push(edge);
        }

        Self { nodes, edges }
    }

    /// Get outgoing dependencies for a file
    pub fn get_outgoing_dependencies(&self, file: &Path) -> Vec<&GraphEdge> {
        self.edges.iter().filter(|e| e.source == file).collect()
    }

    /// Get incoming dependencies for a file
    pub fn get_incoming_dependencies(&self, file: &Path) -> Vec<&GraphEdge> {
        self.edges.iter().filter(|e| e.target == file).collect()
    }

    /// Get dependency path between files
    pub fn get_dependency_path(&self, source: &Path, target: &Path) -> Option<Vec<GraphEdge>> {
        // Simple breadth-first search to find a path
        let mut queue = std::collections::VecDeque::new();
        let mut visited = HashSet::new();
        let mut predecessor = HashMap::new();

        queue.push_back(source.to_path_buf());
        visited.insert(source.to_path_buf());

        while let Some(current) = queue.pop_front() {
            if &current == target {
                // Reconstruct path
                let mut path = Vec::new();
                let mut current = current;

                while &current != source {
                    let (pred, edge): &(PathBuf, GraphEdge) = predecessor.get(&current)?;
                    path.push(edge.clone());
                    current = pred.clone();
                }

                path.reverse();
                return Some(path);
            }

            for edge in self.get_outgoing_dependencies(&current) {
                if !visited.contains(&edge.target) {
                    visited.insert(edge.target.clone());
                    queue.push_back(edge.target.clone());
                    predecessor.insert(edge.target.clone(), (current.clone(), edge.clone()));
                }
            }
        }

        None
    }
}
