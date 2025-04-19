//! Project management for ZSEI
//!
//! This module provides project management for the Zero-Shot
//! Bolted Embedding Indexer, including file tracking, project structure,
//! and project state.

use chrono::{DateTime, Utc};
use ignore::{DirEntry, Walk, WalkBuilder};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::core::config::Config;
use crate::errors::{Result, ZseiError};

/// Project state struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectState {
    /// Last analysis time
    pub last_analysis: DateTime<Utc>,

    /// Last indexing time
    pub last_indexing: DateTime<Utc>,

    /// Last refactoring time
    pub last_refactoring: DateTime<Utc>,

    /// Analyzed files
    pub analyzed_files: HashMap<PathBuf, FileState>,

    /// Indexed files
    pub indexed_files: HashMap<PathBuf, FileState>,
}

/// File state struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileState {
    /// File path
    pub path: PathBuf,

    /// Last modified time
    pub last_modified: DateTime<Utc>,

    /// File hash
    pub hash: String,

    /// File language
    pub language: Option<String>,

    /// File size in bytes
    pub size: u64,
}

/// Project struct
pub struct Project {
    /// Project config
    config: Arc<Config>,

    /// Project state
    state: ProjectState,

    /// Project state file path
    state_path: PathBuf,
}

impl Project {
    /// Create a new project
    pub fn new(config: Arc<Config>) -> Result<Self> {
        let state_path = config.project_root().join(".zsei").join("state.json");

        // Try to load existing state
        let state = if state_path.exists() {
            let content = fs::read_to_string(&state_path)
                .map_err(|e| ZseiError::Config(format!("Failed to read state file: {}", e)))?;

            serde_json::from_str(&content)
                .map_err(|e| ZseiError::Config(format!("Failed to parse state file: {}", e)))?
        } else {
            ProjectState {
                last_analysis: Utc::now(),
                last_indexing: Utc::now(),
                last_refactoring: Utc::now(),
                analyzed_files: HashMap::new(),
                indexed_files: HashMap::new(),
            }
        };

        Ok(Self {
            config,
            state,
            state_path,
        })
    }

    /// Check if the project is initialized
    pub fn is_initialized(&self) -> Result<bool> {
        let zsei_dir = self.config.project_root().join(".zsei");
        Ok(zsei_dir.exists())
    }

    /// Initialize the project
    pub fn initialize(&self, path: &Path) -> Result<()> {
        // Create .zsei directory
        let zsei_dir = path.join(".zsei");
        fs::create_dir_all(&zsei_dir)
            .map_err(|e| ZseiError::Config(format!("Failed to create .zsei directory: {}", e)))?;

        // Create subdirectories
        fs::create_dir_all(zsei_dir.join("index"))
            .map_err(|e| ZseiError::Config(format!("Failed to create index directory: {}", e)))?;

        fs::create_dir_all(zsei_dir.join("metadata")).map_err(|e| {
            ZseiError::Config(format!("Failed to create metadata directory: {}", e))
        })?;

        fs::create_dir_all(zsei_dir.join("branches")).map_err(|e| {
            ZseiError::Config(format!("Failed to create branches directory: {}", e))
        })?;

        // Save initial state
        self.save_state()?;

        // Save initial config
        let config_path = zsei_dir.join("config.toml");
        self.config.save_to_file(&config_path)?;

        Ok(())
    }

