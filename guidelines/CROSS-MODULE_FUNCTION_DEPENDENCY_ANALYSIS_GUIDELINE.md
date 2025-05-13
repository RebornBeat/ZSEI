# CROSS-MODULE_FUNCTION_DEPENDENCY_ANALYSIS_GUIDELINE.md

## Purpose

The purpose of the Cross-Module Function Dependency Analysis is to specifically identify, document, and analyze the dependencies between functions that span module boundaries in the codebase. This focused analysis provides critical insights into the integration points, coupling patterns, and potential fragility in the codebase's architecture. By understanding these cross-boundary dependencies, developers can better manage changes, refactor code more safely, and improve the system's modularity.

## Creation Timing

Create a Cross-Module Function Dependency Analysis:
- After completing BREAKDOWN.md and BREAKDOWN_RELATIONSHIPS.md
- Before making significant changes to module interfaces
- When experiencing integration issues or unexpected side effects
- When planning to refactor module boundaries
- As part of the evaluation phase in implementation planning
- When onboarding developers who need to understand system integration

## Structure

### 1. Executive Summary

- Overview of cross-module dependency patterns and system integration
- Summary of key metrics (number of cross-module calls, most-used interfaces)
- Identification of potential architectural issues (tight coupling, responsibility bleed)
- Highlight critical cross-module interfaces that require careful management
- Recommendations for improving module boundaries and interfaces

Example:
```
# Cross-Module Function Dependency Analysis for [Project Name]

## Executive Summary

This analysis identified 127 cross-module function dependencies spanning 14 modules with these key findings:

1. Interface Stability: The database module provides the most heavily-used cross-module interface (32% of all cross-module calls target it)

2. Coupling Hotspots: The utils module is called by every other module, indicating potential responsibility diffusion

3. Architectural Concerns:
   - Layering violations: 3 instances of lower layers calling higher layers
   - Unexpected dependencies: UI module directly calls database functions
   - Missing abstractions: Multiple modules implement similar logging interfaces

4. Critical cross-module interfaces requiring careful management:
   - Authentication service API (used by 8 modules)
   - Configuration management (used by all modules)
   - Error handling framework (used by all modules)
```

### 2. Cross-Module Call Graph

- Visual representation of cross-module function calls
- Show direction and intensity of cross-module dependencies
- Highlight unexpected or concerning dependencies
- Identify architectural layers and boundary crossings

Example:
```
## Cross-Module Call Graph
```
[Include visual graph with modules as nodes and function calls as directional edges]

### 3. Module Interface Analysis

- For each module, document:
  - The functions it exposes to other modules
  - Which modules call each exported function
  - The purposes of these cross-module calls
  - The stability and change frequency of exposed interfaces

Example:
```
## Module Interfaces

### Module: authentication

#### Exported Functions:
1. `authenticate_user(credentials: Credentials) -> Result<User, AuthError>`
   - Called by: api (3 functions), ui (2 functions), background_jobs (1 function)
   - Purpose: Verify user credentials and create authenticated session
   - Stability: High (unchanged for 8 months)
   - Change Frequency: Low (1 change in past year)
   - Impact Radius: 6 functions in 3 modules

2. `validate_token(token: String) -> Result<User, AuthError>`
   - Called by: api (7 functions), websocket (2 functions), background_jobs (3 functions)
   - Purpose: Verify authentication token for API calls
   - Stability: Medium (minor changes every ~3 months)
   - Change Frequency: Medium (3 changes in past year)
   - Impact Radius: 12 functions in 3 modules
```

### 4. Module Dependency Matrix

- Create a matrix showing which modules depend on which others
- Quantify the number of dependencies between each module pair
- Highlight strong dependencies and unexpected relationships
- Identify potential architectural violations

