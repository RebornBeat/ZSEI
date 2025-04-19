//! Branch management for refactoring
//!
//! This module provides functionality for managing refactoring branches.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

use crate::errors::{Result, ZseiError};
use crate::refactor::{CodeChange, RefactoringBranch};

/// Branch manager
pub struct BranchManager {
    /// Root directory for storing branches
    root_dir: PathBuf,

    /// Active branches
    branches: HashMap<String, RefactoringBranch>,
}

/// Branch comparison result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchComparison {
    /// First branch
    pub branch_a: String,

    /// Second branch
    pub branch_b: String,

    /// Common changes
    pub common_changes: Vec<CodeChange>,

    /// Changes unique to branch A
    pub unique_to_a: Vec<CodeChange>,

    /// Changes unique to branch B
    pub unique_to_b: Vec<CodeChange>,

    /// Conflicting changes
    pub conflicts: Vec<ChangeConflict>,
}

/// Change conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeConflict {
    /// File path
    pub path: PathBuf,

    /// Change from branch A
    pub change_a: CodeChange,

    /// Change from branch B
    pub change_b: CodeChange,
}

impl BranchManager {
    /// Create a new branch manager
    pub fn new(root_dir: PathBuf) -> Result<Self> {
        // Create branch directory if it doesn't exist
        fs::create_dir_all(&root_dir).map_err(|e| {
            ZseiError::Refactor(format!("Failed to create branch directory: {}", e))
        })?;

        Ok(Self {
            root_dir,
            branches: HashMap::new(),
        })
    }

    /// Save a branch
    pub fn save_branch(&mut self, branch: RefactoringBranch) -> Result<()> {
        // Create branch directory
        let branch_dir = self.root_dir.join(&branch.name);
        fs::create_dir_all(&branch_dir).map_err(|e| {
            ZseiError::Refactor(format!("Failed to create branch directory: {}", e))
        })?;

        // Save branch metadata
        let metadata_path = branch_dir.join("metadata.json");
        let metadata = serde_json::to_string_pretty(&branch).map_err(|e| {
            ZseiError::Refactor(format!("Failed to serialize branch metadata: {}", e))
        })?;

        fs::write(metadata_path, metadata)
            .map_err(|e| ZseiError::Refactor(format!("Failed to write branch metadata: {}", e)))?;

        // Save changes
        for (i, change) in branch.changes.iter().enumerate() {
            let change_dir = branch_dir.join(format!("change-{}", i));
            fs::create_dir_all(&change_dir).map_err(|e| {
                ZseiError::Refactor(format!("Failed to create change directory: {}", e))
            })?;

            // Save original content
            let original_path = change_dir.join("original");
            fs::write(&original_path, &change.original).map_err(|e| {
                ZseiError::Refactor(format!("Failed to write original content: {}", e))
            })?;

            // Save modified content
            let modified_path = change_dir.join("modified");
            fs::write(&modified_path, &change.modified).map_err(|e| {
                ZseiError::Refactor(format!("Failed to write modified content: {}", e))
            })?;

            // Save change metadata
            let change_metadata = serde_json::to_string_pretty(&change).map_err(|e| {
                ZseiError::Refactor(format!("Failed to serialize change metadata: {}", e))
            })?;

            let change_metadata_path = change_dir.join("metadata.json");
            fs::write(change_metadata_path, change_metadata).map_err(|e| {
                ZseiError::Refactor(format!("Failed to write change metadata: {}", e))
            })?;
        }

        // Update in-memory cache
        self.branches.insert(branch.name.clone(), branch);

        Ok(())
    }

    /// Load a branch
    pub fn load_branch(&mut self, name: &str) -> Result<RefactoringBranch> {
        // Check if branch is already loaded
        if let Some(branch) = self.branches.get(name) {
            return Ok(branch.clone());
        }

        // Check if branch exists
        let branch_dir = self.root_dir.join(name);
        if !branch_dir.exists() {
            return Err(ZseiError::Refactor(format!("Branch not found: {}", name)));
        }

        // Load branch metadata
        let metadata_path = branch_dir.join("metadata.json");
        let metadata = fs::read_to_string(metadata_path)
            .map_err(|e| ZseiError::Refactor(format!("Failed to read branch metadata: {}", e)))?;

        let mut branch: RefactoringBranch = serde_json::from_str(&metadata).map_err(|e| {
            ZseiError::Refactor(format!("Failed to deserialize branch metadata: {}", e))
        })?;

        // Load changes if not already loaded
        if branch.changes.is_empty() {
            let mut changes = Vec::new();
            let mut i = 0;

            loop {
                let change_dir = branch_dir.join(format!("change-{}", i));
                if !change_dir.exists() {
                    break;
                }

                // Load change metadata
                let change_metadata_path = change_dir.join("metadata.json");
                let change_metadata = fs::read_to_string(change_metadata_path).map_err(|e| {
                    ZseiError::Refactor(format!("Failed to read change metadata: {}", e))
                })?;

                let change: CodeChange = serde_json::from_str(&change_metadata).map_err(|e| {
                    ZseiError::Refactor(format!("Failed to deserialize change metadata: {}", e))
                })?;

                changes.push(change);
                i += 1;
            }

            branch.changes = changes;
        }

        // Update in-memory cache
        self.branches.insert(name.to_string(), branch.clone());

        Ok(branch)
    }

