# ZSEI: Zero-Shot Embedding Indexer

A revolutionary multi-modal analysis and search framework that combines precise structural analysis with Zero-Shot Bolted Embedding for comprehensive understanding across code, images, audio, and video content.

## Overview

ZSEI is a powerful tool for analyzing, indexing, and optimizing code that leverages Small Language Models (SLMs) and/or Large Language Models (LLMs) for enhanced understanding. Using a "zero-shot bolted embedding" approach, it can provide deep insights into your codebase without requiring extensive training data.

The current version focuses primarily on Rust code analysis, with plans to expand to other programming languages and modalities (image, audio, video) in future releases.

## Key Features

### Initialization Phase (Project Codebase Analysis)
- Thoroughly analyzes and indexes the target project codebase
- Recursively processes all code files in the specified project directory
- Extracts comprehensive metadata about structure, components, and relationships
- Maps out the full functionality to ensure nothing is missed
- Prioritizes completeness and accuracy

### Phase 1 (Prompt Analysis)
- Accepts natural language queries describing coding tasks, issues, or optimizations
- Identifies all code relevant to addressing the prompt
- Loads complete context around relevant code components
- Performs a forward pass to collect the minimal but sufficient code subset
- Runs code build commands to obtain and parse error output
- Updates the prompt based on error output

### Phase 2 (Code Refactoring)
- Creates multiple branch versions with different refactoring approaches
- Analyzes each branch for code quality, efficiency, and functionality preservation
- Provides clear explanations of changes and their benefits
- Allows for selection of the best branch or combination of changes
- Preserves original metadata while creating updated versions
- Supports interactive review of changes

### Continuous Loop
- Runs Phases 1 and 2 iteratively, producing incremental optimizations
- Automatically incorporates build feedback in each iteration
- Maintains full history of code evolution
- Allows exporting at any point

## Installation

### Prerequisites

- Rust 1.73.0 or higher
- Cargo
- Optional: Phi-4 Mini (or compatible) for local inference

### From Source

```bash
# Clone the repository
git clone https://github.com/yourusername/zsei.git
cd zsei

# Build the project
cargo build --release

# Install the binary
cargo install --path .
