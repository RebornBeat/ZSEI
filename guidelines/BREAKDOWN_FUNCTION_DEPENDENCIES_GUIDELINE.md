# BREAKDOWN_FUNCTION_DEPENDENCIES.md Guideline

## Purpose
The purpose of BREAKDOWN_FUNCTION_DEPENDENCIES.md is to provide a detailed analysis of the dependencies and relationships between functions across the codebase. This document dives deeper than the module-level relationships to expose the specific function-to-function interactions, enabling precise understanding of execution flows, dependency chains, and the fine-grained impact of code changes.

## Creation Timing
Create BREAKDOWN_FUNCTION_DEPENDENCIES.md:
- After completing BREAKDOWN.md and BREAKDOWN_RELATIONSHIPS.md
- When planning to modify critical or widely-used functions
- When debugging complex interaction issues
- When performance optimization is needed
- As part of the evaluation phase in implementation planning

## Structure

### 1. Executive Summary
- Overview of key function dependency patterns and notable findings
- Summary of function dependency metrics (call depth, fan-in/fan-out)
- Identification of high-impact functions (those with many callers)
- Highlight critical execution paths and bottlenecks
- Identify areas of concern (deeply nested calls, complex recursion)

Example:
```
# Function Dependency Analysis for [Project Name]

## Executive Summary
This analysis mapped 782 function dependencies across the codebase with these key findings:

1. High-impact functions: Three functions are called by >30 other functions
   - `validate_user_input()` (52 callers)
   - `log_event()` (47 callers)
   - `get_configuration()` (34 callers)

2. Critical execution paths:
   - Authentication flow (7 functions, max depth 5)
   - Transaction processing (12 functions, max depth 8)

3. Areas of concern:
   - Deep recursion in tree-traversal functions (depth >10)
   - Callback hell in event handling (8 nested callbacks)
```

### 2. Function Call Graphs
- Provide visual call graphs for important subsystems
- Include both caller and callee relationships
- Highlight critical or high-traffic functions
- Identify function clusters and boundaries
- Mark recursive calls and dependency cycles

Example:
```
## Authentication System Call Graph
```
[Include function call graph visualization]
```
## Transaction Processing Call Graph
```
[Include function call graph visualization]

### 3. Module-level Function Dependencies
- For each module, document the function blocks and their dependencies
- Show which functions call other functions across module boundaries
- Group functions by their role in the module
- Identify entry points and public interfaces

Example format:
```
## Module: authentication

### Function Block: User Authentication
Entry point: `authenticate_user(credentials: Credentials) -> Result<Session>`
Dependencies:
  - Internal:
    - `validate_credentials(credentials: Credentials) -> bool`
    - `create_session(user_id: UserId) -> Session`
  - External:
    - database::users::get_user_by_username(username: String) -> Option<User>
    - crypto::hash::verify_password(plain: String, hashed: String) -> bool

Called by:
  - api::routes::login::handle_login (direct)
  - api::middleware::auth::verify_authentication (indirect)
```

### 4. Function Block Signatures
- Document function signatures with parameter and return types
- Include visibility modifiers
- Note generic type parameters and constraints
- Document trait implementations and method overrides
- Include async/await status or other special characteristics

Example:
```
## Function Signatures

### `authenticate_user`
```rust
pub async fn authenticate_user(credentials: Credentials) -> Result<Session, AuthError>
```

### `validate_credentials`
```rust
fn validate_credentials(credentials: Credentials) -> bool
```

### `create_session`
```rust
pub fn create_session(user_id: UserId, expires_in: Duration) -> Session
```
```

### 5. Cross-Module Function Dependencies
- Focus specifically on functions that cross module boundaries
- Document which functions are exported and imported between modules
- Analyze the stability of these cross-module interfaces
- Identify potential abstraction boundaries

