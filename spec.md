# Model Context Protocol (MCP) - Library Specification

## Purpose and Overview

This MCP (Model Context Protocol) server provides a comprehensive set of tools for AI assistants to efficiently search, analyze, and reference both Rust crate documentation and external source code. The protocol enables AI to:

### Protocol Reference: [Model Context Protocol](https://modelcontextprotocol.io/introduction)

**Rust Crate Documentation Features:**
- Look up comprehensive documentation for any Rust crate from docs.rs
- Search for relevant crates on crates.io using keywords
- Get detailed API documentation for specific items (structs, traits, functions, etc.)
- Analyze type relationships and usage patterns
- Access practical code examples and implementation guidance

**GitHub Code Search Features:**
- Search for relevant repositories on GitHub with advanced filtering
- Perform detailed code analysis using pattern matching and grep
- Browse repository structure, branches, and tags
- View specific file contents with line-by-line access
- Navigate repository references and version history

**Main Use Cases:**

- Learning how to use unfamiliar Rust crates and their APIs
- Finding code examples and implementation patterns
- Investigating specific implementation methods across repositories
- Understanding library design patterns and best practices
- Analyzing differences between crate versions
- Discovering relevant crates for specific functionality

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

   - Personal access token provided via the `GITCODES_MCP_GITHUB_TOKEN` environment variable
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

Two variants of this tool are provided:

1. `grep_repository` - Returns search results in a compact format grouped by file with concatenated line contents
2. `grep_repository_match_line_number` - Returns only the total number of matching lines as a simple numeric value

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
    // Number of lines to include before each match for context (optional, default: 0)
    pub before_context: Option<usize>,
    // Number of lines to include after each match for context (optional, default: 0)
    pub after_context: Option<usize>,
    // Number of results to skip for pagination (optional, default: 0)
    pub skip: Option<usize>,
    // Maximum number of results to return (optional, default: 50)
    pub take: Option<usize>,
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

##### For grep_repository

Returns a `CompactCodeSearchResponse` with the following structure:

```rust
pub struct CompactCodeSearchResponse {
    // Total number of lines that matched the search pattern
    pub total_match_line_number: usize,
    // List of search matches grouped by file
    pub matches: Vec<CompactFileMatch>,
    // The search pattern that was used
    pub pattern: String,
    // Whether the search was case-sensitive
    pub case_sensitive: bool,
    // File extensions filter that was applied (if any)
    pub file_extensions: Option<Vec<String>>,
    // Glob patterns used to include files in the search (if any)
    pub include_globs: Option<Vec<String>>,
    // Directories or glob patterns excluded from the search (if any)
    pub exclude_globs: Option<Vec<String>>,
    // Number of lines of context included before each match
    pub before_context: Option<usize>,
    // Number of lines of context included after each match
    pub after_context: Option<usize>,
}

pub struct CompactFileMatch {
    // Path to the file containing the matches
    pub file_path: String,
    // Concatenated line contents with line numbers
    // Format: "{line_number}:{content}\n{line_number}:{content}..."
    pub lines: String,
}
```

**Example JSON Response:**
```json
{
  "total_match_line_number": 5,
  "matches": [
    {
      "file_path": "src/main.rs",
      "lines": "10:fn main() {\n11:    println!(\"Hello, world!\");"
    },
    {
      "file_path": "src/lib.rs",
      "lines": "25:pub fn main_function() -> Result<(), Error> {"
    }
  ],
  "pattern": "main",
  "case_sensitive": false,
  "include_globs": ["**/*.rs"],
  "exclude_globs": ["**/target/**"],
  "before_context": 0,
  "after_context": 1
}
```

**Key Features of Compact Format:**
- Search results are grouped by file path for better organization
- Line contents are concatenated with format `"{line_number}:{content}"`
- All search metadata is preserved (pattern, filters, context settings)
- Significantly more efficient than line-by-line JSON structures
- Both actual matches and context lines are included in the concatenated string

##### For grep_repository_match_line_number

```rust
// Returns a simple numeric value representing the total number of matching lines
Number
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

### 3. Repository File View Tool

#### Input Parameters

```rust
pub struct ViewFileParams {
    /// Relative path to the file within the repository
    /// This path can be specified with or without a leading slash
    /// Examples: "README.md", "/README.md", "src/lib.rs", "/src/lib.rs"
    pub file_path: PathBuf,
    
    /// Maximum file size to read in bytes
    /// Files larger than this will be rejected to prevent excessive memory usage
    /// If None, the default from lumin::view::ViewOptions will be used
    pub max_size: Option<usize>,
    
    /// Start viewing from this line number (1-indexed)
    /// If None, starts from the first line
    pub line_from: Option<usize>,
    
    /// End viewing at this line number (1-indexed, inclusive)
    /// If None, reads to the end of the file
    pub line_to: Option<usize>,
    
