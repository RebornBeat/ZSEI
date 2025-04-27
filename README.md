# ZSEI: Zero-Shot Embedding Indexer

A revolutionary multi-modal analysis and search framework that combines precise structural analysis with Zero-Shot Bolted Embedding for comprehensive understanding across code, images, audio, and video content.

## Overview

ZSEI is a powerful tool for analyzing, indexing, and optimizing code that leverages Small Language Models (SLMs) and/or Large Language Models (LLMs) for enhanced understanding. Using a "zero-shot bolted embedding" approach, it provides deep insights into your codebase without requiring extensive training data.

The system works through a multi-phase approach that continuously refines its understanding and optimizations:

1. **Initialization Phase**: Thoroughly analyzes and indexes your project
2. **Prompt Analysis Phase**: Identifies relevant code based on natural language queries 
3. **Refactoring Phase**: Creates multiple optimization approaches with explanations
4. **Continuous Refinement**: Iteratively improves code through feedback loops

The current version focuses primarily on Rust code analysis, with plans to expand to other programming languages and modalities (image, audio, video) in future releases.

## Key Features

### Comprehensive Analysis System

- **Function-Level Dependency Tracking**: Captures relationships between functions across all modules
- **Cross-Module Analysis**: Understands connections and patterns across different parts of your codebase
- **Progressive Multi-Pass Analysis**: Refines understanding with increasing detail levels in each pass
- **Memory-Efficient Processing**: Handles codebases of unlimited size through streaming and adaptive chunking
- **LLM-Driven Semantic Understanding**: Leverages language models for deeper code comprehension

### Initialization Phase (Project Codebase Analysis)

- Thoroughly analyzes and indexes the target project codebase
- Recursively processes all code files in the specified project directory
- Extracts comprehensive metadata about structure, components, and relationships
- Maps out the full functionality to ensure nothing is missed
- Creates a prioritized understanding of your code's organization

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

### Dependency Intelligence

- Automatically analyzes `Cargo.toml` and external dependencies
- Identifies available functions in dependencies through documentation scraping
- Detects version updates and API changes
- Provides migration assistance for dependency updates
- Optimizes dependency usage based on code patterns

### Continuous Loop

- Runs Phases 1 and 2 iteratively, producing incremental optimizations
- Automatically incorporates build feedback in each iteration
- Maintains full history of code evolution
- Provides checkpoints for resuming long-running analyses
- Allows exporting at any point

## Installation

### Prerequisites

- Rust 1.73.0 or higher
- Cargo
- ONNX Runtime (for LLM inference)
- Tree-sitter (for code parsing)
- Optional: Phi-4 Mini (or compatible) for local inference

### Install Rust and Cargo (if not already installed)
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Install ONNX Runtime

```bash
# Create a directory for ONNX Runtime
mkdir -p ~/onnxruntime
cd ~/onnxruntime

# Download the Linux version
wget https://github.com/microsoft/onnxruntime/releases/download/v1.21.0/onnxruntime-linux-x64-1.21.0.tgz

# Extract it
tar xzf onnxruntime-linux-x64-1.21.0.tgz

# Set up environment variables (add these to your ~/.bashrc)
echo 'export LD_LIBRARY_PATH=$LD_LIBRARY_PATH:~/onnxruntime/onnxruntime-linux-x64-1.21.0/lib' >> ~/.bashrc
echo 'export LIBRARY_PATH=$LIBRARY_PATH:~/onnxruntime/onnxruntime-linux-x64-1.21.0/lib' >> ~/.bashrc
echo 'export ONNXRUNTIME_LIB_PATH=~/onnxruntime/onnxruntime-linux-x64-1.21.0/lib' >> ~/.bashrc
source ~/.bashrc
```

### Install Tree-Sitter CLI

```bash
# Install the CLI tool
cargo install tree-sitter-cli

# Add cargo bin to your PATH (if not already done)
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Initialize Tree-Sitter configuration
tree-sitter init-config

# Create directory for Tree-Sitter parsers
mkdir -p ~/.tree-sitter/grammars

# Clone the Rust grammar
git clone https://github.com/tree-sitter/tree-sitter-rust.git ~/.tree-sitter/grammars/tree-sitter-rust

# Configure Tree-Sitter
mkdir -p ~/.config/tree-sitter
cat > ~/.config/tree-sitter/config.json << 'EOF'
{
  "parser-directories": [
    "~/.tree-sitter/grammars"
  ]
}
EOF
```

### Download Phi-4 Mini Model

