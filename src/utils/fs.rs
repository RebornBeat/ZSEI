//! Filesystem utilities for ZSEI
//!
//! This module provides filesystem utilities for the Zero-Shot Bolted
//! Embedding Indexer.

use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::errors::{Result, ZseiError};

/// Read a file as text
pub fn read_file_text(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|e| ZseiError::Io(e))
}

/// Read a file as bytes
pub fn read_file_bytes(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).map_err(|e| ZseiError::Io(e))
}

/// Write text to a file
pub fn write_file_text(path: &Path, content: &str) -> Result<()> {
    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| ZseiError::Io(e))?;
    }

    fs::write(path, content).map_err(|e| ZseiError::Io(e))
}

/// Write bytes to a file
pub fn write_file_bytes(path: &Path, content: &[u8]) -> Result<()> {
    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| ZseiError::Io(e))?;
    }

    fs::write(path, content).map_err(|e| ZseiError::Io(e))
}

/// Find all files in a directory matching a pattern
pub fn find_files(dir: &Path, pattern: &str) -> Result<Vec<PathBuf>> {
    let glob_pattern = format!("{}/{}", dir.display(), pattern);
    let paths = glob::glob(&glob_pattern)
        .map_err(|e| ZseiError::Other(format!("Invalid glob pattern: {}", e)))?
        .filter_map(|entry| entry.ok())
        .collect();

    Ok(paths)
}

/// Find all files in a directory with a specific extension
pub fn find_files_with_extension(dir: &Path, extension: &str) -> Result<Vec<PathBuf>> {
    let files = WalkDir::new(dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_type().is_file()
                && entry
                    .path()
                    .extension()
                    .map_or(false, |ext| ext == extension)
        })
        .map(|entry| entry.path().to_path_buf())
        .collect();

    Ok(files)
}

/// Find all files in a directory with a specific extension (recursive)
pub fn find_files_with_extension_recursive(dir: &Path, extension: &str) -> Result<Vec<PathBuf>> {
    let files = WalkDir::new(dir)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_type().is_file()
                && entry
                    .path()
                    .extension()
                    .map_or(false, |ext| ext == extension)
        })
        .map(|entry| entry.path().to_path_buf())
        .collect();

    Ok(files)
}

/// Find all directories in a directory matching a pattern
pub fn find_directories(dir: &Path, pattern: &str) -> Result<Vec<PathBuf>> {
    let glob_pattern = format!("{}/{}", dir.display(), pattern);
    let paths = glob::glob(&glob_pattern)
        .map_err(|e| ZseiError::Other(format!("Invalid glob pattern: {}", e)))?
        .filter_map(|entry| entry.ok())
        .filter(|path| path.is_dir())
        .collect();

    Ok(paths)
}

/// Get relative path from base directory
pub fn relative_path(path: &Path, base_dir: &Path) -> Result<PathBuf> {
    let rel_path = path
        .strip_prefix(base_dir)
        .map_err(|e| ZseiError::Other(format!("Failed to get relative path: {}", e)))?
        .to_path_buf();

    Ok(rel_path)
}

/// Get file size in bytes
pub fn file_size(path: &Path) -> Result<u64> {
    let metadata = fs::metadata(path).map_err(|e| ZseiError::Io(e))?;

    Ok(metadata.len())
}

/// Check if a file is a text file
pub fn is_text_file(path: &Path) -> Result<bool> {
    // Read first 512 bytes
    let mut buffer = vec![0; 512];

    let n = {
        let mut file = fs::File::open(path).map_err(|e| ZseiError::Io(e))?;

        use std::io::Read;
        file.read(&mut buffer).map_err(|e| ZseiError::Io(e))?
    };

    // Resize buffer to actual read size
    buffer.truncate(n);

    // Check for null bytes (common in binary files)
    let is_text = !buffer.contains(&0);

    Ok(is_text)
}

/// Calculate MD5 hash of a file
pub fn file_md5(path: &Path) -> Result<String> {
    let data = fs::read(path).map_err(|e| ZseiError::Io(e))?;

    let digest = md5::compute(&data);
    let hash = format!("{:x}", digest);

    Ok(hash)
}

/// Get file extension
pub fn file_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(String::from)
}

/// Get file name
pub fn file_name(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(String::from)
}