    /// Whether to display the file content without line numbers (optional, default: false)
    /// If true, line numbers will be omitted from the output
    pub without_line_numbers: Option<bool>,
}
```

#### Path Handling and Security

The `file_path` parameter in `ViewFileParams` is processed with specific security and normalization rules:

1. **Leading Slash Handling**: Paths can be specified with or without a leading slash:
   - `README.md` and `/README.md` are both valid and treated equivalently
   - The leading slash, if present, is automatically removed during normalization

2. **Directory Traversal Prevention**: Paths containing `..` (parent directory references) are 
   rejected to prevent directory traversal attacks. This applies even if the path would ultimately
   normalize to a valid location within the repository.

3. **Repository Boundary Enforcement**: All paths must resolve to a location within the repository 
   boundaries. Any attempt to access files outside the repository will be rejected.

4. **Path Normalization**: All paths are normalized to be relative to the repository root before use.

5. **Standalone Security Function**: Path security is enforced using a reusable standalone function:
   ```rust
   pub fn prevent_directory_traversal(path: &std::path::Path) -> Result<(), String> {
       // Check for directory traversal attempts and URL-encoded variants
       // Returns Err with detailed message if path is insecure
   }
   ```
   This function is used throughout the codebase to maintain consistent security validation.

6. **URL-Encoded Path Protection**: The security validation detects attempts to bypass protection
   using URL-encoded characters (e.g., `%2E%2E` for `..`), ensuring complete path security.

These measures ensure that file access is secure and that only files within the repository can be accessed.

#### Implementation Details

The `view_file_contents` function validates and normalizes the path, then uses the `lumin::view` module to:

1. Determine the file type (text, binary, or image)
2. Read the file contents with appropriate handling based on the file type
3. Return structured metadata about the file along with its contents

For text files, the function can optionally return a specific range of lines if specified in the parameters.

#### Return Value

```rust
pub enum FileContents {
    /// Text file contents with the actual content and metadata
    Text {
        /// The actual text content of the file
        content: String,
        /// Metadata about the text content
        metadata: TextMetadata,
    },

    /// Binary file representation with a descriptive message
    Binary {
        /// A descriptive message about the binary file
        message: String,
        /// Metadata about the binary file
        metadata: BinaryMetadata,
    },

    /// Image file representation with a descriptive message
    Image {
        /// A descriptive message about the image file
        message: String,
        /// Metadata about the image file
        metadata: ImageMetadata,
    },
}

pub struct TextMetadata {
    /// Number of lines in the text file
    pub line_count: usize,
    /// Number of characters in the text file
    pub char_count: usize,
}

pub struct BinaryMetadata {
    /// Whether the file is binary (always true for this struct)
    pub binary: bool,
    /// Size of the file in bytes
    pub size_bytes: u64,
    /// MIME type of the file, if it could be determined
    pub mime_type: Option<String>,
}

pub struct ImageMetadata {
    /// Whether the file is binary (always true for images)
    pub binary: bool,
    /// Size of the image file in bytes
    pub size_bytes: u64,
    /// Media type descriptor (typically "image")
    pub media_type: String,
}
```

#### Response Example

```json
{
  "success": true,
  "data": {
    "type": "text",
    "content": "# Example Repository\n\nThis is a sample README file.",
    "metadata": {
      "line_count": 3,
      "char_count": 47
    }
  },
  "metadata": {
    "execution_time_ms": 15
  }
}
```

### 4. GitHub Repository Branches/Tags List Tool

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

### Repository Location Handling

Repository locations are specified with strings that are parsed according to these rules:

1. **GitHub URLs**: Standard GitHub URLs are supported
   - HTTPS format: `https://github.com/user/repo`
   - SSH format: `git@github.com:user/repo.git`
   - Shorthand format: `github:user/repo`

2. **Local Repositories**: Both absolute and relative paths are supported
   - Absolute paths: `/path/to/repository`
   - File URLs: `file:///path/to/repository` or `file:/path/to/repository`

### GitHub URL Handling

To ensure reliable cloning of GitHub repositories, GitCodes implements a robust URL handling strategy that addresses known issues with HTTP redirects in the gitoxide library (gix):

1. **URL Conversion**
   - When cloning GitHub repositories, HTTPS URLs are automatically converted to standard SSH format
   - Example: `https://github.com/user/repo` → `git@github.com:user/repo.git`
   - This conversion happens transparently to work around gitoxide HTTP redirect handling issues
   - The original URL is preserved in repository metadata for reference

2. **URL Normalization**
   - URLs are normalized by removing `.git` suffixes, trailing slashes, and extraneous components
   - Both URL formats (with/without trailing slash) are handled uniformly
   - Example: `https://github.com/user/repo.git` and `https://github.com/user/repo/` are treated identically

3. **Fallback Mechanism**
   - If initial clone attempts fail, a multi-stage fallback mechanism tries different URL formats:
     1. SSH format (`git@github.com:user/repo.git`) as the primary method
     2. HTTPS with explicit `.git` suffix as a secondary attempt
     3. Alternate SSH format construction as a final fallback
   - This ensures maximum reliability when interacting with GitHub repositories

