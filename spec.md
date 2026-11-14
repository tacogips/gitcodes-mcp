# Model Context Protocol (MCP) - Library Specification

## Purpose and Overview

This MCP (Model Context Protocol) provides a set of tools for AI assistants to efficiently search, analyze, and reference external source code. The protocol enables AI to:

### https://modelcontextprotocol.io/introduction

- Search for relevant repositories on GitHub
- Perform detailed analysis of specific repositories using code grep
- Browse repository branches and tags

Main use cases:

- Searching for code examples and patterns
- Investigating specific implementation methods
- Understanding how to use libraries and frameworks
- Analyzing differences between versions

## Basic Design and Common Features

### Process-Specific Identifier

When MCP starts, each `RepositoryManager` instance is assigned a unique process identifier. This identifier combines the current process ID and a random UUID to ensure uniqueness, even when multiple processes or instances run simultaneously.

```rust
// Inside RepositoryManager
fn generate_process_id() -> String {
    use std::process;
    use uuid::Uuid;
    
    let pid = process::id();
    let uuid = Uuid::new_v4();
    
    format!("{}_{}", pid, uuid.simple())
}
```

This process ID is included in repository cache directory names to prevent conflicts between concurrent processes. The repository manager also uses a global singleton pattern with `once_cell` to ensure a single instance with a consistent process ID is maintained throughout the application lifetime:

```rust
use once_cell::sync::OnceCell;

// Global RepositoryManager instance
static GLOBAL_REPOSITORY_MANAGER: OnceCell<RepositoryManager> = OnceCell::new();

// Initialize and access functions
pub fn init_repository_manager(
    github_token: Option<String>,
    repository_cache_dir: Option<PathBuf>,
) -> &'static RepositoryManager {
    GLOBAL_REPOSITORY_MANAGER.get_or_init(move || {
        RepositoryManager::new(github_token, repository_cache_dir)
            .expect("Failed to initialize global repository manager")
    })
}

pub fn get_repository_manager() -> &'static RepositoryManager {
    GLOBAL_REPOSITORY_MANAGER
        .get_or_init(|| RepositoryManager::with_default_cache_dir())
}
```

### Standard Response Format

All tools follow a unified response format:

```rust
pub struct ToolResponse<T> {
    // Whether the operation was successful
    pub success: bool,
    // Result data (tool-specific type)
    pub data: Option<T>,
    // Error information (if failed)
    pub error: Option<ErrorInfo>,
    // Metadata (execution time, resources used, etc.)
    pub metadata: ResponseMetadata,
}

pub struct ErrorInfo {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

pub struct ResponseMetadata {
    pub execution_time_ms: u64,
    pub rate_limit_remaining: Option<u32>,
    pub rate_limit_reset: Option<u64>,
}
```

### CallToolResult Type

`CallToolResult` is a standard type that wraps the results of tool calls:

```rust
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CallToolResult {
    pub content: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

impl CallToolResult {
    pub fn success(content: Vec<Content>) -> Self {
        CallToolResult {
            content,
            is_error: Some(false),
        }
    }
    pub fn error(content: Vec<Content>) -> Self {
        CallToolResult {
            content,
            is_error: Some(true),
        }
    }
}
```

This type has changed from previous implementations and is now implemented as a `struct` rather than an `enum`. The error state is represented by the `is_error` field, and error details are included in the `content` field.

````

## Provided Tools

### 1. GitHub Repository Search Tool

Uses the GitHub API to search for repositories.

#### Input Parameters

