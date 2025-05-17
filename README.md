# ZSEI: Zero-Shot Embedding Indexer

A revolutionary AI guidance framework that leverages Zero-Shot Bolted Embedding to provide structured pathways for AI reasoning and response generation across code, text, images, audio, and video content.

## Core Concept

ZSEI functions as an advanced knowledge management system for AI models (both LLMs and SLMs), acting as organized "cabinets and file drawers" that represent stored knowledge and guiding processes. It prevents cognitive overload in AI systems by providing structured pathways for reasoning and response generation, enabling AI models to handle complex, multi-stage tasks with consistency and precision.

Rather than directly performing analysis or modifications, ZSEI analyzes user prompts, creates comprehensive execution checklists, identifies needed guidelines, and provides a structured approach that guides AI systems through complex workflows until completion.

## Key Components

### Foundation Layer

- **Zero-Shot Bolted Embedding**: Creates semantic representations without requiring extensive training data
- **Vector Storage & Indexing**: Enables efficient storage and retrieval of knowledge components
- **Metadata Management**: Tracks relationships and dependencies between content elements
- **Project Persistence**: Maintains state across extended operations (24+ hours if needed)
- **Continuous Execution Loop**: Ensures completion of all identified tasks regardless of complexity

### Prompt Analysis System

ZSEI begins by analyzing user prompts to:

1. **Task Identification**: Determines the nature of the requested task(s)
2. **Domain Classification**: Identifies which knowledge domains are relevant (code, text, etc.)
3. **Guideline Selection**: Activates appropriate guidance frameworks for the task
4. **Execution Planning**: Develops comprehensive checklists for completion
5. **Resource Allocation**: Determines computational resources needed for completion

### Content Domain Frameworks

ZSEI provides specialized frameworks for different content types:

#### Code Intelligence
- **Code Update Framework**: Five-pass system for modification of existing code
- **Code Creation Framework**: Structured approach for generating new code
- **Code Analysis Framework**: Systematic extraction of code insights and documentation

#### Text Intelligence
- **Documentation Framework**: Comprehensive document creation and management
- **Creative Writing Framework**: Structured approach to narrative creation
- **Analytical Writing Framework**: Research and analysis documentation
- **Legal Document Framework**: Contract and legal analysis systems
- **Technical Documentation Framework**: Specialized technical writing guidance

#### Future Domains (In Development)
- **Image Intelligence Framework**: Visual content analysis and generation
- **Audio Intelligence Framework**: Sound and speech processing guidance
- **Video Intelligence Framework**: Moving image analysis and production
- **EEG Analysis Framework**: Neural signal interpretation (planned)

## How ZSEI Works

1. **User Provides Prompt**: The user submits a request for AI assistance
2. **ZSEI Analyzes Prompt**: The system identifies the task type and requirements
3. **Guidance Framework Activation**: Appropriate guidelines and checklists are activated
4. **Execution Planning**: A comprehensive execution plan is created
5. **AI Guidance**: The AI model is guided through the execution process step by step
6. **Continuous Loop**: The system maintains execution until all checklist items are complete
7. **Result Delivery**: Final output is delivered with supporting documentation

## Core Technical Features

- **Vectorized Knowledge**: All guidelines and checklists are stored as embeddings for efficient retrieval
- **Streaming Processing**: Handles arbitrarily large content through adaptive chunking
- **Incremental Persistence**: Maintains state between sessions for long-running tasks
- **Cross-Domain Integration**: Allows tasks that span multiple content domains
- **Extensible Architecture**: Easily expanded to accommodate new guidance frameworks

## Use Cases

- **Software Development**: Guide AI through complex code updates, generation, or analysis
- **Documentation Management**: Create, update, or analyze comprehensive documentation
- **Legal Document Processing**: Generate, review, or modify legal documents with precision
- **Research Analysis**: Structure and execute complex research workflows
- **Content Creation**: Provide structured guidance for creating various content types
- **Technical Analysis**: Apply specialized analytical frameworks to complex problems

## Installation

### Prerequisites

- Rust 1.73.0 or higher
- Cargo
- ONNX Runtime (for local LLM inference)
- Optional: Phi-4 Mini (or compatible) for local inference

### Install ZSEI

```bash
# Clone the repository
git clone https://github.com/yourusername/zsei.git
cd zsei

# Build the project
cargo build --release

# Install the binary
cargo install --path .
```

## Quick Start

```bash
# Initialize ZSEI with your preferences
zsei init

# Process a request with ZSEI guidance
zsei process "Create a comprehensive analysis of the authentication system in my codebase"

# For extended operations, use checkpoint mode
zsei process --checkpoint "Refactor my entire codebase to implement the repository pattern"

# Access specialized frameworks directly
zsei code-update "Fix the error where LinuxProcessMonitor cannot be found"
zsei document-create "Create technical documentation for my API"
```

## Configuration

Configure ZSEI by editing the `.zsei/config.toml` file:

```toml
[core]
model_type = "LLM"  # Can be "LLM" or "SLM"
model_path = "/path/to/model"
checkpoint_dir = ".zsei/checkpoints"
max_runtime_hours = 24  # Maximum runtime before mandatory checkpoint

[embedding]
dimension = 384
chunk_size = 1024
chunk_overlap = 128

[processing]
vector_store_type = "Hnsw"
store_metadata = true
memory_limit = "8GB"
parallel_processes = 4

[frameworks]
code_enabled = true
text_enabled = true
image_enabled = false
audio_enabled = false
video_enabled = false
```

## Architecture

ZSEI is built with a modular architecture:

- **Core**: Manages configuration, project state, and execution flow
- **Frameworks**: Domain-specific guidance systems (code, text, etc.)
- **Embedding**: Generates and manages zero-shot embeddings
- **Indexing**: Handles vector storage and retrieval
- **Execution**: Manages the continuous execution loop
- **Persistence**: Handles checkpointing and state management
- **Interface**: CLI and API interfaces

## Framework Details

ZSEI includes multiple specialized frameworks, each with its own methodology and approach. See the dedicated documentation files for detailed information:

- [Code Intelligence Framework](docs/frameworks/code-framework.md)
- [Text Intelligence Framework](docs/frameworks/text-framework.md)
- [Code Update Methodology](docs/methodologies/code-update-methodology.md)
- [Text Analysis Methodologies](docs/methodologies/text-analysis-methodologies.md)

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Learn More

Visit [ZSEI.xyz](https://zsei.xyz) for more information, documentation, and updates.