Example:
```
## Module Dependency Matrix

|                | api | auth | database | config | utils | ui |
|----------------|-----|------|----------|--------|-------|-----|
| **api**        | -   | 12   | 8        | 5      | 20    | 0   |
| **auth**       | 0   | -    | 15       | 7      | 13    | 0   |
| **database**   | 0   | 0    | -        | 4      | 9     | 0   |
| **config**     | 0   | 0    | 0        | -      | 6     | 0   |
| **utils**      | 0   | 0    | 0        | 0      | -     | 0   |
| **ui**         | 8   | 5    | 3        | 4      | 18    | -   |

Architectural concerns:
- UI directly calls database (3 calls, violates layering)
- Database makes no outgoing calls (as expected for data layer)
- Utils called by all modules but makes no outgoing calls (as expected)
```

### 5. Function-Level Cross-Module Dependencies

- Document all functions that call across module boundaries
- Include source and target functions with signatures
- Note the nature and purpose of each cross-module call
- Identify parameter and return value dependencies
- Flag potential issues (tight coupling, complex dependencies)

Example:
```
## Function-Level Cross-Module Dependencies

### From Module: api
#### Function: `api::routes::users::create_user`
```rust
async fn create_user(
    req: HttpRequest,
    json: web::Json<CreateUserRequest>
) -> HttpResponse
```

Cross-module calls:
1. → `auth::users::create_user`
   ```rust
   async fn create_user(data: NewUserData) -> Result<User, AuthError>
   ```
   - Purpose: Delegate user creation to auth module
   - Data dependencies: Transforms CreateUserRequest to NewUserData
   - Return handling: Maps Result<User, AuthError> to HttpResponse

2. → `utils::validation::validate_email`
   ```rust
   fn validate_email(email: &str) -> bool
   ```
   - Purpose: Validate email format before submission
   - Data dependencies: Extracts email string from request
   - Return handling: Returns 400 if validation fails
```

### 6. Module Boundary Transition Analysis

- Analyze how data transforms when crossing module boundaries
- Document data marshalling and translation between modules
- Identify potential type safety issues at boundaries
- Note serialization/deserialization requirements
- Highlight complex data dependencies

Example:
```
## Module Boundary Transitions

### Transition: api → auth
Data transformations:
- `CreateUserRequest` → `NewUserData` (manual mapping)
- `LoginRequest` → `Credentials` (direct field mapping)
- `HttpRequest` → `RequestContext` (header extraction)

Return transformations:
- `Result<User, AuthError>` → `HttpResponse` (status mapping + serialization)
- `Result<Token, AuthError>` → `HttpResponse` (status mapping + serialization)

Potential issues:
- Inconsistent error mapping (some errors mapped to 400, others to 500)
- Manual mapping could get out of sync with schema changes
```

### 7. Critical Path Analysis

- Identify critical cross-module execution paths
- Document function call chains that cross multiple modules
- Analyze performance and reliability implications
- Note potential bottlenecks or single points of failure

Example:
```
## Critical Cross-Module Paths

### User Authentication Flow
1. `api::routes::auth::login` (api module)
2. → `auth::authenticate_user` (auth module)
3. → `database::users::find_by_username` (database module)
4. → `auth::verify_password` (auth module)
5. → `auth::generate_token` (auth module)
6. → `database::tokens::store_token` (database module)

Performance implications:
- 2 round trips to database
- Authentication takes ~150ms average (mostly database time)
- Bottlenecks: Password verification (~100ms for bcrypt)

Reliability concerns:
- Token storage failure would leave user authenticated but unable to use token
- Database unavailability affects entire chain
```

### 8. Circular Dependency Analysis

- Identify circular dependencies across module boundaries
- Analyze the nature and cause of these circular dependencies
- Document the impact and risks of each circular dependency
- Suggest strategies for breaking harmful circular dependencies

Example:
```
## Circular Dependencies

### auth ↔ user
Circular chain:
- `auth::authenticate_user` calls `user::get_by_credentials`
- `user::update_last_login` calls `auth::get_current_session`

Impact:
- Complicates testing (each module requires the other)
- Creates initialization order problems
- Makes refactoring difficult

Resolution strategies:
1. Extract session management to separate module
2. Inject dependencies rather than direct calls
3. Use events for login tracking instead of direct calls
```