```rust
pub struct SearchParams {
    // Search query (required)
    pub query: String,
    // How to sort results (optional, default is "relevance")
    pub sort_by: Option<SortOption>,
    // Sort order (optional, default is "descending")
    pub order: Option<OrderOption>,
    // Results per page (optional, default is 30, max 100)
    pub per_page: Option<u8>,
    // Result page number (optional, default is 1)
    pub page: Option<u32>,
}

// SearchParams struct only contains parameters, no URL construction method
// URL construction is now handled by a function in the github_api module:
fn construct_search_url(param: &SearchParams) -> String {
    // Implementation that handles parameter defaults and builds the search URL
}

// Unified SortOption enum in the repository_manager module
pub enum SortOption {
    Relevance, // Default, no specific sort parameter
    Stars,     // Sort by number of stars (popularity)
    Forks,     // Sort by number of forks (derived projects)
    Updated,   // Sort by most recently updated
}

// Unified OrderOption enum in the repository_manager module
pub enum OrderOption {
    Ascending,  // Sort in ascending order (lowest to highest, oldest to newest)
    Descending, // Sort in descending order (highest to lowest, newest to oldest)
}

// Provider-specific enums with conversion from generic options
pub enum GithubSortOption {
    Relevance, 
    Stars,     
    Forks,     
    Updated,   
}

// Implement conversion from generic SortOption to GitHub-specific option
impl From<SortOption> for GithubSortOption {
    fn from(value: SortOption) -> Self {
        match value {
            SortOption::Relevance => Self::Relevance,
            SortOption::Stars => Self::Stars,
            SortOption::Forks => Self::Forks,
            SortOption::Updated => Self::Updated,
        }
    }
}
````

#### Implementation Details

- API endpoint: `https://api.github.com/search/repositories?q={query}`
- URL construction and HTTP requests are handled in the `github_api` module
- Reference documentation: https://docs.github.com/en/rest/search/search

#### API Authentication

There are two ways to provide authentication:

1. **Environment Variable**:

   - Personal access token provided via the `GITCODE_MCP_GITHUB_TOKEN` environment variable
   - This token is stored in memory when MCP starts and is not referenced from the environment variable thereafter

2. **Programmatic API**:

   - Token can be provided directly when initializing `GitHubService` or `GitHubCodeTools`:

   ```rust
   // Direct initialization with token
   let git_service = GitHubService::new(Some("your_github_token".to_string()));

   // Or with the wrapper class
   let github_tools = GitHubCodeTools::new(Some("your_github_token".to_string()));
   ```

Authentication behavior:

- If no token is provided, unauthenticated requests are used (with rate limits)
- Unauthenticated requests: 60 requests/hour
- Authenticated requests: 5,000 requests/hour

> **Note**: Access to private repositories requires an access token with appropriate permissions. The token needs at least the `repo` scope (permission to access private repositories).

#### Return Value

```rust
pub struct SearchRepositoriesResult {
    // List of repositories in search results
    pub repositories: Vec<Repository>,
    // Total number of search results
    pub total_count: u32,
    // Current page number
    pub page: u32,
    // Results per page
    pub per_page: u8,
}

pub struct Repository {
    pub name: String,
    pub full_name: String,
    pub private: bool,
    pub html_url: String,
    pub description: Option<String>,
    pub fork: bool,
    pub created_at: String,
    pub updated_at: String,
    pub pushed_at: String,
    pub git_url: String,
    pub ssh_url: String,
    pub clone_url: String,
    pub svn_url: String,
    pub homepage: Option<String>,
    pub language: Option<String>,
    pub license: Option<License>,
    pub topics: Vec<String>,
    pub visibility: String,
    pub default_branch: String,
}

pub struct License {
    pub key: String,
    pub name: String,
    pub spdx_id: String,
    pub url: Option<String>,
}
```

#### Response Example

```json
{
  "success": true,
  "data": {
    "repositories": [
      {
        "name": "llm",
        "full_name": "simonw/llm",
        "private": false,
        "html_url": "https://github.com/simonw/llm",
        "description": "Access large language models from the command-line",
        "fork": false,
        "created_at": "2023-04-01T21:16:57Z",
        "updated_at": "2025-04-30T14:24:55Z",
        "pushed_at": "2025-04-23T17:55:27Z",
        "git_url": "git://github.com/simonw/llm.git",
        "ssh_url": "git@github.com:simonw/llm.git",
        "clone_url": "https://github.com/simonw/llm.git",
        "svn_url": "https://github.com/simonw/llm",
        "homepage": "https://llm.datasette.io",
        "language": "Python",
        "license": {
          "key": "apache-2.0",
          "name": "Apache License 2.0",
          "spdx_id": "Apache-2.0",
          "url": "https://api.github.com/licenses/apache-2.0"
        },
        "topics": ["ai", "llms", "openai"],
        "visibility": "public",
        "default_branch": "main"
      }
    ],
    "total_count": 145,
    "page": 1,
    "per_page": 30
  },
  "metadata": {
    "execution_time_ms": 234,
    "rate_limit_remaining": 4998,
    "rate_limit_reset": 1620000000
  }
}
```

