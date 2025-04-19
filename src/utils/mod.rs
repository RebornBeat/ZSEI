//! Utility functions for ZSEI
//!
//! This module provides utility functions for the Zero-Shot Bolted
//! Embedding Indexer.

pub mod fs;
pub mod logger;

use std::future::Future;
use std::time::{Duration, Instant};

/// Time a function call and log the result
pub fn time<F, T>(name: &str, f: F) -> T
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let result = f();
    let elapsed = start.elapsed();

    tracing::debug!("{} took {:.2?}", name, elapsed);

    result
}

/// Time an async function call and log the result
pub async fn time_async<F, Fut, T>(name: &str, f: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = T>,
{
    let start = Instant::now();
    let result = f().await;
    let elapsed = start.elapsed();

    tracing::debug!("{} took {:.2?}", name, elapsed);

    result
}

/// Format a duration as a human-readable string
pub fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();

    if total_secs < 60 {
        format!("{:.2}s", duration.as_secs_f64())
    } else if total_secs < 3600 {
        let minutes = total_secs / 60;
        let seconds = total_secs % 60;
        format!("{}m {}s", minutes, seconds)
    } else {
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        let seconds = total_secs % 60;
        format!("{}h {}m {}s", hours, minutes, seconds)
    }
}

/// Format a file size as a human-readable string
pub fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size < KB {
        format!("{} B", size)
    } else if size < MB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else if size < GB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else {
        format!("{:.2} GB", size as f64 / GB as f64)
    }
}

/// Truncate a string to a maximum length
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let mut result = s[..max_len - 3].to_string();
        result.push_str("...");
        result
    }
}

/// Levenshtein distance between two strings
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    // Edge cases
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }

    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();

    // Create distance matrix
    let mut matrix = vec![vec![0; b_chars.len() + 1]; a_chars.len() + 1];

    // Initialize first row and column
    for i in 0..=a_chars.len() {
        matrix[i][0] = i;
    }
    for j in 0..=b_chars.len() {
        matrix[0][j] = j;
    }

    // Fill matrix
    for i in 1..=a_chars.len() {
        for j in 1..=b_chars.len() {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };

            matrix[i][j] = std::cmp::min(
                matrix[i - 1][j] + 1, // deletion
                std::cmp::min(
                    matrix[i][j - 1] + 1,        // insertion
                    matrix[i - 1][j - 1] + cost, // substitution
                ),
            );
        }
    }

    matrix[a_chars.len()][b_chars.len()]
}

/// Calculate similarity between two strings (0.0 to 1.0)
pub fn string_similarity(a: &str, b: &str) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }

    let distance = levenshtein_distance(a, b) as f64;
    let max_len = std::cmp::max(a.len(), b.len()) as f64;

    1.0 - (distance / max_len)
}
