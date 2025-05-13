# Guidelines for Creating Technical Analysis Documentation from Codebase Reviews

## Introduction

When analyzing a codebase or processing update requests, you need to create specific technical documentation that breaks down the structure and relationships within the code. This comprehensive guide will walk you through the process of creating four essential types of breakdown documentation, each serving a distinct analytical purpose.

## Document Selection Framework

Before beginning your analysis, determine which document to create based on your objectives:

### BREAKDOWN.md
**Use when**: You need a comprehensive overview of the entire codebase structure
**Focus**: Files, modules, functions, and their purposes
**Best for**: New developers joining a project, initial codebase understanding, or documenting a complete system

### BREAKDOWN_RELATIONSHIPS.md
**Use when**: You need to understand how different parts of the codebase interact
**Focus**: Dependencies between modules, groups, and files
**Best for**: Architecture reviews, refactoring planning, or identifying tight coupling

### BREAKDOWN_FUNCTION_DEPENDENCIES.md
**Use when**: You need detailed function-level dependency analysis
**Focus**: Which functions call other functions, both within and across modules
**Best for**: Debugging complex issues, planning function modifications, or understanding execution flows

### CROSS-MODULE_FUNCTION_DEPENDENCY_ANALYSIS.md
**Use when**: You need to understand cross-boundary interactions
**Focus**: Functions that cross module boundaries
**Best for**: API design reviews, module interface optimization, or identifying hidden coupling

## Integrated Analysis Process

The four breakdown documents work together to provide a complete understanding of the codebase at different levels of abstraction. Each document builds upon insights from the others to create a comprehensive view.

### Phase 1: Initial Codebase Survey

Begin by conducting a broad survey of the codebase to understand its scope and structure:

1. **Identify the module structure**
   - Locate the main directories and packages
   - Determine how code is organized into logical units
   - Identify naming conventions and organizational patterns
   - Document the architectural layers and boundaries

2. **Map the file hierarchy**
   - Document the directory structure and file organization
   - Note patterns in file naming and placement
   - Identify special files (configuration, entry points, etc.)
   - Create a hierarchical representation of the codebase

3. **Locate entry points**
   - Find main functions or application initialization code
   - Identify API endpoints or service definitions
   - Locate primary user interface components
   - Document the system boundaries and external interfaces

4. **Identify core components**
   - Find central data structures and domain models
   - Locate key algorithms and business logic
   - Identify critical subsystems and their responsibilities
   - Document the primary abstractions used in the system

### Phase 2: Dependency Mapping

Next, trace the relationships between different parts of the code:

1. **Import/use statements**
   - Analyze what each file imports or requires from other files
   - Identify patterns in import usage
   - Map module-to-module dependencies based on imports
   - Detect potential circular dependencies

2. **Function calls**
   - Track which functions call other functions
   - Identify high-impact functions called from many places
   - Map function call chains across module boundaries
   - Document critical execution paths

3. **Data flow**
   - Follow how data moves through the system
   - Identify data transformations between components
   - Document data ownership and responsibility boundaries
   - Map shared data structures and their usage patterns

4. **Event chains**
   - Identify event-driven relationships or callback patterns
   - Map event emissions and subscriptions
   - Document asynchronous communication pathways
   - Trace complex interactions triggered by events

### Phase 3: Function Analysis

Dive deeper into function-level details:

1. **Function signatures**
   - Document parameters, return types, and visibility
   - Note generic parameters and constraints
   - Identify function overloads and variants
   - Document error handling patterns and exceptions

2. **Call graphs**
   - Map out which functions call which other functions
   - Create directed graphs showing call relationships
   - Identify clusters of tightly-coupled functions
   - Document call depth and complexity metrics

3. **Side effects**
   - Note functions that modify global state or external resources
   - Identify hidden dependencies through shared state
   - Document I/O operations and external system calls
   - Map functions with unpredictable or non-deterministic behavior

4. **Critical paths**
   - Identify essential execution flows through the system
   - Document performance-critical or reliability-critical paths
   - Map user journey paths through the code
   - Identify bottlenecks and single points of failure

### Phase 4: Documentation Creation

Based on your analysis and chosen document type, create the appropriate breakdown:

## Creating BREAKDOWN.md

Follow this systematic approach:

### 1. Start with the Introduction
Write a clear explanation of what the codebase does and its primary purpose. Include:
- The system's overall purpose and goals
- Key features and capabilities
- Architectural style and patterns
- Technology stack and major dependencies
- Historical context if relevant

### 2. Document the File Structure
Create a hierarchical representation of all files, organized by their modules or directories.

Example format:
```
src/
├── main.rs
│   Purpose: Application entry point
│   Functions:
│   - main() - Initializes the application and starts the server
│   - setup_logging() - Configures the logging system
│   Dependencies: config.rs, server.rs
│   External crates: tokio, log
│
├── config.rs
│   Purpose: Configuration management
│   Functions:
│   - load_config() - Loads configuration from file
│   - validate_config() - Validates configuration values
│   Dependencies: None
│   External crates: serde, toml
```