### 2. GitHub Repository Code Grep Tool

Clones the specified GitHub repository locally and greps the code. Supports both public and private repositories.

#### Input Parameters

```rust
pub struct GrepParams {
    // Repository URL (required) - supports the following formats:
    // - git@github.com:{user_name}/{repo}.git (most reliable for git operations)
    // - https://github.com/{user_name}/{repo} (with automatic fallback to SSH if HTTPS fails)
    // - github:{user_name}/{repo}
    pub repository: String,
    // Branch or tag (optional, default is main or master)
    pub ref_name: Option<String>,
    // Search pattern (required)
    pub pattern: String,
    // Whether to be case-sensitive (optional, default is false)
    pub case_sensitive: Option<bool>,
    // Whether to use regex (optional, default is true)
    pub use_regex: Option<bool>,
    // File extensions to search (optional, e.g., ["rs", "toml"])
    pub file_extensions: Option<Vec<String>>,
    // Directories to exclude from search (optional, e.g., ["target", "node_modules"])
    pub exclude_dirs: Option<Vec<String>>,
}
```

#### Implementation Details

1. Parse the repository URL and extract the username and repository name
2. Generate a temporary directory: `{system_temp_dir}/mcp_gitcodes_{user_name}_{repo}_{hash}_pid{process_id}`
   - Where `hash` is a short hash of the repository path
   - And `process_id` is the unique process identifier from the RepositoryManager
3. Create a GitRemoteRepositoryInfo instance to encapsulate clone parameters:
   ```rust
   pub struct GitRemoteRepositoryInfo {
       // GitHub username or organization
       pub user: String,
       // Repository name
       pub repo: String,
       // Branch or tag name to checkout (optional)
       pub ref_name: Option<String>,
   }
   ```
4. Check if the repository is already cloned
   - If cloned:
     - Validate the existing repository directory for integrity
     - If valid, reuse the existing repository
     - If invalid, clean up the directory and proceed with a fresh clone
   - If not cloned:
     - Create the parent directory structure if it doesn't exist
     - Use the gitoxide (`gix`) library to perform a shallow clone:
       ```rust
       // Initialize a repo for fetching
       let mut fetch = PrepareFetch::new(
           auth_url.as_str(),
           repo_dir,
           Kind::WithWorktree,
           gix::create::Options::default(),
           OpenOptions::default(),
       )?;
       
       // Set up shallow clone with depth=1
       let depth = NonZeroU32::new(1).unwrap();
       fetch = fetch.with_shallow(Shallow::DepthAtRemote(depth));
       
       // Perform the clone operation
       fetch.fetch_then_checkout(&mut Discard, &gix::interrupt::IS_INTERRUPTED)?;
       ```
     - Add authentication if a GitHub token is available by incorporating it into the URL:
       ```rust
       // Add token to URL for authentication if it's a GitHub HTTPS URL
       if clone_url.starts_with("https://github.com") {
           auth_url = format!(
               "https://{}:x-oauth-basic@{}",
               token,
               clone_url.trim_start_matches("https://")
           );
       }
       ```
5. Use the lumin crate to perform code search
6. Return results in the standard response format

#### Temporary Directory Management

- Reuse existing directories if they exist
- Update directories in the following cases:
  - More than 24 hours since the last update
  - The requested branch/tag is different from the current one
