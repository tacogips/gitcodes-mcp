# Development Log

This file documents the development process, architectural decisions, and implementation details for the GitCodes MCP project.

## Structure

- Each major change or feature should be documented with a date
- Include design decisions, implementation challenges, and solutions
- Document any significant refactorings or architecture changes
- Note any dependencies added or removed with rationale

## Recent Changes

### 2024-05-06: Refactor construct_search_url Method to InternalSearchParams

- Moved `construct_search_url` method from `GitHubService` to `InternalSearchParams`
- Updated reference in `GitHubService::search_repositories` to use the new method
- Improved encapsulation by placing URL construction logic with the parameters it uses
- Enhanced code organization by following the principle that methods should be on the data they operate on
- Made the API more intuitive as the search parameters now directly construct their URL representation

### 2024-05-06: Refactor build_internal_search_params to Use InternalSearchParams::new

- Modified `build_internal_search_params` method to use the `InternalSearchParams::new()` constructor
- Updated comments in the method to be more concise and descriptive
- Improved code maintainability by using proper constructor pattern
- Removed redundant comment about ensuring per_page limits as this is now handled in the constructor

### 2024-05-06: Add InternalSearchParams Constructor Method

- Added a new `InternalSearchParams::new()` constructor method to improve encapsulation and code clarity
- Constructor method now handles GitHub API limit validation (capping per_page at 100)
- Updated `build_internal_search_params` to use the new constructor method
- Improved code maintainability by centralizing parameter validation and initialization logic
- Enhanced documentation with clearer parameter descriptions

### 2024-05-06: Implement Default Trait for Enums

- Implemented `Default` trait for `SortOption` and `OrderOption` enums
- Added `Relevance` variant to `SortOption` with empty string serialization
- Updated `build_internal_search_params` to use the `Default` implementations
- Removed hardcoded default values ("", "desc") in favor of type-safe defaults
- Improved code maintainability by centralizing default value definitions in the enum types

### 2024-05-06: Add strum for Enum String Conversion

- Added `strum` crate as a dependency for simplifying enum-to-string conversions
- Applied derive macros to `SortOption` and `OrderOption` enums:
  - `Display`: Enables formatting enum values with `{}` in strings
  - `EnumString`: Allows parsing strings into enum values
  - `AsRefStr`: Provides `as_ref()` method for string references
- Used `#[strum(serialize_all = "lowercase")]` to ensure all variants output lowercase by default
- Added specific `#[strum(serialize = "...")]` attributes for custom API string values:
  - `Ascending` → "asc"
  - `Descending` → "desc"
- Simplified the `to_str()` methods to use the derived `as_ref()` method
- Fixed lifetime issues by removing the `'static` constraint
- Maintained backward compatibility with existing API pattern

### 2024-05-06: Refactor Search Parameters to Use Structured Type

- Changed `search_repositories` parameter from individual parameters to a unified `SearchParams` struct
- Converted the existing `SearchParams` struct from an internal implementation detail to a public API parameter
- Added proper documentation for each field with `#[schemars(description = "...")]` annotations
- Updated `tool` description to show the new structured parameter format
- Created an internal `InternalSearchParams` struct to maintain separation between API and implementation details
- Modified the search URL construction to use the new structured parameter type
- Updated example usage in tool documentation to demonstrate the new approach

### 2024-05-05: Remove Leftover Tool Attributes

- Removed remaining `#[tool]` attributes from `GitHubService::search_repositories`
- Cleaned up unused imports (`tool`, `schemars`) from `mod.rs`
- Fixed formatting issues in the module
- Completed the proper separation of core functionality and MCP tool interface

### 2024-05-05: Reorganize Git Repository Code Structure

- Moved `RepositoryManager` struct from `mod.rs` to `git_repository.rs`
- Made `temp_dir_base` field module-private with `pub(crate)` visibility
- Improved encapsulation by keeping Git-related functionality in its dedicated file
- Better organization of code following separation of concerns principles

### 2024-05-05: Refactor MCP Tool Implementation with Wrapper Pattern

- Separated `#[tool(tool_box)]` implementation from `GitHubService`
- Created a new `GitHubCodeTools` wrapper struct for MCP protocol integration
- Moved tool annotations and handler implementation to the wrapper in a dedicated `tools.rs` file
- Implemented a clean separation of concerns between:
  - Core business logic in `GitHubService`
  - MCP protocol integration in `GitHubCodeTools`
- Updated transport implementations to use the wrapper
- Fixed tool parameter handling in the wrapper
- Made core service methods public for use by the wrapper

### 2024-05-05: Refactor GitHubService::grep_repository Method

- Restructured the `grep_repository` method to improve modularity and readability
- Created helper methods to handle specific responsibilities:
  - `parse_and_prepare_repository`: Handles URL parsing and repository preparation
  - `perform_code_search`: Executes the search using lumin search engine
  - `format_search_results`: Formats results for user-friendly output
- Added a `RepositoryInfo` struct to encapsulate repository data between functions
- Improved error handling with more consistent Result return types
- Enhanced documentation for each helper method

### 2024-05-05: Refactor RepositoryManager Creation

- Moved `new_repository_manager()` function to a proper `RepositoryManager::new()` method
- Enhanced object-oriented design by implementing constructor directly on the struct
- Updated `Default` implementation to use the new method
- Removed redundant standalone function from `git_repository.rs`
- Improved code organization and adherence to Rust idioms

### 2024-05-05: Refactor Enum String Conversion to Methods

- Added `to_str()` methods to `SortOption` and `OrderOption` enums 
- Encapsulated string conversion logic within the enum types
- Simplified `build_search_params()` function by using these methods
- Enhanced code maintainability by centralizing conversion logic
- Followed Rust best practices by implementing behavior directly on types