### 3. Describe Modules
For each module in the codebase:
- Explain its overall purpose and responsibility
- List all sub-modules, structs, enums, and traits
- Document important interactions with other modules
- Describe the module's boundaries and interfaces
- Note design patterns implemented within the module

### 4. Document Data Structures
For key data structures:
- Describe their purpose and usage
- Document fields and relationships
- Note validation rules and constraints
- Explain lifecycle and ownership patterns
- Document serialization or persistence behaviors

### 5. List Crate Dependencies
Create a comprehensive list of external dependencies, including:
- The crate name and version
- Why it's used in the project
- Which files depend on it
- Any known issues or constraints
- Plans for future upgrades or replacements

### 6. Include Configuration and Setup
Document how to get the project running, including:
- Environment variables needed
- Configuration file formats
- Database setup requirements
- Build instructions
- Development environment setup

## Creating BREAKDOWN_RELATIONSHIPS.md

When documenting relationships between components:

### 1. Module-level Dependencies
Start with the highest level - modules depending on other modules:
- Create a module dependency matrix
- Document the nature of module-to-module dependencies
- Identify architectural layers and layer violations
- Calculate coupling metrics between modules

Example:
```
Module: authentication
Depends on:
  - database (for user storage)
    Files: auth/login.rs uses database/users.rs
  - crypto (for password hashing)
    Files: auth/password.rs uses crypto/hash.rs
```

### 2. Group-level Dependencies
Within modules, identify logical groupings and their dependencies:
- Group files by functionality or subsystem
- Document dependencies between these groups
- Identify cross-module group dependencies
- Analyze cohesion within groups and coupling between groups

### 3. File-level Dependencies
For each file, document:
- Other files it depends on
- The specific functions or types it uses from those files
- Whether the dependency is for data, functionality, or types
- The strength and importance of each dependency

### 4. Dependency Direction and Type
Always clearly indicate:
- The direction of dependency (A → B means A depends on B)
- The type of dependency (function call, data structure, trait implementation)
- The strength of the dependency (how tightly coupled)
- Whether the dependency is optional or required

### 5. Circular Dependencies
Identify and analyze any circular dependencies:
- Document the chain of dependencies that form the circle
- Analyze the nature and cause of the circular dependency
- Assess the impact on the codebase (testing difficulty, initialization issues)
- Suggest approaches to breaking harmful circular dependencies

### 6. Criticality Assessment
Evaluate each dependency's importance:
- Mark critical dependencies that would break functionality if removed
- Identify optional dependencies that could be refactored
- Create a heat map of dependency criticality
- Assess the impact of changes to various components

## Creating BREAKDOWN_FUNCTION_DEPENDENCIES.md

For detailed function-level analysis:

### 1. Module Function Mapping
For each module, create a comprehensive list of:
- All functions and their signatures
- Which functions they call (internal and external)
- Which functions call them
- Function complexity and size metrics

Example:
```
Module: payment_processing
File: payment_processing/transaction.rs

Function: process_payment(amount: f64, user_id: u32) -> Result<Transaction>
Calls:
  - validate_amount(amount) [internal]
  - user_service::get_user(user_id) [external - user module]
  - payment_gateway::charge_card(...) [external - gateway module]
Called by:
  - api::handle_payment_request() [external - api module]
```

### 2. Function Call Graphs
Create visual representations of function call relationships:
- Overall call graph for the system
- Focused call graphs for important subsystems
- Highlight critical or frequently-called functions
- Mark recursive or cyclic call patterns

### 3. Function Signatures
Provide complete function signatures including:
- Function name and visibility
- Parameters with types
- Return type
- Generic parameters and trait bounds
- Exception or error handling patterns

### 4. Cross-Module Functions
Identify and document functions that cross module boundaries:
- Functions exported by modules for external use
- Functions that import and use external module functions
- The nature and purpose of the cross-module calls
- How data transforms when crossing module boundaries

### 5. Dependency Direction
Use clear notation to show call direction:
- Create call hierarchy diagrams
- Document execution sequences
- Show how data flows through function calls
- Identify call chains and their depth

Example:
```
process_order() → validate_items() → check_inventory()
                → calculate_total() → apply_discounts()
```

### 6. Function Impact Analysis
Assess the impact and importance of functions:
- High-impact functions called from many places
- Functions on critical execution paths
- Functions with access to critical resources
- Functions with high maintenance or change frequency

## Creating CROSS-MODULE_FUNCTION_DEPENDENCY_ANALYSIS.md

For cross-module analysis:

### 1. Identify Cross-Module Functions
Find all functions that are called across module boundaries:
- Public functions exposed by modules
- Functions imported and used by other modules
- Functions that transform data between module boundaries
- Event handlers that respond to cross-module events

### 2. Module Interface Analysis
For each module, document:
- The functions it exposes to other modules
- Which modules call these exported functions
- The stability and change frequency of the interface
- The impact radius of interface changes

### 3. Document Usage Patterns
For each cross-module function:
- List all modules that use it
- Document the specific files and functions that call it
- Note the frequency and context of usage
- Identify critical dependencies on the function