4. **Error Handling**
   - Detailed error messages provide guidance on URL formatting issues
   - Error messages include suggestions for alternative URL formats
   - Network, authentication, and redirect errors are distinctly handled with specific recommendations
   - Relative paths: `./repository` or `relative/path/to/repository`
   - Relative paths are automatically converted to absolute paths using the current working directory

3. **Security Validation**: All paths undergo security validation
   - Directory traversal sequences (`..`) are not allowed in any path
   - URL-encoded directory traversal attempts are detected and rejected
   - Paths are normalized for consistent processing

4. **Path Conversion Logic**: The `process_repository_location` function in the CLI handles:
   - Detecting repository location format (URL or path)
   - Converting relative paths to absolute paths
   - Performing security validation
   - Maintaining URL formats as-is

This approach maximizes usability while enforcing strong security protections.

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
- Path security is enforced through dedicated validation functions
- Relative paths are safely converted to absolute paths in the CLI
- Directory traversal attacks are prevented with rigorous path validation
- URL-encoded security bypass attempts are detected and blocked
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

## Directory Tree Generation

The library provides comprehensive directory tree generation functionality through the `get_tree_with_params` method on `LocalRepository`. This feature allows AI assistants to explore repository structure and understand project organization.

### Core Functionality

The tree generation is implemented using the `lumin` library for filesystem traversal and provides fine-grained control over the output through `TreeParams`:

```rust
pub struct TreeParams {
    pub case_sensitive: Option<bool>,
    pub search_relative_path: Option<PathBuf>,
    pub respect_gitignore: Option<bool>,
    pub depth: Option<usize>,
    pub strip_path_prefix: Option<bool>,
}
```

### Gitignore Handling

One of the most important aspects of the tree functionality is its handling of `.gitignore` files through the `respect_gitignore` parameter:

#### Default Behavior (Respecting .gitignore)

When `respect_gitignore` is `Some(true)` or `None` (default):

- **Excludes ignored files**: Files and directories listed in `.gitignore` are excluded from the tree
- **Performance optimized**: Skips scanning ignored directories, resulting in faster processing
- **Clean output**: Shows only tracked and relevant files, making it easier to understand project structure
- **Hierarchical gitignore**: Respects nested `.gitignore` files in subdirectories

```rust
let params = TreeParams {
    respect_gitignore: Some(true), // or None for default
    // ... other fields
};
// Result excludes: target/, *.log, .env, node_modules/, etc.
```

#### Complete File System View (Ignoring .gitignore)

When `respect_gitignore` is `Some(false)`:

- **Includes all files**: Shows complete filesystem structure regardless of `.gitignore` rules
- **Debug-friendly**: Useful for troubleshooting or when you need to see ignored files
- **Higher resource usage**: May result in larger trees and longer processing times
- **Build artifacts visible**: Includes compiled binaries, logs, temporary files, etc.

```rust
let params = TreeParams {
    respect_gitignore: Some(false),
    // ... other fields
};
// Result includes: target/, build/, *.log, .git/, node_modules/, etc.
```

### Use Cases and Best Practices

#### For Code Analysis (Recommended Default)
```rust
let params = TreeParams {
    case_sensitive: Some(false),
    search_relative_path: None,
    respect_gitignore: Some(true), // Clean view of source code
    depth: Some(3), // Limit depth for large projects
    strip_path_prefix: Some(true),
};
```

#### For Complete Project Investigation
```rust
let params = TreeParams {
    case_sensitive: Some(false),
    search_relative_path: None,
    respect_gitignore: Some(false), // See everything
    depth: None, // No depth limit
    strip_path_prefix: Some(true),
};
```

#### For Specific Subdirectory Analysis
```rust
let params = TreeParams {
    case_sensitive: Some(false),
    search_relative_path: Some(PathBuf::from("src")),
    respect_gitignore: Some(true),
    depth: Some(2),
    strip_path_prefix: Some(true),
};
```

### Performance Considerations

- **Gitignore Respect**: Enabling `respect_gitignore` (default) significantly improves performance by avoiding ignored directories
- **Depth Limiting**: Setting a reasonable `depth` limit prevents excessive recursion in deep directory structures
- **Relative Paths**: Using `search_relative_path` to focus on specific subdirectories reduces processing time
- **Memory Usage**: Large trees with `respect_gitignore: false` may consume significant memory

### Integration with MCP Tools

The tree functionality integrates seamlessly with other MCP tools:

1. **Repository Search**: First discover repositories with GitHub search
2. **Repository Preparation**: Clone/prepare repository with `RepositoryManager`
3. **Tree Generation**: Explore structure with `get_tree_with_params`
4. **Code Search**: Use tree insights to guide targeted code searches
5. **File Viewing**: Navigate to specific files identified in the tree

This workflow provides AI assistants with a comprehensive understanding of project structure before diving into specific code analysis.