### 2024-05-05: Refactor GitHubService::search_repositories for Better Modularity

- Refactored `search_repositories` method into smaller, focused helper methods
- Created `SearchParams` struct to encapsulate search configuration
- Extracted logic into three primary helper methods:
  - `build_search_params`: Handles parameter preparation and validation
  - `construct_search_url`: Builds the GitHub API URL with all query parameters
  - `execute_search_request`: Handles the HTTP request and response processing
- Improved code organization and maintainability while preserving functionality
- Enhanced code documentation with more detailed method descriptions

### 2024-05-05: Rename GitHubRepositoryRouter to GitHubService

- Renamed `GitHubRepositoryRouter` to `GitHubService` to better reflect its purpose as a service provider rather than a router
- Removed "Router" terminology as the component doesn't perform routing functionality
- Updated struct documentation to clarify its role as an integrated service for GitHub operations
- Updated all references throughout the codebase (transport implementations, exports, etc.)

### 2024-05-05: Rename CargoDocRouter to GitHubRepositoryRouter

- Renamed `CargoDocRouter` to `GitHubRepositoryRouter` to better reflect its purpose and functionality
- Updated all references across the codebase to use the new name
- Removed mentions of planned Rust crate documentation in router documentation
- Updated imports and usages in transport implementations

### 2024-05-05: Code Structure Refactoring for Repository Management

- Refactored the `RepositoryManager` implementation to address compilation errors
- Moved method implementations from the struct to standalone functions to improve modularity
- Fixed function signature issues in `git_repository.rs` and `mod.rs`
- Updated method calls in `mod.rs` to use the refactored functions from `git_repository`
- Properly exported functions across module boundaries to ensure visibility
- Fixed the return type of `parse_repository_url` to use `String` instead of `str`

### 2024-05-04: Implement Lumin Integration for Code Search

- Added `lumin` v1.0.3 dependency for file search functionality
- Replaced direct git grep command execution with lumin's search API
- Improved code organization and error handling in the code search functionality
- Aligned implementation with the original specification in spec.md

### 2024-05-04: Dependency Documentation Update

- Confirmed the removal of `lumin` dependency as documented in the implementation challenges
- Noted the discrepancy between spec.md (which references lumin) and the actual implementation
- Verified that direct git command execution is used instead of lumin for grepping

### 2024-05-01: Initial Implementation of Model Context Protocol for GitHub

Implemented the core functionality specified in `spec.md`:

1. **GitHub Repository Search Tool**
   - Created a tool to search for repositories using the GitHub API
   - Implemented sorting, pagination, and proper error handling
   - Added authentication support via GitHub token

2. **GitHub Repository Code Grep Tool**
   - Implemented code search functionality using git clone and git grep
   - Added support for regex search, case sensitivity options
   - Created repository cloning and update logic with proper state management

3. **GitHub Repository Branches/Tags List Tool**
   - Added ability to list all branches and tags for a repository
   - Implemented proper formatting for branch/tag display

### Implementation Challenges

#### Dependency Management Issues

- Initially attempted to use `git2` and `gitoxide` libraries, but encountered dependency conflicts with `libgit2-sys` and `lumin` 
- Switched to using direct git command execution via `std::process::Command` for simplicity and to avoid dependency issues
- Removed `git2` and `gitoxide` from direct dependencies to resolve build errors

#### Concurrent Task Management

- Had to handle moved values in async closures carefully, using cloning for strings passed to `tokio::task::spawn_blocking`
- Implemented proper error handling for CLI git commands with context-aware error messages
- Created a repository manager to handle temporary directory management and repository state

#### Building without OpenSSL and Other Native Dependencies

- Encountered OpenSSL dependency issues during build
- Modified `reqwest` configuration to use `default-features = false` to avoid requiring OpenSSL

### Architecture Decisions 

1. **Repository Management**
   - Created a `RepositoryManager` class to handle git repository operations
   - Used a unique naming scheme for temporary directories to prevent conflicts
   - Added proper cleanup and error handling for repository operations

2. **Response Format**
   - Simplified return values to use strings instead of complex JSON structures
   - Added formatted output for search results and repository information

3. **Error Handling Strategy**
   - Used context-aware error messages to help users understand issues
   - Implemented graceful fallbacks for commands that might fail
   - Added proper resource cleanup in error cases

### 2024-05-05: GitHub Authentication Documentation

Added detailed documentation regarding the use of the `GITCODE_MCP_GITHUB_TOKEN` environment variable:

- The environment variable `GITCODE_MCP_GITHUB_TOKEN` is used for GitHub API authentication
- Authentication is optional but recommended to avoid rate limiting:
  - Unauthenticated requests: Limited to 60 requests/hour
  - Authenticated requests: 5,000 requests/hour
- Token usage:
  - When provided, the token is stored in memory when MCP starts
  - It is not referenced from the environment variable after initialization
  - For accessing private repositories, the token must have the `repo` scope
- When token is not provided:
  - The system falls back to unauthenticated requests
  - All public repository operations still work
  - Rate limits are significantly lower (60 vs 5,000 requests/hour)
  - Private repositories will not be accessible
- Token security:
  - Tokens are never logged or exposed in error messages
  - Tokens are only used for GitHub API requests, not for local operations

### Future Improvements

1. **Security Enhancements**
   - Add credential management for authenticated git operations
   - Implement proper token handling and security for GitHub API requests
   
2. **Performance Optimizations**
   - Add caching for repository operations to reduce git command calls
   - Implement smarter code search algorithms to handle large repositories

3. **Features**
   - Add support for additional GitHub operations like pull requests
   - Implement diff visualization for comparing branches/commits