Example:
```
Function: authentication::verify_token(token: &str) -> Result<User>
Used in:
  - api::middleware::auth_middleware() - Called for every API request
  - websocket::handle_connection() - Called on new WebSocket connections
  - scheduler::run_scheduled_task() - Called for authenticated background tasks
```

### 4. Boundary Analysis
Analyze how data and control flow across module boundaries:
- Document data transformations at boundaries
- Identify parameter and return value dependencies
- Note error handling and propagation patterns
- Assess impact of interface changes on consumers

### 5. Architectural Assessment
Evaluate architectural quality based on cross-module dependencies:
- Check for layering violations
- Identify inappropriate dependencies
- Assess modularity and encapsulation
- Suggest improvements to module interfaces

## Best Practices for All Documentation Types

### Accuracy and Completeness
- Verify all documented relationships by checking the actual code
- Use automated tools when possible to ensure completeness
- Double-check function signatures and import statements
- Validate findings with code authors when possible
- Document assumptions and limitations of the analysis

### Clarity and Consistency
- Use consistent formatting throughout all documents
- Maintain a clear hierarchy in your documentation structure
- Use the same terminology across all sections
- Include both details and high-level summaries
- Use visual aids when they enhance understanding

### Maintainability
- Include timestamps or version information
- Document your analysis methodology
- Provide clear instructions for updating the documentation
- Consider automating parts of the documentation generation
- Create a process for keeping documentation current

### Visual Aids
When helpful, include:
- Dependency graphs or diagrams
- Call flow charts
- Module relationship diagrams
- Heat maps of dependency intensity
- Execution flow diagrams

### Focus on Actionable Insights
- Highlight areas of concern or improvement opportunities
- Provide recommendations based on findings
- Prioritize insights by impact and feasibility
- Connect findings to architectural goals and principles
- Include both problems and positive aspects of the codebase

## Common Challenges and Solutions

### Large Codebases
For very large codebases:
- Start with high-level module analysis
- Focus on critical paths first
- Use automated tools to generate initial documentation
- Break the work into manageable phases
- Sample representative parts of the codebase if full analysis isn't feasible

### Complex Dependencies
When dealing with complex dependency chains:
- Create visual diagrams to supplement text
- Group related dependencies together
- Focus on the most important relationships first
- Create multiple views (high-level and detailed)
- Use dependency types to organize and clarify relationships

### Dynamic Dependencies
For runtime or configuration-based dependencies:
- Document the conditions under which dependencies exist
- Note configuration options that affect dependencies
- Include examples of different runtime scenarios
- Supplement static analysis with runtime analysis when possible
- Document dependency injection patterns and configuration

### Legacy Code
When documenting legacy systems:
- Start with the current state, not the intended design
- Note areas that need refactoring
- Document workarounds and technical debt
- Prioritize understanding critical paths
- Focus on areas that are frequently modified

## Integration with Implementation Planning

The analysis documentation should feed directly into implementation planning:

1. **From Analysis to Planning**
   - Use BREAKDOWN.md to understand the overall system architecture
   - Use BREAKDOWN_RELATIONSHIPS.md to identify modules affected by changes
   - Use BREAKDOWN_FUNCTION_DEPENDENCIES.md to determine function impact
   - Use CROSS-MODULE_ANALYSIS.md to plan interface changes carefully

2. **Create Implementation Plan**
   - Base implementation plans on the knowledge from breakdown documents
   - Use dependency analysis to sequence work appropriately
   - Identify areas requiring special care due to high connectivity
   - Plan testing strategy based on dependency knowledge

3. **Validate Implementation Approach**
   - Use dependency documents to validate implementation strategies
   - Check for unintended consequences across module boundaries
   - Verify that planned changes respect architectural boundaries
   - Confirm that all affected components are considered

## Tools and Automation

Consider these tools to assist in your analysis:

1. **Static Analysis Tools**
   - Code dependency analyzers
   - Call graph generators
   - Coupling and cohesion metrics calculators
   - Architecture visualization tools

2. **Language-Specific Tools**
   - For Rust: cargo-modules, rust-analyzer
   - For JavaScript/TypeScript: ESLint dependency plugins, Madge
   - For Java: JDepend, Structure101
   - For Python: Pyan, Pylint

3. **Custom Scripts**
   - Create scripts to extract import statements
   - Build tools to analyze function calls
   - Develop documentation generators from code analysis
   - Implement dependency graph visualization

4. **Version Control Integration**
   - Track changes to dependencies over time
   - Monitor interface stability through history
   - Correlate dependency changes with issues
   - Analyze impact of past refactorings

## Conclusion

Creating effective technical analysis documentation requires a systematic approach and attention to detail. By following these guidelines, you'll be able to produce documentation that helps developers understand, maintain, and evolve the codebase effectively.

The key to success is choosing the right type of documentation for your specific needs and following the structured approach outlined for each document type. Regular updates and maintenance of these documents ensure they remain valuable resources throughout the project's lifecycle.

Remember that the goal is not just to document what exists, but to provide insights that enable better decision-making about the code's structure and future development.