### 9. Dependency Type and Pattern Analysis

- Categorize cross-module dependencies by type:
  - Direct function calls
  - Interface implementations
  - Event subscriptions
  - Dependency injection
  - Shared data structures
- Identify common patterns and anti-patterns in cross-module dependencies

Example:
```
## Dependency Patterns

### Observed Patterns

1. Service Pattern (43% of cross-module calls)
   - Module exposes service interface
   - Other modules consume service through well-defined API
   - Example: `authentication` module provides auth services

2. Direct Data Access (27% of cross-module calls)
   - Module directly reads/writes data owned by another module
   - Often bypasses business logic
   - Example: `reports` module directly queries `users` tables

3. Event-based (15% of cross-module calls)
   - Module subscribes to events emitted by another
   - Loose coupling but harder to trace
   - Example: `notification` subscribes to `order` events

4. Shared Data Structures (15% of cross-module calls)
   - Modules share common data types
   - Creates implicit coupling through structure changes
   - Example: `Order` struct used by 5 different modules

### Anti-patterns

1. Shotgun Surgery Risk
   - Changes to `config` module require changes in all 12 other modules
   - No abstraction layer to isolate changes

2. Feature Envy
   - `reports` module contains logic that should belong in `orders`
   - Makes extensive calls to `orders` functionality
```

### 10. Module Interface Stability Analysis

- Assess the stability of module interfaces
- Document change frequency and impact of past changes
- Identify interfaces that require careful management
- Suggest improvements for unstable interfaces

Example:
```
## Interface Stability Analysis

### Stable Interfaces (unchanged > 6 months)
- `utils::logging` (used by all modules)
- `database::connection` (used by 7 modules)
- `crypto::hash` (used by 3 modules)

### Evolving Interfaces (changed in past 6 months)
- `api::client` (3 changes, impacted 5 modules)
- `auth::permissions` (2 changes, impacted 4 modules)
- `storage::files` (5 changes, impacted 3 modules)

### Volatile Interfaces (changed monthly or more)
- `ui::components` (12 changes, impacted 3 modules)
- `analytics::tracking` (8 changes, impacted 2 modules)

Recommendations:
1. Create stable adapter for `ui::components` to isolate frequent changes
2. Version the `analytics::tracking` interface to manage evolution
3. Document `api::client` changes more thoroughly
```

### 11. Cross-Cutting Concerns Analysis

- Identify concerns that span multiple modules
- Analyze how these concerns are handled across module boundaries
- Document inconsistencies or duplication in cross-cutting functionality
- Suggest improvements for managing cross-cutting concerns

Example:
```
## Cross-Cutting Concerns

### Identified Cross-Cutting Concerns:

1. Logging
   - Implemented in 8 different ways across modules
   - Inconsistent log levels and formats
   - Recommendation: Extract unified logging framework

2. Error Handling
   - 4 different error handling approaches
   - Inconsistent error translation at module boundaries
   - Recommendation: Standardize on Result type with common error enum

3. Authentication/Authorization
   - Handled at multiple levels (API, service, data)
   - Duplicate permission checks
   - Recommendation: Centralize in auth module with clear delegation
```

### 12. Recommendations for Improvement

- Provide actionable recommendations based on the analysis
- Prioritize improvements by impact and feasibility
- Suggest specific refactorings to improve module boundaries
- Recommend architectural adjustments to reduce harmful coupling

Example:
```
## Recommendations

### High Priority (Significant architectural impact)
1. Extract Data Access Layer
   - Problem: 5 modules directly access database, bypassing domain logic
   - Solution: Create repository interfaces in domain layer, implement in data layer
   - Impact: Decouples domain logic from data access, enables testing

2. Break auth ↔ user circular dependency
   - Problem: Circular dependency creates tight coupling
   - Solution: Extract session management to separate module
   - Impact: Simplifies testing, initialization, and future changes

### Medium Priority (Maintainability improvements)
1. Standardize Cross-Module Error Handling
   - Problem: Inconsistent error handling across module boundaries
   - Solution: Create common error types and translation patterns
   - Impact: More consistent error reporting, better diagnostics

2. Reduce utils Module Coupling
   - Problem: Every module depends on utils, creating change ripple effects
   - Solution: Split utils into cohesive sub-modules with stable interfaces
   - Impact: More focused dependencies, reduced change impact
```