    /// Delete a branch
    pub fn delete_branch(&mut self, name: &str) -> Result<()> {
        // Remove from in-memory cache
        self.branches.remove(name);

        // Delete branch directory
        let branch_dir = self.root_dir.join(name);
        if branch_dir.exists() {
            fs::remove_dir_all(branch_dir).map_err(|e| {
                ZseiError::Refactor(format!("Failed to delete branch directory: {}", e))
            })?;
        }

        Ok(())
    }

    /// Get all branch names
    pub fn get_branch_names(&self) -> Result<Vec<String>> {
        let mut names = Vec::new();

        // Scan branch directory
        for entry in fs::read_dir(&self.root_dir)
            .map_err(|e| ZseiError::Refactor(format!("Failed to read branch directory: {}", e)))?
        {
            let entry = entry.map_err(|e| {
                ZseiError::Refactor(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name() {
                    if let Some(name_str) = name.to_str() {
                        names.push(name_str.to_string());
                    }
                }
            }
        }

        Ok(names)
    }

    /// Compare two branches
    pub fn compare_branches(&mut self, branch_a: &str, branch_b: &str) -> Result<BranchComparison> {
        // Load branches
        let branch_a_data = self.load_branch(branch_a)?;
        let branch_b_data = self.load_branch(branch_b)?;

        // Create change map for branch A
        let mut a_changes: HashMap<PathBuf, &CodeChange> = HashMap::new();
        for change in &branch_a_data.changes {
            a_changes.insert(change.path.clone(), change);
        }

        // Create change map for branch B
        let mut b_changes: HashMap<PathBuf, &CodeChange> = HashMap::new();
        for change in &branch_b_data.changes {
            b_changes.insert(change.path.clone(), change);
        }

        // Find common, unique, and conflicting changes
        let mut common_changes = Vec::new();
        let mut unique_to_a = Vec::new();
        let mut unique_to_b = Vec::new();
        let mut conflicts = Vec::new();

        // Check changes in branch A
        for change_a in &branch_a_data.changes {
            if let Some(change_b) = b_changes.get(&change_a.path) {
                // Both branches modify the same file
                if change_a.modified == change_b.modified {
                    // Identical changes
                    common_changes.push(change_a.clone());
                } else {
                    // Conflicting changes
                    conflicts.push(ChangeConflict {
                        path: change_a.path.clone(),
                        change_a: change_a.clone(),
                        change_b: (*change_b).clone(),
                    });
                }
            } else {
                // Only in branch A
                unique_to_a.push(change_a.clone());
            }
        }

        // Check for changes only in branch B
        for change_b in &branch_b_data.changes {
            if !a_changes.contains_key(&change_b.path) {
                // Only in branch B
                unique_to_b.push(change_b.clone());
            }
        }

        Ok(BranchComparison {
            branch_a: branch_a.to_string(),
            branch_b: branch_b.to_string(),
            common_changes,
            unique_to_a,
            unique_to_b,
            conflicts,
        })
    }

    /// Merge two branches
    pub fn merge_branches(
        &mut self,
        branch_a: &str,
        branch_b: &str,
        new_branch_name: &str,
    ) -> Result<RefactoringBranch> {
        // Compare branches
        let comparison = self.compare_branches(branch_a, branch_b)?;

        // Load branch A for metadata
        let mut branch_a_data = self.load_branch(branch_a)?;

        // Create new changes list
        let mut changes = Vec::new();

        // Add common changes
        changes.extend(comparison.common_changes);

        // Add unique changes from both branches
        changes.extend(comparison.unique_to_a);
        changes.extend(comparison.unique_to_b);

        // For conflicts, we need manual resolution
        // For now, we'll use changes from branch A
        for conflict in comparison.conflicts {
            changes.push(conflict.change_a);
        }

        // Create new branch
        let mut new_branch = branch_a_data.clone();
        new_branch.name = new_branch_name.to_string();
        new_branch.description = format!("Merged from {} and {}", branch_a, branch_b);
        new_branch.changes = changes;
        new_branch.created_at = chrono::Utc::now();

        Ok(new_branch)
    }
}
