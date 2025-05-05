# Development Log

This file documents the development process, architectural decisions, and implementation details for the GitCodes MCP project.

**IMPORTANT NOTE:** This devlog contains only changes made by AI agents and may not include modifications made directly by human programmers. There may be discrepancies between the current source code and the history documented here.

## Structure

- Each major change or feature should be documented with a date
- Include design decisions, implementation challenges, and solutions
- Document any significant refactorings or architecture changes
- Note any dependencies added or removed with rationale

## Recent Changes

### 2024-05-06: Rename and Improve Repository Cache Directory Configuration

- Renamed fields and parameters for better semantic clarity:
  - Changed `temp_dir_base` to `repository_cache_dir_base` in RepositoryManager
  - Updated CLI parameter from `--temp-dir` to `--cache-dir` with short form `-c`
  - Updated all method and constructor parameters for consistency
- Improved terminology throughout the codebase:
  - Updated documentation to use "repository cache" instead of "temporary directory"
  - Changed convenience method names from `with_default_temp_dir` to `with_default_cache_dir`
  - Updated log messages to use "repository cache directory" for clarity
- Enhanced CLI help text to better explain the purpose of the parameter
- Updated all related code paths to maintain consistency:
  - Parameter names in constructors
  - Variable names in function implementations
  - Field names in structs
  - Messages in logging statements

### 2024-05-06: Add Custom Temporary Directory Configuration

- Added option to configure custom temporary directory for repository storage:
  - Modified `RepositoryManager::new()` to accept an optional PathBuf for temp_dir_base
  - Added validation to ensure the directory exists and is writable
  - Added fallback to system temporary directory if none provided
  - Implemented proper error handling for validation failures
- Added `--temp-dir` command-line parameter to both stdio and http server modes:
  - Updated clap configuration in the main binary
  - Added clear help text describing the parameter function
  - Implemented proper logging to notify when custom directory is used
- Updated service initialization chain:
  - Modified `GitHubService::new()` and `GitHubCodeTools::new()` to accept temp_dir parameter
  - Added convenience methods `with_default_temp_dir()` for backward compatibility
  - Updated transport implementations to pass the parameter through
- Enhanced robustness with proper I/O validation:
  - Added existence check for specified directory
  - Added check to verify the path is a directory and not a file
  - Added write permission verification using a test file

### 2024-05-06: Improve Type Safety with Path and PathBuf

- Replaced string path representations with proper Path and PathBuf types:
  - Changed `repo_dir` in `RepositoryInfo` from `String` to `PathBuf`
  - Changed `temp_dir_base` in `RepositoryManager` from `String` to `PathBuf`
  - Updated all related method signatures to use `&Path` instead of `&str` for path parameters
  - Modified `get_repo_dir` to use `PathBuf::join()` instead of string concatenation
- Enhanced error handling for path operations:
  - Used `Path::to_string_lossy()` for paths that need to be passed to command-line tools
  - Maintained proper path handling throughout the codebase
- Improved code safety and maintainability:
  - Better type safety by using specialized path types
  - Clearer semantics for path and directory operations
  - More robust path manipulation using standard library functions
- Eliminated potential path handling bugs and edge cases

### 2024-05-06: Improve Encapsulation by Removing Unnecessary Public Modifiers

- Restricted visibility of internal helper functions in the `git_repository` module:
  - Changed `parse_repository_url` from `pub fn` to `fn`
  - Changed `clone_repository` from `pub async fn` to `async fn`
  - Changed `update_repository` from `pub async fn` to `async fn`
  - Changed internal methods on `RepositoryManager` from `pub fn` to `fn` where possible
- Improved code encapsulation by limiting public API surface:
  - Only exposed methods and types required by external modules
  - Kept `RepositoryInfo` struct and its fields public as they're used externally
  - Maintained `parse_and_prepare_repository` method as public since it's needed by `GitHubService`
- Enhanced maintainability by reducing API surface area:
  - Limited exposure of implementation details
  - Made code changes easier by reducing public contract
  - Followed the principle of least privilege

### 2024-05-06: Refactor Repository Manager Methods

- Converted standalone functions in `git_repository.rs` to methods on the `RepositoryManager` struct:
  - Moved `get_repo_dir` to a method on `RepositoryManager`
  - Moved `is_repo_cloned` to a method on `RepositoryManager`
  - Moved `parse_and_prepare_repository` to a method on `RepositoryManager`
- Updated callers in the `GitHubService` class to use the new method-based approach
- Improved the code organization by following object-oriented principles:
  - Methods that operate on `RepositoryManager` state are now properly encapsulated
  - Clear distinction between stateless utility functions and object methods
- Simplified function signatures by removing the redundant manager parameter
- Enhanced code readability and maintainability

### 2024-05-06: Refactor Repository Management to Git Repository Module

- Moved repository management functionality from `GitHubService` to `git_repository.rs`:
  - Extracted `RepositoryInfo` struct to the module
  - Moved `parse_and_prepare_repository` method from `GitHubService` to `git_repository` module
  - Updated GitHubService methods to use the new consolidated function
- Enhanced code organization by completing the separation of concerns between:
  - Repository management (cloning, updating, info extraction)
  - Service orchestration (high-level API)
- Made the repository preparation more consistent across different methods
- Improved readability by reducing redundancy in the code
- Eliminated dead code warning for `RepositoryInfo` fields by making them public

### 2024-05-06: Refactor Code Search Functions to Separate Module

- Created a new `code_search.rs` module for code search functionality:
  - Extracted `perform_code_search` function from `GitHubService` class
  - Extracted `format_search_results` function from `GitHubService` class
  - Made the module private with mod-level visibility