Example:
```
## Cross-Module Function Calls

### From Module: api
To Module: authentication
- `api::routes::login::handle_login` calls `authentication::authenticate_user`
- `api::middleware::auth::verify_request` calls `authentication::validate_session`

To Module: database
- `api::routes::users::get_user` calls `database::users::find_by_id`
- `api::routes::products::list_products` calls `database::products::find_all`
```

### 6. Dependency Direction
- Clearly indicate the direction of dependencies between functions
- Use flow diagrams or text-based representation
- Show call hierarchies and execution sequences
- Identify inverted dependencies (where lower-level functions call higher-level ones)

Example:
```
## Authentication Flow Sequence

1. `api::routes::login::handle_login`
   ↓
2. `authentication::authenticate_user`
   ↓
3. `authentication::validate_credentials`
   ↓
4. `database::users::get_user_by_username`
   ↓
5. `crypto::hash::verify_password`

Inverted dependency: `logger::audit::log_login_attempt` is called by multiple levels
```

### 7. Dependency Type Analysis
- Categorize function dependencies by type:
  - Direct function calls
  - Callback registrations
  - Event subscriptions
  - Decorator patterns
  - Higher-order functions
  - Dependency injection
- Note how these different dependency types affect coupling

Example:
```
## Dependency Types

### Direct Function Calls
- Most common (87% of dependencies)
- Strongest coupling, hardest to change
- Example: `process_order()` directly calls `validate_inventory()`

### Callback Registration
- Used in event handling (8% of dependencies)
- Moderate coupling, can be modified at runtime
- Example: `ui::button::on_click(handle_submit)`

### Event-Based
- Used for system-wide notifications (5% of dependencies)
- Loose coupling, but harder to trace
- Example: `events::emit("order_completed", order_data)`
```

### 8. Recursive and Cyclical Dependencies
- Identify and document recursive function calls
- Map cycles in function call chains
- Analyze termination conditions and potential issues
- Suggest improvements for problematic patterns

Example:
```
## Recursive Dependencies

### Self-Recursion
- `tree::traverse_nodes` calls itself for child nodes
  - Termination: Leaf node reached
  - Maximum observed depth: 12
  - Concerns: Stack overflow for deeply nested structures

### Mutual Recursion
- `parser::parse_expression` ↔ `parser::parse_term`
  - Termination: Token list consumed
  - Concerns: Complex control flow, difficult to debug
```

### 9. Criticality and Impact
- Assess the criticality of each function
- Document impact radius (how many functions would be affected by a change)
- Identify functions on the critical path
- Create risk profiles for high-impact functions

Example:
```
## Function Impact Analysis

### High-Impact Functions
1. `database::connection::get_connection`
   - Called by: 37 other functions
   - Impact radius: 124 functions (52% of codebase)
   - Risk: High - changes affect database access across system
   - Stability requirement: Very high

2. `auth::validate_token`
   - Called by: 28 other functions
   - Impact radius: 42 functions (18% of codebase)
   - Risk: High - affects security boundaries
   - Stability requirement: High
```

### 10. Change Propagation Analysis
- Analyze how changes to function signatures propagate
- Identify breaking vs. non-breaking changes
- Document the ripple effect of different change types
- Create a change impact heat map

Example:
```
## Change Impact Analysis

### Adding Optional Parameter
Function: `get_user(id: UserId, include_details: bool = false)`
Impact: Low - default value means existing calls remain valid
Affected callers: None (backwards compatible)

### Changing Return Type
Function: `authenticate_user()` (Result<Session> to Result<AuthToken>)
Impact: High - breaks all calling code
Affected callers: 12 functions across 3 modules
Mitigation: Create adapter function with old signature
```

### 11. Optimization Opportunities
- Identify redundant function calls
- Detect duplicate functionality across functions
- Highlight opportunities for function consolidation
- Suggest potential performance improvements

