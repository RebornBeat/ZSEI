# BREAKDOWN.md Guideline for Codebases

## Purpose
The purpose of the BREAKDOWN.md file is to provide a comprehensive, detailed overview of a codebase's structure, components, and functionalities. It serves as the foundation document for understanding the system architecture and provides a map for navigating the codebase for developers, reviewers, and other stakeholders.

## File Location
The BREAKDOWN.md file should be located in the root directory of the codebase to ensure maximum visibility.

## Creation Timing
Create BREAKDOWN.md:
- At the beginning of a codebase analysis project
- When preparing for substantial refactoring or enhancement
- When onboarding new team members who need a comprehensive reference
- As the first step in the Implementation Planning workflow

## Sections

### 1. Introduction
- Briefly explain the purpose and functionality of the codebase
- Provide necessary background information or context
- Include system architecture diagrams where applicable
- Mention the technology stack and key dependencies
- Overview of design principles and architectural patterns used

### 2. File Structure
- List all files in the codebase, grouped by their respective modules or directories
- For each file, include:
  - File name and path
  - Brief description of its purpose
  - List of all functions defined in the file with their signatures
  - Brief explanation of each function's functionality and responsibilities
  - Inter-dependencies with other files (imports and exports)
  - Crates/libraries imported (both internal and external)
  - Functions called from each imported crate and their usage in the file
  - Entry points and initialization paths

Example format:
```
src/
├── main.rs
│   Purpose: Application entry point
│   Functions:
│   - main() -> Result<()> - Initializes the application and starts the server
│   - setup_logging(level: LogLevel) -> Result<()> - Configures logging system
│   Dependencies: config.rs, server.rs
│   External crates: tokio, log, env_logger
│   Entry point: main()
│
├── config.rs
│   Purpose: Configuration management
│   Functions:
│   - load_config(path: PathBuf) -> Result<Config> - Loads configuration from file
│   - validate_config(config: &Config) -> Result<()> - Validates configuration values
│   Dependencies: None
│   External crates: serde, toml, thiserror
```

### 3. Modules
- List and describe all the modules in the codebase
- For each module, include:
  - Module name and path
  - Brief description of its purpose and functionality
  - List of all sub-modules, structs, enums, and traits defined in the module
  - Important dependencies or interactions with other modules
  - Responsibilities and boundaries
  - Design patterns implemented within the module

Example format:
```
## Module: authentication
Purpose: Handles user authentication and authorization
Sub-modules:
- providers/ - Authentication provider implementations
- middleware/ - Authentication middleware
- utils/ - Helper utilities for authentication

Key structures:
- User - Represents authenticated user data
- Credentials - Handles credential validation
- Session - Manages user sessions

Dependencies:
- database module (for user retrieval)
- config module (for auth settings)
```

### 4. Data Structures
- Document key data structures (structs, classes, etc.)
- For each data structure, include:
  - Name and location
  - Fields and their types
  - Purpose and usage context
  - Relationships with other data structures
  - Invariants and constraints
  - Serialization/deserialization behaviors

Example format:
```
## Structure: Transaction (core/transaction.rs)
Purpose: Represents a blockchain transaction
Fields:
- id: Uuid - Unique identifier
- sender: Address - Transaction sender
- receiver: Address - Transaction recipient
- amount: u64 - Transaction amount
- signature: Option<Signature> - Transaction signature
- timestamp: u64 - Transaction creation time

Relationships:
- Used by: Block, TransactionPool, Wallet
- Contains: Address (sender, receiver)
```

### 5. Core Algorithms
- Identify and document critical algorithms used in the system
- For each algorithm, include:
  - Purpose and functionality
  - Location in the codebase
  - Time and space complexity
  - Key considerations or constraints
  - Edge cases and limitations

### 6. Crate Dependencies
- List all the external crates/libraries used in the codebase
- For each crate, include:
  - Crate name and version
  - Brief description of its purpose and usage in the codebase
  - Files and functions that depend on the crate
  - Import patterns and dependency management
  - Any planned upgrades or replacements

Example format:
```
## Crate: tokio v1.28.0
Purpose: Asynchronous runtime for Rust
Used for: Network communication, concurrent task execution, async I/O
Used in:
- server.rs (TCP server implementation)
- client.rs (HTTP client connections)
- worker.rs (background task processing)
```

### 7. Configuration and Setup
- Explain how to configure and set up the codebase to run locally
- Include any necessary environment variables, configuration files, or database setup steps
- Document deployment prerequisites and dependencies
- Provide instructions for different environments (development, testing, production)

### 8. Testing
- Describe the testing approach and any key testing frameworks or tools used
- Explain how to run the test suite and any important test categories or conventions
- Document test coverage expectations and goals
- Describe test data generation and management
- Include information about CI/CD integration

### 9. Deployment
- Explain the deployment process and any key considerations or requirements
- Include information about the target environment, deployment tools, and CI/CD pipeline
- Document release procedures and versioning strategy
- Include rollback procedures and disaster recovery information

### 10. Contributing
- Provide guidelines for contributing to the codebase
- Explain any coding conventions, commit message formats, or pull request processes
- Document the code review process

### 11. Additional Resources
- Link to any additional documentation, READMEs, or external resources
- Include references to architectural documents, design decisions, and ADRs
- Link to issue trackers, project management tools, and communication channels

### 12. Technical Debt and Known Issues
- Document known technical debt
- List architectural compromises made and their rationales
- Identify areas for improvement and refactoring
- Note known bugs or limitations

## Best Practices

### Clarity and Comprehensiveness
- Keep the BREAKDOWN.md file comprehensive yet concise
- Use clear, descriptive language and avoid jargon
- Include diagrams where text explanations are insufficient
- Provide examples for complex components
- Balance detail with readability

### Structure and Formatting
- Use consistent formatting throughout the document
- Structure the document hierarchically with clear section headings
- Use markdown tables for comparing components or listing attributes
- Use code blocks with syntax highlighting for code examples

### Maintenance
- Keep the file up to date as the codebase evolves
- Add version information or last-updated timestamps
- Document significant changes to the architecture
- Consider using automated tools to generate and update parts of the file
- Schedule regular reviews to ensure accuracy

### Advanced Techniques
- Use hyperlinks within the document to reference related sections
- Include a changelog to track significant updates to the document
- Consider adding a searchable table of contents for large codebases
- Add tags or categorization to help readers find specific information
- Include a glossary for domain-specific terminology

## Integration with Other Documents
The BREAKDOWN.md should be referenced by and connected to:
- Implementation Plan documents
- Function Dependency Analysis documents
- Relationship Analysis documents
- Cross-Module Analysis documents
- Development roadmaps and sprint planning

## Example Template
```markdown
# [Project Name] Breakdown

## Introduction
[Brief description of the project and its purpose]

## System Architecture
[High-level architecture diagram and description]

## File Structure
[Detailed file tree with descriptions]

## Modules
[Module descriptions and relationships]

## Data Structures
[Key data structures and their purposes]

## Core Algorithms
[Critical algorithms and implementations]

## Crate Dependencies
[External dependencies and their usage]

## Configuration and Setup
[Setup instructions and configuration details]

## Testing
[Testing approach and instructions]

## Deployment
[Deployment procedures and environments]

## Contributing
[Guidelines for contribution]

## Additional Resources
[Links to other documentation]

## Technical Debt and Known Issues
[Known limitations and areas for improvement]
``
