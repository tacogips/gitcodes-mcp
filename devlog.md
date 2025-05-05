# Development Log

This file documents the development process, architectural decisions, and implementation details for the GitCodes MCP project.

## Structure

- Each major change or feature should be documented with a date
- Include design decisions, implementation challenges, and solutions
- Document any significant refactorings or architecture changes
- Note any dependencies added or removed with rationale

## Recent Changes

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