Example:
```
## Optimization Opportunities

### Redundant Calls
- `validate_input()` called multiple times with same parameters
  - Called 3x in `process_form()`
  - Recommendation: Cache validation result

### Duplicate Functionality
- Email validation logic duplicated in 4 functions:
  - `user::validate_email()`
  - `newsletter::validate_subscriber()`
  - `contact::validate_form()`
  - `marketing::validate_recipient()`
  - Recommendation: Extract common validation function
```

### 12. Test Coverage Analysis
- Document test coverage for functions
- Identify high-impact functions lacking tests
- Note complex functions that need additional testing
- Correlate test coverage with dependency complexity

Example:
```
## Test Coverage Analysis

### Test Coverage by Impact
- High-impact functions: 87% coverage
- Medium-impact functions: 62% coverage
- Low-impact functions: 41% coverage

### Critical Functions Lacking Tests
1. `security::validate_permissions` (0% coverage, 23 callers)
2. `database::transaction::commit` (40% coverage, critical path)
3. `api::authentication::verify_token` (50% coverage, security boundary)
```

## Methodology

### Analysis Approaches
- Static code analysis to extract function calls
- Control flow analysis to trace execution paths
- Call graph visualization tools
- Dependency matrix analysis
- Code instrumentation to validate dynamic calls

### Tools and Techniques
- Function extraction from abstract syntax trees
- Call graph generators
- Metrics calculation (fan-in, fan-out, cyclomatic complexity)
- Visualization libraries for call graphs
- Integration with IDE or code navigation tools

## Integration with Other Documents
BREAKDOWN_FUNCTION_DEPENDENCIES.md should be referenced by and connected to:
- BREAKDOWN.md (general architecture)
- BREAKDOWN_RELATIONSHIPS.md (higher-level dependency overview)
- CROSS-MODULE_FUNCTION_DEPENDENCY_ANALYSIS.md (cross-boundary focus)
- Implementation Plan (for planning function modifications)

## Best Practices

### Focus and Scope
- Focus on critical or complex subsystems
- Prioritize high-impact functions
- Balance breadth and depth of analysis
- Cover representative samples of different function types
- Adjust detail level based on function criticality

### Clarity and Organization
- Use consistent terminology and notation
- Organize by module and functional area
- Provide clear visual aids for complex dependencies
- Include navigation aids for large documents
- Highlight key findings and actionable insights

### Maintenance
- Update when function signatures or dependencies change
- Track function dependency metrics over time
- Auto-generate function dependency data where possible
- Flag potentially unstable or rapidly changing areas
- Include analysis date and tool information

## Example Template
```markdown
# [Project Name] Function Dependency Analysis

## Executive Summary
[Summary of key findings and metrics]

## Function Call Graphs
[Visual representations of key call graphs]

## Module-level Function Dependencies
[Analysis by module]

## Function Block Signatures
[Key function signatures]

## Cross-Module Function Dependencies
[Functions crossing module boundaries]

## Dependency Direction
[Call hierarchies and sequences]

## Dependency Type Analysis
[Types of function dependencies]

## Recursive and Cyclical Dependencies
[Recursive and cyclical function calls]

## Criticality and Impact
[Impact assessment of key functions]

## Change Propagation Analysis
[How changes would propagate]

## Optimization Opportunities
[Potential improvements]

## Test Coverage Analysis
[Test coverage relative to function impact]

## Methodology
[Analysis approach and tools]

## Action Plan
[Prioritized recommendations]
```

## Advanced Applications

### API Evolution Planning
- Use function dependency analysis to plan API changes
- Identify backwards compatibility requirements
- Design interface evolution strategies
- Document deprecation paths

### Performance Optimization
- Identify hot paths for optimization focus
- Detect redundant or inefficient call patterns
- Find opportunities for parallelization
- Spot unnecessary function boundary crossings

### Refactoring Planning
- Identify functions to extract or combine
- Plan dependency inversion opportunities
- Detect inappropriate dependencies to fix
- Design safer incremental refactoring steps

### Codebase Learning
- Use function dependencies to understand code flow
- Create learning paths through the codebase
- Identify key functions to study first
- Build mental models of subsystem interaction