    /// Save project state
    pub fn save_state(&self) -> Result<()> {
        // Create parent directories if they don't exist
        if let Some(parent) = self.state_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| ZseiError::Config(format!("Failed to create directories: {}", e)))?;
        }

        let content = serde_json::to_string_pretty(&self.state)
            .map_err(|e| ZseiError::Config(format!("Failed to serialize state: {}", e)))?;

        fs::write(&self.state_path, content)
            .map_err(|e| ZseiError::Config(format!("Failed to write state file: {}", e)))?;

        Ok(())
    }

    /// Check if the project has a file
    pub fn has_file(&self, relative_path: &str) -> bool {
        self.config.project_root().join(relative_path).exists()
    }

    /// Add a reference project path
    pub fn add_reference_path(&self, path: &Path) -> Result<()> {
        let mut config = (*self.config).clone();
        config.add_project_path(path.to_path_buf());

        let config_path = self.config.project_root().join(".zsei").join("config.toml");
        config.save_to_file(&config_path)?;

        Ok(())
    }

    /// Get files that need to be analyzed
    pub fn get_files_to_analyze(&self, incremental: bool) -> Result<Vec<PathBuf>> {
        self.get_files_to_process(incremental, &self.state.analyzed_files)
    }

    /// Get files that need to be indexed
    pub fn get_files_to_index(&self, incremental: bool) -> Result<Vec<PathBuf>> {
        self.get_files_to_process(incremental, &self.state.indexed_files)
    }

    /// Find files that need to be processed
    fn get_files_to_process(
        &self,
        incremental: bool,
        tracked_files: &HashMap<PathBuf, FileState>,
    ) -> Result<Vec<PathBuf>> {
        let mut files_to_process = Vec::new();

        // Get all files in the project
        let project_files = self.scan_project_files()?;

        for file_path in project_files {
            if self.should_process_file(&file_path, incremental, tracked_files)? {
                files_to_process.push(file_path);
            }
        }

        Ok(files_to_process)
    }

    /// Check if a file should be processed
    fn should_process_file(
        &self,
        path: &Path,
        incremental: bool,
        tracked_files: &HashMap<PathBuf, FileState>,
    ) -> Result<bool> {
        // Skip if excluded by pattern
        if self.is_excluded(path)? {
            return Ok(false);
        }

        // Always process if not incremental
        if !incremental {
            return Ok(true);
        }

        // Check if file is modified
        if let Some(state) = tracked_files.get(path) {
            let metadata = fs::metadata(path)
                .map_err(|e| ZseiError::Config(format!("Failed to get file metadata: {}", e)))?;

            let modified: DateTime<Utc> = metadata
                .modified()
                .map_err(|e| ZseiError::Config(format!("Failed to get modified time: {}", e)))?
                .into();

            // Process if modified after last tracking
            Ok(modified > state.last_modified)
        } else {
            // Always process new files
            Ok(true)
        }
    }

    /// Check if a file is excluded by patterns
    fn is_excluded(&self, path: &Path) -> Result<bool> {
        // Check file extension
        if let Some(ext) = path.extension() {
            if let Some(ext_str) = ext.to_str() {
                if !self
                    .config
                    .indexing
                    .include_extensions
                    .contains(&ext_str.to_string())
                {
                    return Ok(true);
                }
            }
        } else {
            // No extension, skip
            return Ok(true);
        }

        // Check exclude patterns
        for pattern in &self.config.indexing.exclude_patterns {
            if glob::Pattern::new(pattern)
                .map_err(|e| ZseiError::Config(format!("Invalid glob pattern: {}", e)))?
                .matches_path(path)
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Scan project files
    fn scan_project_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        // Scan main project directory
        self.scan_directory(self.config.project_root(), &mut files)?;

        // Scan additional project directories
        for additional_path in &self.config.additional_project_paths {
            self.scan_directory(additional_path, &mut files)?;
        }

        Ok(files)
    }

    /// Scan a directory for files
    fn scan_directory(&self, dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        let walker = WalkBuilder::new(dir)
            .hidden(false) // Don't skip hidden files
            .ignore(true) // Use .gitignore patterns
            .git_ignore(true) // Use .gitignore patterns
            .build();

        for entry in walker {
            match entry {
                Ok(entry) => {
                    if entry.file_type().map_or(false, |ft| ft.is_file()) {
                        files.push(entry.path().to_path_buf());
                    }
                }
                Err(err) => {
                    tracing::warn!("Error walking directory: {}", err);
                }
            }
        }

        Ok(())
    }

    /// Update file state after analysis
    pub fn update_analyzed_file(&mut self, path: &Path, language: Option<String>) -> Result<()> {
        let file_state = self.create_file_state(path, language)?;
        self.state
            .analyzed_files
            .insert(path.to_path_buf(), file_state);
        self.state.last_analysis = Utc::now();
        self.save_state()?;
        Ok(())
    }

    /// Update file state after indexing
    pub fn update_indexed_file(&mut self, path: &Path, language: Option<String>) -> Result<()> {
        let file_state = self.create_file_state(path, language)?;
        self.state
            .indexed_files
            .insert(path.to_path_buf(), file_state);
        self.state.last_indexing = Utc::now();
        self.save_state()?;
        Ok(())
    }

    /// Update file state
    fn create_file_state(&self, path: &Path, language: Option<String>) -> Result<FileState> {
        let metadata = fs::metadata(path)
            .map_err(|e| ZseiError::Config(format!("Failed to get file metadata: {}", e)))?;

        let modified: DateTime<Utc> = metadata
            .modified()
            .map_err(|e| ZseiError::Config(format!("Failed to get modified time: {}", e)))?
            .into();

        let content =
            fs::read(path).map_err(|e| ZseiError::Config(format!("Failed to read file: {}", e)))?;

        let hash = format!("{:x}", md5::compute(&content));

        Ok(FileState {
            path: path.to_path_buf(),
            last_modified: modified,
            hash,
            language,
            size: metadata.len(),
        })
    }

    /// Get all tracked file states
    pub fn get_tracked_files(&self) -> &HashMap<PathBuf, FileState> {
        &self.state.analyzed_files
    }

    /// Get project structure
    pub fn get_structure(&self) -> Result<ProjectStructure> {
        let mut structure = ProjectStructure {
            root: self.config.project_root().to_path_buf(),
            directories: HashMap::new(),
            files: HashMap::new(),
        };

        // Scan project files
        let files = self.scan_project_files()?;

        // Build directory structure
        for file_path in files {
            if self.is_excluded(&file_path)? {
                continue;
            }

            let relative_path = file_path
                .strip_prefix(self.config.project_root())
                .map_err(|e| ZseiError::Config(format!("Failed to get relative path: {}", e)))?;

            let metadata = fs::metadata(&file_path)
                .map_err(|e| ZseiError::Config(format!("Failed to get file metadata: {}", e)))?;

            // Add file to structure
            structure.add_file(
                relative_path.to_path_buf(),
                FileInfo {
                    path: relative_path.to_path_buf(),
                    language: self.detect_language(&file_path),
                    size: metadata.len(),
                },
            )?;
        }

        Ok(structure)
    }

    /// Detect language from file extension
    fn detect_language(&self, path: &Path) -> Option<String> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| match ext {
                "rs" => "Rust",
                "py" => "Python",
                "js" => "JavaScript",
                "ts" => "TypeScript",
                "go" => "Go",
                "c" | "h" => "C",
                "cpp" | "cc" | "cxx" | "hpp" | "hxx" => "C++",
                "java" => "Java",
                "kt" => "Kotlin",
                "cs" => "C#",
                "rb" => "Ruby",
                "php" => "PHP",
                "swift" => "Swift",
                "scala" => "Scala",
                "hs" => "Haskell",
                "ex" | "exs" => "Elixir",
                "erl" => "Erlang",
                "ml" | "mli" => "OCaml",
                "fs" | "fsx" => "F#",
                "clj" => "Clojure",
                "html" => "HTML",
                "css" => "CSS",
                "sh" | "bash" => "Shell",
                "sql" => "SQL",
                "md" | "markdown" => "Markdown",
                "json" => "JSON",
                "xml" => "XML",
                "yml" | "yaml" => "YAML",
                "toml" => "TOML",
                _ => "Unknown",
            })
            .map(String::from)
    }
}