- Updated the `grep_repository` method to use the new functions
- Improved initialization of SearchOptions using struct initialization rather than mutation
- Enhanced code organization by following better separation of concerns:
  - `params.rs` for data structures 
  - `github_api.rs` for API interaction logic
  - `git_repository.rs` for Git operations
  - `code_search.rs` for code search logic 
  - `mod.rs` for service orchestration
- Maintained all functionality while improving code maintainability

### 2024-05-06: Refactor Git Repository Functions to Separate Module

- Extracted repository operations from `GitHubService` to `git_repository.rs`:
  - Moved `clone_repository` function to the `git_repository` module as mentioned in the TODO comment
  - Moved `update_repository` function to the `git_repository` module
  - Removed the duplicate implementations from `GitHubService`
  - Updated function calls in `GitHubService` to use the new functions
- Enhanced code organization by grouping related functionality:
  - All Git operations now reside in the `git_repository` module
  - Service class now delegates repository operations rather than implementing them
- Improved code reusability by making these functions public and stateless
- Enhanced maintainability by eliminating duplicate code
- Completed refactoring mentioned in TODO comment

### 2024-05-06: Refactor GitHub API Functions to Separate Module

- Moved API-related functionality from `GitHubService` to a separate `github_api.rs` module:
  - Extracted `construct_search_url` function from `SearchParams` implementation
  - Moved `execute_search_request` method from `GitHubService` class to a standalone function
  - Made `github_api` module private with mod-level visibility
- Updated `search_repositories` method in `GitHubService` to use the new function
- Simplified token handling in the `GitHubService::new` function
- Added TODO comments for future improvements, including proper error handling using `anyhow::Result<String>`
- Enhanced code organization by following better separation of concerns:
  - `params.rs` for data structures 
  - `github_api.rs` for API interaction logic
  - `mod.rs` for service orchestration
- Maintained documentation for the extracted API methods
- Improved modularity and maintainability by grouping related functionality

### 2024-05-06: Move Git Repository Manager to GitHub Service Package

- Moved `git_repository.rs` from `gitcodes` directory to `github_service` directory
- Reorganized imports and function calls to use the relocated module
- Updated `github_service/mod.rs` to export and use the git repository functionality
- Modified the main `gitcodes/mod.rs` to re-export the repository manager
- Ensured backward compatibility with existing API
- Completed the restructuring of all GitHub-related components into a single package
- Improved code organization by keeping related components together
- Enhanced maintainability with a more consistent module structure

### 2024-05-06: Refactor GitHub Service Components into Separate Package

- Created a dedicated `github_service` directory under `gitcodes` to improve code organization
- Moved parameter-related structs and enums to a separate `params.rs` file:
  - `SearchParams` struct and methods
  - `GrepParams` struct and methods
  - `SortOption` enum
  - `OrderOption` enum
- Moved service implementation to `github_service/mod.rs`
- Updated imports and exports throughout the codebase
- Modified documentation to reflect the new module structure
- Maintained backward compatibility with existing API
- Enhanced code organization by following better separation of concerns
- Improved code maintainability by grouping related components

### 2024-05-06: Create GrepParams Struct for Code Search

- Created a new `GrepParams` struct for code search parameters
- Refactored `GitHubService::grep_repository()` to use the structured parameter type
- Updated `GitHubCodeTools::grep_repository()` to create and pass a GrepParams struct
- Enhanced code documentation with examples for the new parameter type
- Aligned implementation with specification in spec.md which already defined `GrepRequest`
- Improved API consistency between search_repositories and grep_repository methods
- Followed the same pattern used for the SearchParams refactoring
- Added comprehensive Rustdoc comments for better developer experience

### 2024-05-06: Merge SearchParams and Eliminate InternalSearchParams

- Consolidated search parameter handling by removing `InternalSearchParams` struct
- Moved functionality directly into `SearchParams` implementation
- Enhanced encapsulation by adding `construct_search_url()` method directly to `SearchParams`
- Simplified the API by removing an unnecessary abstraction layer
- Improved code readability and maintainability by consolidating parameter validation
- Updated `GitHubService::search_repositories()` to work with the merged implementation
- Reduced code duplication and improved type safety
- Removed internal parameter conversion, making the code more maintainable

### 2024-05-06: Add Command Line GitHub Token Configuration

- Added `--github-token` parameter to command line interface using clap
- Created option for passing GitHub token directly through command line arguments
- Implemented priority authentication flow:
  1. Command line argument (highest priority)
  2. Programmatic parameter (second priority)
  3. Environment variable (fallback)
- Modified transport implementations (stdio and sse_server) to accept the token parameter
- Updated all documentation to reflect the three authentication options
- Enhanced flexibility by supporting multiple authentication methods
- Added tracing information to log when command line token is used
- Ensured backward compatibility with all existing authentication methods

### 2024-05-06: Merge Authentication Methods in GitHubService

- Refactored `GitHubService::new()` to accept an optional GitHub token parameter
- Merged functionality from `with_token()` into the main constructor
- Simplified the API by using a single method with flexible authentication options
- Added token priority: explicit parameter takes precedence over environment variable
- Updated `GitHubCodeTools` constructor to match the changes
- Updated documentation and examples to reflect the unified approach
- Enhanced maintainability by reducing duplicate code
- This change maintains backward compatibility by supporting both authentication methods

### 2024-05-06: Update GitHubService to Accept GitHub Token Parameter

- Modified `GitHubService` to accept a GitHub token directly during initialization
- Added a new `GitHubService::with_token()` method for explicit token configuration
- Added a matching `GitHubCodeTools::with_token()` method to the wrapper class
- Updated documentation to describe both authentication methods (environment variable and programmatic)
- Enhanced user experience by providing flexibility in how authentication is configured
- Updated the `get_info()` method to document both authentication approaches
- This change maintains backward compatibility as the original approach (environment variable) still works

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