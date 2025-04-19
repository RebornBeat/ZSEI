//! Logging utilities for ZSEI
//!
//! This module provides logging utilities for the Zero-Shot Bolted
//! Embedding Indexer.

use crate::errors::Result;
use tracing_subscriber::{fmt, util::SubscriberInitExt, EnvFilter};

/// Initialize the logger
pub fn init(level: &str) -> Result<()> {
    // Create filter
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("zsei={}", level)));

    // Create subscriber
    let subscriber = fmt::Subscriber::builder()
        .with_env_filter(filter)
        .with_ansi(true) // Enable colors
        .with_target(true) // Show targets (module names)
        .with_level(true) // Show log levels
        .with_line_number(true) // Show line numbers
        .finish();

    // Set as global default
    subscriber
        .try_init()
        .map_err(|e| crate::ZseiError::Other(format!("Failed to initialize logger: {}", e)))?;

    Ok(())
}