/// Project structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStructure {
    /// Project root directory
    pub root: PathBuf,

    /// Directories in the project
    pub directories: HashMap<PathBuf, DirectoryInfo>,

    /// Files in the project
    pub files: HashMap<PathBuf, FileInfo>,
}

/// Directory information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryInfo {
    /// Directory path (relative to project root)
    pub path: PathBuf,

    /// Child directories
    pub child_directories: HashSet<PathBuf>,

    /// Child files
    pub child_files: HashSet<PathBuf>,
}

/// File information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// File path (relative to project root)
    pub path: PathBuf,

    /// File language
    pub language: Option<String>,

    /// File size in bytes
    pub size: u64,
}

impl ProjectStructure {
    /// Add a file to the structure
    pub fn add_file(&mut self, path: PathBuf, info: FileInfo) -> Result<()> {
        // Add file to files map
        self.files.insert(path.clone(), info);

        // Add directories in path
        let mut current_path = PathBuf::new();
        let parent = path.parent().unwrap_or(Path::new(""));

        for component in parent.components() {
            let component_path = PathBuf::from(component.as_os_str());

            // Skip empty components
            if component_path.as_os_str().is_empty() {
                continue;
            }

            // Build path incrementally
            current_path = if current_path.as_os_str().is_empty() {
                component_path.clone()
            } else {
                current_path.join(component_path)
            };

            // Add directory if it doesn't exist
            if !self.directories.contains_key(&current_path) {
                self.directories.insert(
                    current_path.clone(),
                    DirectoryInfo {
                        path: current_path.clone(),
                        child_directories: HashSet::new(),
                        child_files: HashSet::new(),
                    },
                );
            }

            // Add child directory to parent
            if let Some(parent_path) = current_path.parent() {
                if !parent_path.as_os_str().is_empty() {
                    let parent_path = parent_path.to_path_buf();
                    if let Some(parent_info) = self.directories.get_mut(&parent_path) {
                        parent_info.child_directories.insert(current_path.clone());
                    }
                }
            }
        }

        // Add file to parent directory
        if let Some(parent_path) = parent.to_path_buf().parent() {
            let parent_path = parent_path.to_path_buf();
            if let Some(parent_info) = self.directories.get_mut(&parent_path) {
                parent_info.child_files.insert(path);
            }
        }

        Ok(())
    }

    /// Get all files in the structure
    pub fn get_all_files(&self) -> Vec<&FileInfo> {
        self.files.values().collect()
    }

    /// Get all directories in the structure
    pub fn get_all_directories(&self) -> Vec<&DirectoryInfo> {
        self.directories.values().collect()
    }

    /// Get files by language
    pub fn get_files_by_language(&self, language: &str) -> Vec<&FileInfo> {
        self.files
            .values()
            .filter(|file| file.language.as_deref() == Some(language))
            .collect()
    }

    /// Get files in a directory
    pub fn get_files_in_directory(&self, path: &Path) -> Vec<&FileInfo> {
        if let Some(dir_info) = self.directories.get(path) {
            dir_info
                .child_files
                .iter()
                .filter_map(|file_path| self.files.get(file_path))
                .collect()
        } else {
            Vec::new()
        }
    }
}