## Methodology and Tools

### Data Collection Methods
- Static analysis tools to identify function calls
- Code inspection to verify and classify dependencies
- Version control history to assess interface stability
- Developer interviews to understand architectural intent
- Runtime analysis to capture dynamic dependencies

### Analysis Techniques
- Graph analysis of module dependencies
- Matrix analysis to identify clusters and patterns
- Impact analysis for critical functions
- Architectural conformance checking
- Layering violation detection

### Visualization Approaches
- Directed graphs for module dependencies
- Heat maps for dependency intensity
- Sankey diagrams for function call flows
- Chord diagrams for inter-module relationships
- Layer diagrams to show architectural boundaries

## Integration with Other Documents

The Cross-Module Function Dependency Analysis should be referenced by and connected to:
- BREAKDOWN.md (general architecture)
- BREAKDOWN_RELATIONSHIPS.md (higher-level relationship overview)
- BREAKDOWN_FUNCTION_DEPENDENCIES.md (detailed function relationships)
- Implementation Planning documents (for planning cross-module changes)

## Best Practices

### Focus and Scope
- Focus on true cross-module dependencies (not intra-module)
- Prioritize frequently changing or critical interfaces
- Analyze representative examples of different dependency types
- Pay special attention to architectural boundaries
- Balance breadth and depth based on system complexity

### Clarity and Organization
- Use consistent terminology and notation
- Organize findings by module and by insight type
- Provide visual aids for complex dependency networks
- Highlight architectural violations and concerns
- Include both details and high-level summaries

### Maintenance
- Update when module interfaces change significantly
- Track key metrics over time to identify trends
- Verify findings through code review and developer feedback
- Consider automating parts of the analysis
- Include analysis date and scope to provide context

## Example Template

```markdown
# [Project Name] Cross-Module Function Dependency Analysis

## Executive Summary
[Summary of key findings and recommendations]

## Cross-Module Call Graph
[Visual representation of module dependencies]

## Module Interface Analysis
[Detailed analysis of module interfaces]

## Module Dependency Matrix
[Matrix showing inter-module dependencies]

## Function-Level Cross-Module Dependencies
[Specific cross-module function calls]

## Module Boundary Transition Analysis
[How data transforms across boundaries]

## Critical Path Analysis
[Key execution paths crossing modules]

## Circular Dependency Analysis
[Circular dependencies and their impact]

## Dependency Type and Pattern Analysis
[Patterns in cross-module dependencies]

## Module Interface Stability Analysis
[Interface stability assessment]

## Cross-Cutting Concerns Analysis
[Analysis of cross-cutting concerns]

## Recommendations for Improvement
[Prioritized recommendations]

## Methodology and Tools
[Analysis approach and tools used]

## Appendix
[Additional details and supporting data]
```

## Advanced Applications

### Architecture Conformance Checking
- Use cross-module analysis to verify architectural rules
- Check for layer violations and unauthorized dependencies
- Validate that modules respect boundaries
- Ensure cross-cutting concerns are handled consistently

### API Evolution Management
- Track cross-module interface changes over time
- Identify breaking vs. non-breaking changes
- Plan safer module interface evolution
- Design deprecation strategies for changing interfaces

### Technical Debt Assessment
- Identify architectural debt in module relationships
- Quantify coupling and the cost of change
- Detect dependency anti-patterns
- Create remediation plans for problematic dependencies

### System Understanding
- Use cross-module analysis to understand system integration
- Create "mental maps" of system behavior
- Identify key integration points for knowledge transfer
- Support onboarding of new developers