```bash
# Create a virtual environment
python -m venv hf_env

# Activate the virtual environment
source hf_env/bin/activate

# Install huggingface_hub
pip install huggingface_hub

# Download the model files
mkdir -p ~/models/phi-4-mini
huggingface-cli download microsoft/Phi-4-mini-instruct-onnx --include cpu_and_mobile/cpu-int4-rtn-block-32-acc-level-4/* --local-dir ~/models/phi-4-mini

# Get additional tokenizer files
wget -O ~/models/phi-4-mini/tokenizer.json https://huggingface.co/microsoft/Phi-4-mini-instruct/resolve/main/tokenizer.json
wget -O ~/models/phi-4-mini/special_tokens_map.json https://huggingface.co/microsoft/Phi-4-mini-instruct/resolve/main/special_tokens_map.json

# Deactivate the virtual environment when done
deactivate
```

### From Source

```bash
# Clone the repository
git clone https://github.com/yourusername/zsei.git
cd zsei

# Build the project
cargo build --release

# Install the binary
cargo install --path .
```

## Usage

### Basic Commands

```bash
# Initialize a project
zsei init [--path PATH]

# Analyze a project
zsei analyze --progress

# Index a project for searching
zsei index --progress

# Query the indexed project
zsei query "your query here" [--max-results 10]

# Run a refactoring operation
zsei refactor "fix error in function X" [--branches 5]

# Run the full analysis-refactoring loop
zsei run "optimize error handling" [--iterations 3]

# Check and manage dependencies
zsei dependency check
```

### Dependency Management

```bash
# Check for dependency updates
zsei dependency check

# Generate migration plan for updates
zsei dependency plan

# Find available functions in a dependency
zsei dependency functions serde

# Apply dependency updates
zsei dependency update
```

### Example Workflow

#### Initialize the project

```bash
cd /path/to/your/project
zsei init
```

#### Run a complete analysis-refactoring loop

```bash
zsei run "fix the error where LinuxProcessMonitor cannot be found in linux module"
```

#### Review and select the best branch of changes

```bash
# After ZSEI presents multiple solution branches
# You'll be prompted to select the most appropriate solution
```

#### Let the tool apply those changes and continue iterating

```bash
# ZSEI will apply the selected changes and continue the optimization process
# You can interrupt at any point to review progress
```

## Configuration

The configuration is stored in the `.zsei/config.toml` file in your project directory. You can modify this file to customize the behavior of ZSEI.

### Example configuration

```toml
[llm]
model_type = "PhiMini"
model_path = "/path/to/phi-mini-model"

[embedding]
dimension = 384
chunk_size = 1024
chunk_overlap = 128

[indexing]
vector_store_type = "Hnsw"
store_metadata = true
store_content = true
include_extensions = ["rs", "py", "js", "ts"]
exclude_patterns = ["**/target/**", "**/node_modules/**", "**/.git/**"]

[refactor]
num_branches = 5
keep_iterations = 3
auto_apply = false
max_modified_files = 50

[analysis]
# New configuration section for progressive analysis
passes = 3                 # Number of analysis passes to perform
memory_limit = "8GB"       # Maximum memory to use
parallel_files = 4         # Number of files to process in parallel
checkpoint_frequency = 10  # Create checkpoints after every N files

[dependency]
# New configuration section for dependency management
scrape_docs = true         # Whether to scrape documentation for functions
check_updates = true       # Automatically check for dependency updates
```

## Architecture

ZSEI is built with a modular architecture that separates concerns and allows for easy extension:

- **Analyzers**: Language-specific code analyzers for extracting structural information
- **Embedding**: Zero-Shot Bolted Embedding generation and management
- **Indexing**: Vector storage and retrieval for efficient searching
- **Query**: Natural language query processing and context building
- **Refactoring**: Code optimization and transformation based on LLM suggestions
- **Dependency**: Dependency management and API tracking
- **Core**: Project and configuration management
- **CLI**: Command-line interface for user interaction

### Progressive Analysis

The system uses a multi-pass approach to analyze code:

1. **Initial Pass**: Basic structure and relationships
2. **Semantic Pass**: Deeper understanding of code meaning
3. **Quality Pass**: Assessment of implementation quality and completeness
4. **Optimization Pass**: Identification of improvement opportunities

Each pass builds upon the previous, creating a progressively more detailed understanding of your codebase.

## Future Expansion

While the current version focuses on Rust code analysis, future versions will add:

- Support for more programming languages (Python, JavaScript, TypeScript, Go, etc.)
- Image analysis and embedding
- Audio analysis and embedding
- Video analysis and embedding
- Cross-modal search and understanding
- Enhanced dependency intelligence with automated migration

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

- Fork the repository
- Create your feature branch (git checkout -b feature/amazing-feature)
- Commit your changes (git commit -m 'Add some amazing feature')
- Push to the branch (git push origin feature/amazing-feature)
- Open a Pull Request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Learn More

Visit ZSEI.xyz for more information, documentation, and updates.