- Automatic cleanup when MCP shuts down
- Automatic deletion of directories not accessed for more than 7 days
- Delete oldest repositories when total capacity limit (default: 10GB) is reached

#### Return Value

```rust
pub struct GrepResult {
    // List of matched files
    pub matches: Vec<FileMatch>,
    // Search statistics
    pub stats: SearchStats,
}

pub struct FileMatch {
    // File path (relative to repository root)
    pub path: String,
    // Matched lines and their content
    pub line_matches: Vec<LineMatch>,
}

pub struct LineMatch {
    // Line number
    pub line_number: u32,
    // Line content
    pub line: String,
    // Matched ranges within the line (start position and length)
    pub ranges: Vec<(usize, usize)>,
}

pub struct SearchStats {
    // Total number of files searched
    pub files_searched: u32,
    // Total number of matches found
    pub total_matches: u32,
    // Number of files with at least one match
    pub files_with_matches: u32,
    // Time taken for search (milliseconds)
    pub execution_time_ms: u64,
}
```

#### Response Example

```json
{
  "success": true,
  "data": {
    "matches": [
      {
        "path": "src/main.rs",
        "line_matches": [
          {
            "line_number": 42,
            "line": "    async fn process_request(&self, req: Request) -> Result<Response> {",
            "ranges": [[4, 10]]
          }
        ]
      }
    ],
    "stats": {
      "files_searched": 156,
      "total_matches": 23,
      "files_with_matches": 5,
      "execution_time_ms": 345
    }
  },
  "metadata": {
    "execution_time_ms": 1234
  }
}
```

### 3. GitHub Repository Branches/Tags List Tool

Retrieves a list of branches and tags for the specified GitHub repository.

#### Input Parameters

```rust
pub struct ListRefsRequest {
    // Repository URL (required) - supports the following formats:
    // - git@github.com:{user_name}/{repo}.git (most reliable)
    // - https://github.com/{user_name}/{repo} (with automatic fallback to SSH if HTTPS fails)
    // - github:{user_name}/{repo}
    pub repository: String,
}
```

#### Implementation Details

- Create or reuse a local checkout of the repository using the same method as the Grep tool
- Use the gitoxide crate to extract branch and tag information

#### Return Value

```rust
pub struct RefsResult {
    // List of branches
    pub branches: Vec<String>,
    // List of tags
    pub tags: Vec<String>,
}
```

#### Response Example

```json
{
  "success": true,
  "data": {
    "branches": ["main", "develop"],
    "tags": ["v0.0.1", "v0.1.0"]
  },
  "metadata": {
    "execution_time_ms": 123
  }
}
```

## Error Handling

MCP appropriately handles the following error situations:

### API-Related Errors

- Network errors: Automatic retry (exponential backoff)
- Authentication errors: Attempt to validate the token and notify the user
- Rate limit errors: Calculate wait time and return the next possible request time

### Git Operation Errors

- Clone failure: Detailed error message and repository information validation
- Checkout failure: Information about non-existent branches/tags
- Permission errors: Clear explanation of access rights problems

### Temporary File Errors

- Insufficient disk space: Attempt cleanup and notify required space
- Write permission errors: Try alternative directories

All errors are handled through standard Rust Result types and returned with meaningful error messages.

### Error Response Example

```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "rate_limit_exceeded",
    "message": "GitHub API rate limit exceeded. Reset at 2023-05-01T12:00:00Z",
    "details": {
      "rate_limit_reset": 1620000000,
      "rate_limit_remaining": 0
    }
  },
  "metadata": {
    "execution_time_ms": 45
  }
}
```

## Input Validation

All inputs are validated before use, and invalid inputs are reported as errors early:

### Repository URL Validation

- Format check: Verify it's a valid GitHub repository URL
- Existence check: Verify the repository exists (optional)

### Search Query Validation

- Empty strings are not allowed
- Unsafe characters or patterns are escaped or rejected

### Branch/Tag Validation

- Verify it's an existing branch/tag
- Verify it doesn't contain invalid characters

### Grep Search Pattern Validation

- Verify it's valid as a regular expression (if use_regex=true)
- Check pattern complexity (overly complex patterns may impact performance)

## Performance Considerations

MCP maintains high performance through the following strategies:

### Repository Operation Optimization

- Use shallow clones instead of full clones (--depth=1)
- Support for sparse checkout (only specific directories)
- Reuse and efficiently update existing clones (git pull)

### Search Optimization

- Index-based search (where possible)
- Parallel search execution
- Streaming results for large repositories

### Memory Usage

- Limit memory consumption when searching large repositories
- Result buffering and paging

## Security Considerations

MCP implements the following security measures:

### Credential Protection

- GitHub API tokens are securely stored and processed
- Tokens are not logged
- Read only from environment variables or secure storage

### Prevention of Code Execution

- Downloaded code is only analyzed, not executed
- Execution of scripts or executable files is prevented

### Sandboxing

- All operations are limited to temporary directories
- Access to parent directories is prevented

### Input Sanitization

- All user input is validated and sanitized before use
- Command injection attacks are prevented

## Implementation Requirements

### Dependent Crates

- Use the `lumin` crate for grepping local files
- Use the `tempfile` crate for temporary directory management
- Use the `gitoxide` crate for git checkout

### Reference Implementation

- Reference sources under `gitcodes-mcp/rust-sdk`
- MCP tool responses use the `CallToolResult` type rather than strings

### Concurrent Execution Management

- Multiple requests to the same repository share temporary directories
- Access to shared resources is protected by Rust's standard synchronization mechanisms

## Implementation Notes

#### Type System Design

- Git references (branches and tags) are now primarily handled with `Option<String>` in the `GitRemoteRepositoryInfo` struct, making the API more flexible
- Repository cache directories use deterministic naming based on repository owner, name, and the unique process ID to prevent conflicts
- The `RepositoryManager` structure has been enhanced with process isolation via the `process_id` field

#### Global Singleton Pattern

The codebase now uses a global singleton pattern for the `RepositoryManager`:

- The `repository_manager/instance.rs` module provides a global static instance using `once_cell::sync::OnceCell`
- Two public functions manage the instance lifecycle:
  - `init_repository_manager()`: Initializes the global instance with provided parameters
  - `get_repository_manager()`: Retrieves the global instance, creating it with defaults if needed
- This ensures all operations within the same process use the same `RepositoryManager` instance with the same `process_id`

#### Git Library Integration

- Migrated from direct git command execution to the `gix` (gitoxide) Rust library
- Implemented repository operations using the `gix` API:
  - Shallow cloning with `PrepareFetch` and `Shallow::DepthAtRemote`
  - Two-phase clone: `fetch_then_checkout` followed by `main_worktree`
  - Automatic URL format conversion from HTTPS to SSH when HTTPS clone fails
  - Improved error handling with specific guidance based on error type
  - Automatic authentication via URL modification instead of credential helpers
  - Repository reference listing with `repo.references()` iteration
- Benefits:
  - No dependency on external git commands
  - Better error handling and type safety
  - Improved performance with native Rust implementation
  - Consistent API access across all git operations

#### Repository References Listing

- Implemented in `LocalRepository::list_repository_refs()`
- Returns all references (branches and tags) in the repository as a JSON array
- Uses strongly-typed structs (`GitRefObject`, `RefObject`) for proper serialization
- Each reference includes:
  - Full reference name (e.g., `refs/heads/main`, `refs/tags/v1.0.0`)
  - SHA-1 object identifier for the referenced commit
  - Object type (typically "commit")
- Format matches the GitHub API reference listing for consistency
- Proper error handling through Result<String, String> return type
- Usage example:

```json
[
  {
    "ref": "refs/heads/main",
    "object": {
      "sha": "8f92384c20cd034bd30e96c07845f2d43d490f94",
      "type": "commit"
    }
  },
  {
    "ref": "refs/tags/v1.0.0",
    "object": {
      "sha": "7b9fc62ec24755d65c084edc4b8396dedce13e73",
      "type": "commit"
    }
  }
]
```
