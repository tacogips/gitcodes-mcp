# Development Log

### Lumin 0.1.16 Glob Normalization Removal Pattern

Removed unnecessary glob path normalization logic from `LocalRepository` after upgrading to lumin 0.1.16. The lumin library now handles both `include_glob` and `exclude_glob` parameters consistently, both expecting relative paths from the search directory.

#### Problem Analysis

Previously, `LocalRepository` included a `normalize_glob_path` method that converted relative glob patterns to absolute paths for the `include_glob` parameter. This was needed for earlier versions of lumin where `include_glob` and `exclude_glob` had different path format expectations.

#### Changes Made

1. **Removed `normalize_glob_path` method** - No longer needed with lumin 0.1.16's consistent behavior
2. **Simplified `include_globs` processing** - Now simply strips leading slashes to ensure relative paths:
   ```rust
   let normalized_include_globs = options.include_globs.as_ref().map(|globs| {
       globs.iter().map(|glob| {
           glob.strip_prefix('/').unwrap_or(glob).to_string()
       }).collect::<Vec<String>>()
   });
   ```
3. **Updated documentation** - Reflected new consistent behavior in comments and tool descriptions

#### Pattern Application

- **When to apply:** After library upgrades that standardize API behavior
- **Testing approach:** Verify functionality with actual CLI commands after changes
- **Documentation update:** Update both code comments and external documentation simultaneously

#### Verification Command

The fix was verified with this test command that now works correctly:
```bash
cargo run --bin gitcodes-cli grep "https://github.com/BurntSushi/ripgrep" "struct WalkParallel|impl WalkParallel" --include "crates/ignore/src/walk.rs"
```

### GitHub Issue Search Implementation Pattern

Implemented comprehensive GitHub issue search functionality following the established repository search pattern. This pattern demonstrates how to extend the MCP tooling framework with new search capabilities while maintaining consistent architecture and strong type safety.

#### Core Components Added

1. **Issue-specific domain models** in `providers/models.rs`:
   ```rust
   pub struct IssueSearchResults {
       pub total_count: u64,
       pub incomplete_results: bool,
       pub items: Vec<IssueItem>,
   }
   
   pub struct IssueItem {
       pub id: String,
       pub number: u64,
       pub title: String,
       pub body: Option<String>,
       pub state: String,
       pub user: IssueUser,
       // ... additional fields
   }
   ```

2. **GitHub-specific issue search parameters** with sort options:
   ```rust
   pub enum GithubIssueSortOption {
       Created,
       Updated,
       Comments,
       BestMatch,
   }
   
   pub struct GithubIssueSearchParams {
       pub query: String,
       pub sort_by: Option<GithubIssueSortOption>,
       pub order: Option<GithubOrderOption>,
       pub per_page: Option<u8>,
       pub page: Option<u32>,
   }
   ```

3. **Generic issue sort options** in repository manager:
   ```rust
   pub enum IssueSortOption {
       Created,
       Updated,
       Comments,
       BestMatch,
   }
   ```

#### Implementation Pattern Application

- **Provider abstraction:** Issue search follows the same provider-agnostic pattern as repository search
- **Type conversion:** Generic `IssueSortOption` converts to provider-specific `GithubIssueSortOption`
- **Error handling:** Consistent error patterns with rate limit detection
- **Response transformation:** GitHub API responses converted to common domain models
- **MCP tool integration:** New `search_issues` tool with comprehensive parameter documentation

#### API Integration Details

The GitHub Issues Search API implementation includes:
- URL construction with proper query encoding
- Support for GitHub's search syntax (`repo:`, `state:`, `label:`, etc.)
- Pagination with per_page and page parameters
- Sorting by creation date, update date, comments, or relevance
- Authentication token support for higher rate limits

#### Pattern Benefits

- **Extensibility:** Easy to add new providers (GitLab, Bitbucket) following the same pattern
- **Type safety:** Compile-time validation of sort options and parameters
- **Consistency:** Unified interface across repository and issue search
- **Testing:** Comprehensive test coverage with parameter validation and query syntax testing

#### Usage Examples

```json
{"name": "search_issues", "arguments": {"query": "repo:rust-lang/rust state:open label:bug"}}
{"name": "search_issues", "arguments": {"query": "label:enhancement", "sort_by": "Updated", "per_page": 20}}
```

### Documentation Quality and Rustdoc Compliance Pattern

Updated project documentation to maintain high quality standards and comply with Rustdoc best practices. This pattern ensures documentation remains current, accurate, and accessible to both AI agents and human developers.

#### Rustdoc URL Formatting Pattern

When including URLs in Rustdoc comments, distinguish between actual links and example URLs:

```rust
/// # API References
/// - [GitHub API: Git References](https://docs.github.com/en/rest/git/refs?apiVersion=2022-11-28)
/// 
/// # Supported URL formats
/// - Remote URLs: `github:user/repo`, `git@github.com:user/repo.git`, `https://github.com/user/repo`
```

For actual documentation links, use proper markdown link syntax. For example URLs that show format patterns, use code formatting with backticks to avoid bare URL warnings while keeping them readable.

#### Documentation Type Safety Pattern

Use proper code formatting for types in Rustdoc comments to avoid HTML parsing issues:

```rust
/// * `repo_opt` - The `Option<LocalRepository>` to clean up if Some
```

Instead of bare type references that may be interpreted as invalid HTML tags.

#### Multi-Layer Documentation Strategy

The project maintains documentation across multiple layers:
- **Rustdoc comments**: API-level documentation accessible via `cargo doc`
- **spec.md**: Technical specifications and implementation details
- **devlog.md**: Design patterns and architectural decisions for AI agents
- **README.md**: User-facing documentation and usage instructions
- **docs/** directory: Specialized guides (e.g., glob-patterns.md)

When updating documentation, ensure consistency across all layers and update both code comments and markdown files simultaneously.

### Compact Response Format Pattern for grep_repository Tool

Implemented a new compact response format for the `grep_repository` tool that groups search results by file and concatenates line contents. This pattern provides more efficient JSON output while preserving all essential information.

#### Response Structure Design Pattern

The compact response uses a two-level grouping strategy:

```rust
pub struct CompactCodeSearchResponse {
    pub total_match_line_number: usize,
    pub matches: Vec<CompactFileMatch>,
    pub pattern: String,
    // ... other search metadata
}

pub struct CompactFileMatch {
    pub file_path: String,
    pub lines: String, // Format: "{line_number}:{content}\n{line_number}:{content}..."
}
```

#### Conversion Pattern Implementation

The conversion from verbose `CodeSearchResult` to compact format follows this pattern:

1. **Group by file path**: Use `HashMap<String, Vec<String>>` to collect lines per file
2. **Concatenate with line numbers**: Format each line as `"{line_number}:{content}"`
3. **Join with newlines**: Combine all lines for a file into a single string
4. **Preserve metadata**: Copy all original search parameters and statistics

This pattern can be applied to other tools that return line-based results for improved JSON efficiency.

#### Tool Response Evolution Pattern

When updating tool response formats:

1. **Create new response types** in `tools/responses.rs` with clear documentation
2. **Implement conversion methods** using `from_*` constructors
3. **Update tool implementation** to use new format while preserving functionality
4. **Keep legacy types** marked as `#[allow(dead_code)]` for compatibility
5. **Update all documentation** including tool descriptions, README, spec.md, and devlog.md

This approach ensures backward compatibility while providing improved efficiency for new implementations.

### Lumin 0.1.15 Upgrade: Directory Exclusion Pattern Fix

Upgraded to Lumin 0.1.15 and fixed a critical issue with directory exclusion patterns in the `exclude_dirs` functionality.

#### The Problem

The `normalize_glob_path` function was incorrectly converting relative glob patterns to absolute paths for exclude patterns. Lumin expects exclude_glob patterns to be relative to the search directory, but the normalization was producing absolute paths like:
- Input: `"core"` (directory name)
- Incorrect output: `/tmp/repo_path/**/core/**` (absolute path)
- Correct output: `**/core/**` (relative path)

This caused directory exclusion to fail completely, as lumin couldn't match the absolute patterns against its internal relative file paths.

#### The Solution

Updated the `perform_code_search` method to handle include_globs and exclude_globs differently:

```rust
// For exclude_globs, lumin expects relative paths (relative to the search directory)
// Convert directory names to glob patterns and ensure paths are relative
let normalized_exclude_globs = options.exclude_globs.as_ref().map(|dirs| {
    dirs.iter()
        .map(|dir| {
            if dir.contains('/') || dir.contains('*') {
                // Make it relative to search dir by removing repo prefix and leading slash
                let repo_path = self.repository_location.to_string_lossy();
                if dir.starts_with(repo_path.as_ref()) {
                    let relative_path = dir.strip_prefix(repo_path.as_ref()).unwrap_or(dir);
                    relative_path.strip_prefix('/').unwrap_or(relative_path).to_string()
                } else {
                    dir.strip_prefix('/').unwrap_or(dir).to_string()
                }
            } else {
                // Simple directory name to glob pattern
                format!("**/{}/**", dir)
            }
        })
        .collect()
});
```

#### Key Learning Points

1. **Lumin API Expectations**: Different lumin parameters have different path format requirements:
   - `include_glob`: Can work with absolute paths when using `omit_path_prefix`
   - `exclude_glob`: Must use relative paths relative to the search directory

2. **Glob Pattern Debugging**: When glob patterns don't work, check if the library expects relative vs absolute paths by consulting the documentation carefully

3. **Test-Driven Fixes**: The failing test `test_grep_exclude_dirs` was essential for identifying and verifying the fix

#### API Evolution for File Filtering

With improved glob support in Lumin 0.1.13, we've deprecated the `file_extensions` parameter in favor of the more flexible `include_glob` parameter. The `include_glob` approach offers several advantages:

- More precise pattern matching with full glob syntax
- Support for path-based filters, not just extensions
- Direct mapping to Lumin's internal filtering mechanisms

For backward compatibility, we continue to support `file_extensions` by automatically converting it to appropriate glob patterns. However, new code should use `include_glob` directly.

This file documents the architectural decisions and implementation patterns for the GitCodes MCP project. It's organized by pattern categories rather than chronological history to better guide future code generation.

**IMPORTANT NOTE:** This devlog contains only changes made by AI agents and may not include modifications made directly by human programmers. There may be discrepancies between the current source code and the patterns documented here.

### Tool Description Optimization Pattern

Optimized MCP tool descriptions for better readability and maintainability by reducing verbosity while preserving essential information.

#### Description Slimming Strategy

Applied consistent patterns to slim down tool descriptions:
- Reduced multiple examples to 1-2 most representative ones
- Removed redundant explanatory text 
- Condensed parameter descriptions to core functionality
- Eliminated repetitive format explanations

Example transformation for repository location parameters:
```rust
// Before (verbose)
description = "Repository URL or local file path (required) - supports GitHub formats: 'git@github.com:user/repo.git' (SSH format, most reliable), 'https://github.com/user/repo' (HTTPS format with automatic fallback to SSH), 'github:user/repo', or local paths like '/path/to/repo'. SSH URL format is recommended for the most reliable git operations..."

// After (concise)  
description = "Repository URL or local path. Supports GitHub formats: 'git@github.com:user/repo.git' (SSH, recommended), 'https://github.com/user/repo', 'github:user/repo', or absolute local paths. Private repos require GITCODES_MCP_GITHUB_TOKEN environment variable."
```

#### Consistent Parameter Description Patterns

Established standard formats for common parameter types:
- Boolean options: "Description (optional, default value)"
- Numeric limits: "Description (optional, default X, max Y)" 
- File patterns: "Description (optional). Example: [\"pattern1\", \"pattern2\"]"
- Context parameters: "Lines of context before/after each match (optional, default 0)"

This optimization improves tool usability while maintaining all necessary functionality information.

### Directory Tree Generation and Gitignore Handling

Implemented comprehensive directory tree functionality with sophisticated gitignore handling through the `get_tree_with_params` method on `LocalRepository`.

#### TreeParams Pattern for Flexible Configuration

The tree generation uses a parameter object pattern for clean, extensible configuration:

```rust
pub struct TreeParams {
    pub case_sensitive: Option<bool>,
    pub search_relative_path: Option<PathBuf>,
    pub respect_gitignore: Option<bool>,
    pub depth: Option<usize>,
    pub strip_path_prefix: Option<bool>,
}
```

All fields are `Option<T>` to allow for defaults, following the "null object" pattern where `None` represents sensible defaults.

#### Gitignore Behavior Design

The `respect_gitignore` parameter implements a critical design decision:

- **Default (`None` or `Some(true)`)**: Respects `.gitignore` files for clean, performance-optimized trees
- **Debug mode (`Some(false)`)**: Includes all files for complete filesystem visibility

This dual-mode approach balances common use cases (clean source code view) with debugging needs (complete file visibility).

#### Path Manipulation Pattern

The implementation demonstrates proper `PathBuf` usage:

```rust
// WRONG: String concatenation
let tree_root_path = self.repository_location + relative_path;

// CORRECT: PathBuf join method
let tree_root_path = self.repository_location.join(relative_path);
```

The pattern extracts `search_relative_path` before converting `TreeParams` to `lumin::TreeOptions` because the underlying library doesn't support this field directly.

#### Test Environment Independence

Tree tests demonstrate environment-independent testing by using dynamic repository cloning instead of local file references:

```rust
// PROBLEMATIC: Direct local dependency
fn get_test_repository() -> LocalRepository {
    let repo_path = PathBuf::from(".private.deps-src/gitcodes-mcp-test-1");
    LocalRepository::new(repo_path)
}

// SOLUTION: Dynamic cloning with RepositoryManager
async fn get_test_repository() -> LocalRepository {
    let manager = create_test_manager();
    let repo_location = RepositoryLocation::from_str(TEST_REPO_URL)?;
    manager.prepare_repository(&repo_location, None).await?
}
```

This pattern ensures tests work in CI/CD environments and other developer machines where local dependencies aren't available.

#### Performance Optimization Patterns

Tree generation implements several performance optimizations:

- **Gitignore respect**: Default behavior skips ignored directories for faster processing
- **Depth limiting**: Optional `depth` parameter prevents excessive recursion
- **Path focusing**: `search_relative_path` allows targeted subdirectory analysis

These patterns reduce memory usage and processing time for large repositories.

### Robust URL Handling for GitHub Repositories

- Enhanced URL normalization for GitHub repositories to handle HTTPS URL inconsistencies
- Implemented fallback strategy for repository cloning (HTTPS → SSH)
- Addressed gitoxide HTTP transport issues with GitHub URL redirects
- Added comprehensive error messages with specific remediation actions for different error types
- Fixed HTTPS URL handling by enabling HTTP redirect following in gix transport

#### Repository URL Preprocessing Pattern

- Normalized GitHub URLs by consistently removing `.git` suffix which causes redirect issues
- Added support for different URL formats (with/without trailing slash)
- Ensured proper path normalization by removing any extra leading slashes
- Properly handled authentication tokens in URLs with secure logging (redaction of sensitive information)
- Configured redirect following in the gix transport layer to solve GitHub redirect issues (see https://github.com/GitoxideLabs/gitoxide/issues/974)

#### HTTP Transport Configuration Pattern

- Used `with_in_memory_config_overrides(["http.followRedirects=true"])` to enable redirect following
- Applied configuration before attempting to fetch, enabling proper handling of GitHub redirects
- Reference implementation in `gix-transport/src/client/blocking_io/http/reqwest/remote.rs`

### Native Git Operations Using Gitoxide (gix)

- Replaced shell command execution with native Rust library calls using the `gix` library
- Implemented robust error handling with proper error propagation patterns
- Used typed repository references instead of string handling for better type safety

#### Repository Fetching Implementation

- Implemented `fetch_remote` method using the native `gix` library instead of shelling out to git
- Used `find_fetch_remote` to get a properly configured remote for fetching
- Used a step-by-step approach to connect, prepare, and then execute the fetch operation
- Maintained fetch from all configured remotes, continuing on individual failures
- Structured error handling to provide meaningful context about which stage of the fetch failed
- Integrated fetch operation into `services::list_repository_refs` function to ensure local repositories are updated before listing references

#### Repository Cloning Error Handling Improvements

- Improved HTTPS URL normalization to properly handle GitHub URLs with and without .git suffix
- Implemented explicit URL preprocessing for HTTPS GitHub URLs to avoid transport errors
- Enhanced HTTPS to SSH URL conversion as a reliable fallback mechanism
- Improved error logging by capturing and logging detailed error information from gitoxide
- Added more specific error detection and handling for "IO error occurred when talking to the server" errors
- Updated CLI error messages to provide more targeted guidance based on error signatures
- Documentation now recommends using 'https://github.com/user/repo' format without .git suffix for HTTPS URLs
- Implemented robust tests that verify fetch operation in both direct usage and service integration scenarios

```rust
// Example pattern for fetching a remote using gix
let remote_result = repo.find_fetch_remote(Some(&*remote_name));
match remote_result {
    Ok(remote) => {
        // Connect to the remote for fetching
        match remote.connect(gix::remote::Direction::Fetch) {
            Ok(connection) => {
                // Prepare the fetch operation
                match connection.prepare_fetch(&mut progress, Default::default()) {
                    Ok(prepare) => {
                        // Execute the fetch operation
                        match prepare.receive(&mut progress, &gix::interrupt::IS_INTERRUPTED) {
                            Ok(_outcome) => {
                                // Fetch successful
                            },
                            Err(e) => {
                                // Handle fetch execution error
                            }
                        }
                    },
                    Err(e) => {
                        // Handle fetch preparation error
                    }
                }
            },
            Err(e) => {
                // Handle connection error
            }
        }
    },
    Err(e) => {
        // Handle remote initialization error
    }
}
```

### Repository Reference Listing with Native Git Integration

- Implemented `list_repository_refs` function using the `gix` Rust library instead of shell commands
- Returns a Result with either JSON data or error message
- Uses strongly-typed structs (GitRefObject, RefObject) for serialization instead of raw JSON construction
- References include both name and SHA hash for proper version identification
- Follows idiomatic Rust error handling instead of encoded errors in JSON
- Matches GitHub API format for consistency between local and remote repositories

```rust
// Example implementation using gix to list repository references with strongly-typed structs
pub async fn list_repository_refs(&self, _repository_location: &RepositoryLocation) -> Result<String, String> {
    // Define structs for reference objects
    #[derive(Serialize)]
    struct RefObject {
        sha: String,
        #[serde(rename = "type")]
        object_type: &'static str,
    }
    
    #[derive(Serialize)]
    struct GitRefObject {
        #[serde(rename = "ref")]
        ref_name: String,
        object: RefObject,
    }
    
    // Open the repository
    let repo = gix::open(&self.repository_location)
        .map_err(|e| format!("Failed to open repository: {}", e))?;
    
    let refs_platform = repo.references()
        .map_err(|e| format!("Failed to access repository references: {}", e))?;
        
    let all_refs = refs_platform.all()
        .map_err(|e| format!("Failed to list references: {}", e))?;
        
    // Process references into structured objects
    let mut result = Vec::new();
    for reference in all_refs {
        if let Ok(r) = reference {
            let ref_name = r.name().as_bstr().to_string();
            let sha = r.target().id().to_hex().to_string();
            
            // Use properly structured types
            let ref_obj = GitRefObject {
                ref_name,
                object: RefObject {
                    sha,
                    object_type: "commit",
                },
            };
            
            result.push(ref_obj);
        }
    }
    
    // Serialize to JSON
    serde_json::to_string(&result)
        .map_err(|e| format!("Failed to serialize references: {}", e))
}
```

## Type System Patterns

### Pagination Defaults at Service Layer

- **Pattern:** Apply sensible default values for pagination parameters at the service layer rather than at the API boundary
- **Rationale:** Ensures consistent behavior across different tool interfaces while maintaining flexibility for explicit control
- **Implementation:**
  ```rust
  // In service layer functions, apply defaults using Option::or()
  let search_params = CodeSearchParams {
      // ... other fields
      skip: params.skip, // Allow explicit None for no skipping
      take: params.take.or(Some(50)), // Default to 50 if not specified
  };
  ```
- **Benefits:**
  - Prevents unbounded result sets that could overwhelm system resources
  - Provides predictable behavior for MCP tools
  - Allows explicit override when needed (pass Some(value) or None)
  - Documents expected usage patterns through reasonable defaults

### Structured Return Types over JSON Strings

- **Pattern:** Use structured types with proper serialization support instead of returning JSON strings
- **Example:** Changed `search_code` to return a proper `CodeSearchResult` struct instead of a JSON string
- **Implementation:**
  ```rust
  // Instead of:
  pub async fn perform_code_search(...) -> Result<String, String> {
      // Process search and format as JSON string
      let json_results = json!({
          "matches": all_results,
          "pattern": pattern,
          // ... other fields
      });
      to_string_pretty(&json_results)
  }
  
  // Use structured type:
  pub async fn perform_code_search(...) -> Result<CodeSearchResult, String> {
      // Process search
      Ok(CodeSearchResult::new(
          all_results,
          pattern,
          repo_path,
          case_sensitive,
          file_extensions,
          exclude_dirs
      ))
  }
  
  // With a helper method for backward compatibility if needed:
  impl CodeSearchResult {
      pub fn to_json(&self) -> Result<String, String> {
          serde_json::to_string_pretty(self)
              .map_err(|e| format!("Failed to convert search results to JSON: {}", e))
      }
  }
  ```
- **Benefits:** Type safety, easier to use in consuming code, better abstraction, better documentation
- **When to apply:** When returning complex structured data that might be processed further before serialization

#### MCP Tool Response Type Implementation

- **Pattern Extension:** Applied structured return types to all MCP tool methods
- **Example:** Changed all tool methods in `GitHubCodeTools` to return `Result<CallToolResult, McpError>` but with strong internal typing
- **Implementation:**
  ```rust
  // BEFORE: Tool method returning String or Result<String, String>
  async fn search_repositories(
      &self,
      query: String,
      // Other parameters...
  ) -> String {
      // Process and return JSON string
  }

  // AFTER: Tool method returning Result<CallToolResult, McpError> with internal typing
  async fn search_repositories(
      &self,
      query: String,
      // Other parameters...
  ) -> Result<CallToolResult, McpError> {
      // Internal processing with strong typing
      match self.manager.search_repositories(...).await {
          Ok(json_result) => {
              // Validate with strongly-typed struct
              match serde_json::from_str::<RepositorySearchResponse>(&json_result) {
                  Ok(_) => Self::success_result(json_result),
                  Err(e) => Self::error_result(format!("Failed to parse: {}", e))
              }
          },
          Err(err) => Self::error_result(format!("Operation failed: {}", err))
      }
  }
  ```
- **Module Organization:** 
  1. Created a dedicated `responses.rs` module that defines all structured response types
  2. Added helper methods to ensure consistent formatting for MCP protocol
  ```rust
  // In responses.rs
  pub struct RepositorySearchResponse {
      pub total_count: u64,
      pub incomplete_results: bool,
      pub items: Vec<RepositoryItem>,
  }
  
  // In tools/mod.rs
  impl GitHubCodeTools {
      /// Helper method to create a CallToolResult for successful responses
      fn success_result(json: String) -> Result<CallToolResult, McpError> {
          Ok(CallToolResult::success(vec![Content::text(json)]))
      }
      
      /// Helper method to create a CallToolResult for error responses
      fn error_result(message: impl Into<String>) -> Result<CallToolResult, McpError> {
          let error_message = message.into();
          Ok(CallToolResult::error(vec![Content::text(error_message)]))
      }
  }
  ```
- **Benefits:** 
  1. Type safety: The exact structure of responses is defined at compile time
  2. Better documentation: Response structures are self-documenting
  3. Protocol compatibility: All tools return `Result<CallToolResult, McpError>` for MCP compatibility
  4. Consistent formatting: Helper methods ensure all responses and errors are formatted consistently
- **When to apply:** When integrating strongly-typed internal APIs with external frameworks that have their own protocol requirements

### Use Native Types Over Single-Field Wrappers

- **Pattern:** Replace wrapper types with native Rust types when they don't add significant behavior
- **Example:** Replaced `GitRef` struct with direct `String` usage
- **Implementation:**
  ```rust
  // Instead of:
  pub struct GitRef { pub name: String }
  async fn update_repository(&self, git_ref: &GitRef) -> Result<(), String> { ... }
  
  // Use directly:
  async fn update_repository(&self, ref_name: &str) -> Result<(), String> { ... }
  ```
- **When to apply:** When a wrapper type doesn't add validation, behavior, or type safety

### Parameter Grouping With Structured Types

- **Pattern:** Group related parameters into a structured type
- **Example:** `RemoteGitRepositoryInfo` for repository cloning parameters
- **Implementation:**
  ```rust
  #[derive(Debug, Clone, schemars::JsonSchema, serde::Serialize, serde::Deserialize)]
  pub struct RemoteGitRepositoryInfo {
      pub user: String,
      pub repo: String,
      pub ref_name: String,
  }
  
  async fn clone_repository(repo_dir: &Path, params: &RemoteGitRepositoryInfo) -> Result<(), String> { ... }
  ```
- **When to apply:** When multiple parameters are often passed together

### Type-Safe Variants With Enums

- **Pattern:** Use enums to represent different variants with distinct behaviors
- **Example:** `RepositoryLocation` for GitHub URLs vs local paths
- **Implementation:**
  ```rust
  pub enum RepositoryLocation {
      GitHubUrl(String),
      LocalPath(PathBuf),
  }
  
  impl FromStr for RepositoryLocation { ... }
  ```
- **When to apply:** When a parameter can have multiple distinct types with different behaviors

### Descriptive Field Naming

- **Pattern:** Field names should reflect both type and purpose
- **Example:** Renamed `repository` to `repository_location` for clarity
- **When to apply:** When using enum types as fields, include the enum name in the field name
- **Apply consistently in:** Struct definitions, function parameters, API documentation

## Architecture Patterns

### Deterministic Resource Naming with Process Isolation

- **Pattern:** Generate consistent resource identifiers with process-specific differentiation
- **Example:** Repository cache directory naming using hash of repository info and process ID
- **Implementation:**
  ```rust
  // In RepositoryManager
  fn generate_process_id() -> String {
      use std::process;
      use uuid::Uuid;
      
      let pid = process::id();
      let uuid = Uuid::new_v4();
      
      format!("{}_{}", pid, uuid.simple())
  }
  
  // In LocalRepository
  fn generate_repository_hash(info: &GitRemoteRepositoryInfo) -> String {
      use std::collections::hash_map::DefaultHasher;
      use std::hash::{Hash, Hasher};
      
      let repo_key = format!("{}/{}", info.user, info.repo);
      let mut hasher = DefaultHasher::new();
      repo_key.hash(&mut hasher);
      let hash_value = hasher.finish();
      
      format!("{:x}", hash_value)[0..12].to_string()
  }
  
  pub fn new_local_repository_to_clone(
      remote_repository_info: GitRemoteRepositoryInfo, 
      process_id: Option<&str>
  ) -> Self {
      let hash_value = Self::generate_repository_hash(&remote_repository_info);
      
      // Include process_id in directory name if provided
      let dir_name = if let Some(pid) = process_id {
          format!(
              "mcp_gitcodes_{}_{}_{}_pid{}",
              remote_repository_info.user, remote_repository_info.repo, hash_value, pid
          )
      } else {
          format!(
              "mcp_gitcodes_{}_{}_{}" ,
              remote_repository_info.user, remote_repository_info.repo, hash_value
          )
      };

      let mut repo_dir = std::env::temp_dir();
      repo_dir.push(dir_name);

      Self::new(repo_dir)
  }
  ```
- **Benefits:** 
  - Predictable caching for deterministic behavior
  - Process isolation to prevent conflicts in concurrent environments
  - Consistent paths across runs for the same repository
  - Uniqueness ensured by combination of deterministic hashing and process-specific IDs

### Separation of Concerns

- **Pattern:** Separate data handling from presentation
- **Example:** Low-level modules return raw data; formatting done in API layers
- **Implementation:**
  ```rust
  // Service method returns raw data:
  pub async fn grep_repository(&self, params: GrepParams) -> Result<GrepResult, String> { ... }
  
  // API layer handles formatting:
  match service.grep_repository(params).await {
      Ok(result) => format_for_display(result),
      Err(error) => format!("Search failed: {}", error),
  }
  ```
- **Application:** Service methods, API controllers, data transformations

### Explicit Error Handling

- **Pattern:** Use Result types for operations that can fail
- **Implementation:**
  ```rust
  pub async fn operation_name(&self) -> Result<SuccessType, ErrorType> {
      match potentially_failing_operation() {
          Ok(value) => Ok(value),
          Err(e) => Err(format!("Descriptive error: {}", e)),
      }
  }
  ```
- **When to apply:** All functions that can fail should return Result

## Module Organization Patterns

### Feature-Based Module Structure

- **Pattern:** Organize code by feature rather than technical layer
- **Example:** Dedicated modules for GitHub API, Git repository operations, code search
- **Implementation:**
  ```
  gitcodes/
   ├── github/
   │   ├── api.rs           # GitHub API operations
   │   ├── params.rs         # API parameter structures
   │   └── mod.rs
   ├── repository/
   │   ├── manager.rs        # Repository management
   │   ├── location.rs       # Repository location types
   │   └── mod.rs
   └── mod.rs
  ```
- **Benefits:** Improves discoverability, keeps related code together

### Component Encapsulation

- **Pattern:** Hide implementation details behind public interfaces
- **Example:** Making helper functions module-private with explicit public APIs
- **Implementation:**
  ```rust
  // Private implementation:
  fn parse_repository_url(url: &str) -> Result<(String, String), String> { ... }
  
  // Public interface:
  pub fn prepare_repository(url: &str) -> Result<RepositoryInfo, String> {
      let (user, repo) = parse_repository_url(url)?;
      // ...
  }
  ```
- **When to apply:** For internal helper functions, implementation details

### Method Refactoring

- **Pattern:** Convert standalone functions to methods on relevant structs
- **Example:** Moving repository utility functions to `RepositoryManager` methods
- **Implementation:**
  ```rust
  // Instead of:
  fn get_repo_dir(manager: &RepositoryManager, info: &RepositoryInfo) -> PathBuf { ... }
  
  // Use:
  impl RepositoryManager {
      fn get_repo_dir(&self, info: &RepositoryInfo) -> PathBuf { ... }
  }
  ```

#### Moving Service Functions to Their Logical Owner

- **Pattern:** Migrate service functions to methods on the types they primarily operate on
- **Example:** Moving `list_repository_refs` from services module to `RepositoryManager`
- **Implementation:**
  ```rust
  // Before: Function in services module
  pub async fn list_repository_refs(
      repository_manager: &repository_manager::RepositoryManager,
      repository_location_str: &str,
  ) -> Result<(String, Option<LocalRepository>), String> {
      // Implementation...
  }

  // After: Method on RepositoryManager
  impl RepositoryManager {
      pub async fn list_repository_refs(
          &self,
          repository_location_str: &str,
      ) -> Result<(String, Option<LocalRepository>), String> {
          // Implementation...
      }
  }
  ```

- **Benefits:**
  1. Places methods with the data they operate on
  2. Reduces the need to pass the repository manager as a parameter
  3. Creates a more intuitive API for consumers
  4. Clearly identifies the logical owner of repository-related operations
- **Benefits:** Better encapsulation, clearer ownership, simplified signatures

## API Design Patterns

### Authentication Options Hierarchy

- **Pattern:** Implement a clear priority order for configuration options
- **Example:** GitHub token authentication precedence
- **Implementation:**
  1. Command line argument (highest priority)
  2. Programmatic parameter (second priority)
  3. Environment variable (fallback)
- **Benefits:** Consistent predictable behavior, flexibility for users

### Consistent Parameter Types

- **Pattern:** Use consistent parameter types across related APIs
- **Example:** Structured parameter types for search and grep operations
- **Implementation:**
  ```rust
  // Related operation parameters use similar structure:
  pub struct SearchParams { /* ... */ }
  pub struct GrepParams { /* ... */ }
  
  // API methods have consistent signatures:
  pub async fn search_repositories(&self, params: SearchParams) -> Result<...> { ... }
  pub async fn grep_repository(&self, params: GrepParams) -> Result<...> { ... }
  ```
- **Benefits:** Easier to learn API, consistent usage patterns

### Wrapper Pattern For Tool Integration

## Testing Patterns

### Dynamic Test Assertion Patterns

- **Pattern:** Make tests adaptable to different test repositories by finding relevant test data dynamically  
- **Example:** In directory exclusion tests, we first identify which directories actually contain matches
- **Implementation:**
  ```rust
  // Instead of hardcoding a directory to exclude:
  let api_match_count = matches.iter().filter(|m| {
      file_path.contains("/api/")
  }).count();
  
  // Dynamically find directories with matches:
  let mut dirs_with_matches = std::collections::HashSet::new();
  for match_item in matches_no_exclusion {
      let file_path = match_item["file_path"].as_str().unwrap();
      if let Some(parent) = std::path::Path::new(file_path).parent() {
          if let Some(dir_name) = parent.file_name() {
              if let Some(dir_str) = dir_name.to_str() {
                  dirs_with_matches.insert(dir_str.to_string());
              }
          }
      }
  }
  
  // Use first available directory for the test
  let dir_to_exclude = dirs_with_matches.iter().next().unwrap().clone();
  ```
- **When to apply:** When tests need to work across different test repositories or with varying data

### Search Functionality Testing

- **Pattern:** Test different aspects of search with specifically tailored test cases
- **Example:** For code search, we test basic pattern matching, case sensitivity, file extension filtering, directory exclusion, and regex patterns separately
- **Implementation:**
  ```rust
  // Basic search test
  #[tokio::test]
  async fn test_search_code_basic_pattern() { ... }
  
  // Case sensitivity test
  #[tokio::test]
  async fn test_search_code_case_sensitive() { ... }
  
  // File extension filtering test
  #[tokio::test]
  async fn test_search_code_file_extension_filter() { ... }
  ```
- **When to apply:** When testing complex functionality with multiple features that need separate verification

### Path Pattern Improvements

- **Pattern:** Use more robust directory exclusion patterns with leading wildcards
- **Example:** Changed directory exclusion pattern format from `{dir}/**` to `**/{dir}/**`
- **Implementation:**
  ```rust
  // Original pattern - only works for top-level directories
  exclude_glob: dirs.iter().map(|dir| format!("{}/**", dir)).collect()
  
  // Improved pattern - works for directories at any level
  exclude_glob: dirs.iter().map(|dir| format!("**/{}/**", dir)).collect()
  ```
- **When to apply:** When working with glob patterns that need to match at any level of directory hierarchy

- **Pattern:** Use wrapper classes to separate core functionality from tool integration
- **Example:** `GitHubCodeTools` wrapper for MCP integration around `GitHubService`
- **Implementation:**
  ```rust
  // Core service with business logic:
  pub struct GitHubService { /* ... */ }
  
  // Wrapper with tool annotations:
  #[derive(tool::ToolBox)]
  pub struct GitHubCodeTools {
      service: GitHubService,
  }
  
  impl GitHubCodeTools {
      #[tool(description = "...")]
      pub async fn search_repositories(&self, params: SearchParams) -> CallToolResult {
          match self.service.search_repositories(params).await {
              Ok(result) => CallToolResult::success(result),
              Err(error) => CallToolResult::error(error),
          }
      }
  }
  ```
- **Benefits:** Clean separation of concerns, core services remain independent of tool framework

## Data Handling Patterns

### Path Type Safety

- **Pattern:** Use specialized path types instead of strings
- **Example:** Replaced string paths with `Path` and `PathBuf`
- **Implementation:**
  ```rust
  // Instead of:
  fn get_repo_dir(base_dir: &str, repo: &str) -> String { ... }
  
  // Use:
  fn get_repo_dir(base_dir: &Path, repo: &str) -> PathBuf {
      base_dir.join(format!("mcp_repo_{}", repo))
  }
  ```
- **Benefits:** Type safety, better path manipulation, clearer semantics

### Enum String Conversion

- **Pattern:** Add methods for consistent enum-to-string conversions
- **Example:** `to_str()` methods on enumeration types
- **Implementation:**
  ```rust
  pub enum SortOption {
      Relevance,
      Stars,
      Forks,
      Updated,
  }
  
  impl SortOption {
      pub fn to_str(&self) -> &str {
          match self {
              SortOption::Relevance => "",
              SortOption::Stars => "stars",
              SortOption::Forks => "forks",
              SortOption::Updated => "updated",
          }
      }
  }
  ```

### Strong Type Safety with Domain-Specific Enums

- **Pattern:** Use domain-specific enums rather than strings for type safety
- **Example:** Moved `SortOption` and `OrderOption` enums from tools module to repository_manager module
- **Implementation:**
  ```rust
  // In repository_manager module where these are primarily used
  pub enum SortOption {
      Relevance,
      Stars,
      Forks,
      Updated,
  }
  
  pub enum OrderOption {
      Ascending,
      Descending,
  }
  
  // Implement conversion from generic SortOption to provider-specific options
  impl From<SortOption> for providers::github::GithubSortOption {
      fn from(value: SortOption) -> Self {
          match value {
              SortOption::Relevance => Self::Relevance,
              SortOption::Stars => Self::Stars,
              SortOption::Forks => Self::Forks,
              SortOption::Updated => Self::Updated,
          }
      }
  }
  
  // In repository_manager's search_repositories method, accept enums directly
  pub async fn search_repositories(
      &self,
      provider: providers::GitProvider,
      query: String,
      sort_option: Option<SortOption>,  // Generic sort option
      order_option: Option<OrderOption>, // Generic order option
      per_page: Option<u8>,
      page: Option<u32>,
  ) -> Result<String, String> {
      // Convert generic enums to provider-specific ones
      let sort_by = sort_option.map(providers::github::GithubSortOption::from);
      // ...
  }
  ```
- **Benefits:** Prevents invalid input at compile time, eliminates runtime string conversion, improves type safety, and enables IDE autocompletion
- **Benefits:** Centralized conversion logic, consistent string representations

## Implementation Notes

### Enhanced Code Search with Context Lines

The code search functionality has been improved to provide better context around matches, making search results more useful for understanding the code:

```rust
pub struct CodeSearchParams {
    // ... existing fields ...
    
    /// Number of lines to include before each match (default: 0)
    pub before_context: Option<usize>,

    /// Number of lines to include after each match (default: 0)
    pub after_context: Option<usize>,
}
```

These new parameters allow users to specify how many lines of context to include before and after each match, which is especially helpful when trying to understand the surrounding code structure. For example, when searching for a function call, seeing the surrounding lines can provide insight into how the function is being used.

The implementation uses the native capabilities of the underlying search engine (ripgrep via lumin) to efficiently retrieve context lines without re-reading files.

### File Viewing Enhancements

The file viewing functionality has been enhanced to support an option for displaying file contents without line numbers:

```rust
pub async fn show_file_contents(
    // ... existing parameters ...
    without_line_numbers: Option<bool>,
) -> Result<
    (
        lumin::view::FileContents,
        crate::gitcodes::local_repository::LocalRepository,
        bool, // Effective use_line_numbers value
    ),
    String,
>
```

This enhancement makes it possible to get clean file contents without line numbers, which can be useful for copying code snippets or when line numbers would interfere with further processing of the content.

### CLI Tracing Configuration Improvements

Enhanced the logging configuration for the CLI tool to provide a better user experience:

1. **Verbose-Only Logging**
   - Tracing logs are now only shown when the `--verbose` flag is used
   - Without the flag, only the actual command output is displayed
   - This makes the default output much cleaner for regular use

2. **Debug Log Level**
   - Added proper separation between `--verbose` (INFO level) and `--debug` (DEBUG level) 
   - Debug-specific details are only shown when explicitly requested

3. **Clean Log Format**
   - Removed thread IDs from log output
   - Removed file names and line numbers
   - Replaced ISO timestamps with simple elapsed time
   - Makes logs more compact and easier to read

### HTTPS to SSH URL Fallback for Git Clone Operations

To solve the gitoxide HTTP redirect handling issues with GitHub repositories, we implemented a URL conversion pattern that automatically transforms HTTPS URLs to SSH format (`git@github.com:user/repo.git`) for better reliability with the gitoxide library. The implementation includes:

1. **Adding Converter Methods to Repository Types**:
   ```rust
   impl GithubRemoteInfo {
       /// Converts the repository URL to SSH format to avoid HTTPS URL handling issues with gitoxide
       pub fn to_ssh_url(&self) -> String {
           format!("git@github.com:{}/{}.git", self.repo_info.user, self.repo_info.repo)
       }
   }
   
   impl GitRemoteRepository {
       /// Converts the repository URL to SSH format
       pub fn to_ssh_url(&self) -> String {
           match self {
               GitRemoteRepository::Github(github_info) => github_info.to_ssh_url(),
           }
       }
   }
   ```

2. **Proactively Using SSH URLs for GitHub HTTPS Repositories**:
   ```rust
   // For GitHub repositories with HTTPS URLs, use SSH format to avoid HTTP redirect issues with gitoxide
   let clone_url = if remote_repository.clone_url().starts_with("https://github.com") {
       // Use SSH URL format for GitHub HTTPS URLs to avoid redirect issues
       let original_url = remote_repository.clone_url();
       let ssh_url = remote_repository.to_ssh_url();
       tracing::info!("Converting GitHub HTTPS URL '{}' to SSH format '{}'", original_url, ssh_url);
       ssh_url
   } else {
       // For non-GitHub or already SSH URLs, use the original URL
       remote_repository.clone_url()
   };
   ```

3. **Multi-Stage Fallback Mechanism**:
   If the primary clone attempt fails, we've implemented a cascading fallback pattern that tries multiple URL formats:
   - First attempt: SSH URL format (using the to_ssh_url method)
   - Second attempt: HTTPS URL with explicit .git suffix
   - Third attempt: Another SSH URL format (direct construction)
   
   This ensures maximum reliability when working with GitHub repositories while still preserving the user's original URL format in the repository information.
   
4. **Comprehensive URL Normalization**:
   ```rust
   let user_repo = if url.starts_with("https://github.com") {
       // Handle both with and without trailing slash
       if url.starts_with("https://github.com/") {
           url.trim_start_matches("https://github.com/")
       } else {
           url.trim_start_matches("https://github.com")
       }
       .trim_start_matches('/')
       .trim_end_matches('/')
       .trim_end_matches(".git")
       .to_string()
   }
   ```

This approach solves the gitoxide HTTPS URL handling issue described in https://github.com/GitoxideLabs/gitoxide/issues/974 without requiring changes to the underlying gitoxide library.

```rust
// If HTTPS URL fails, try converting to SSH URL format as a fallback
if fetch_result.is_err() && clone_url.starts_with("https://github.com") {
    tracing::info!(
        "HTTPS clone failed, attempting fallback to SSH URL format"
    );
    
    // Convert https://github.com/user/repo to git@github.com:user/repo.git
```

### Specialized Repository Search Tools

Added a specialized variant of the `grep_repository` tool called `grep_repository_match_line_number` that returns only the total count of matching lines. This tool uses the same search mechanism but provides a more lightweight response when only the count is needed.

The implementation leverages the new `total_match_line_number` field in the `CodeSearchResult` struct, which stores the total number of matching lines found during a search operation. This field is useful for pagination and for determining the total size of a result set without processing all matches.

```rust
pub struct CodeSearchResult {
    /// Total number of matching lines found
    pub total_match_line_number: usize,
    
    /// List of search matches found
    pub matches: Vec<LuminSearchResultLine>,
    // ... other fields
}
```

The tool implementation extracts this count and returns it as a simple JSON number:

```rust
// Inside grep_repository_match_line_number implementation
match serde_json::to_string(&result.total_match_line_number) {
    Ok(json) => success_result(json),
    Err(e) => error_result(format!("Failed to serialize match count: {}", e)),
}
```

This approach provides a more efficient way to get just the count of matches without transferring all the match data, useful for applications that need to know the total number of matches before deciding whether to retrieve the full results.

### Enhanced GitHub URL Handling and Cloning

Improved GitHub URL handling to ensure that HTTPS URLs consistently include the `.git` suffix, which significantly improves cloning reliability.

```rust
// Generate proper clone URL with .git suffix for GitHub URLs
let clone_url = if url.starts_with("https://github.com/") {
    if url.ends_with(".git") {
        url.to_string() // Already has .git suffix
    } else {
        format!("https://github.com/{}/{}.git", user_clone, repo_clone) // Add .git suffix
    }
} else if url.starts_with("github:") {
    // Convert github:user/repo to https://github.com/user/repo.git
    format!("https://github.com/{}/{}.git", user_clone, repo_clone)
} else {
    url.to_string() // Keep original URL for other formats
};
```

### Enhanced Multi-Layered Repository Cloning

Built on the HTTPS to SSH fallback by implementing a full multi-layered approach to git repository cloning with three distinct layers of fallback mechanisms:

1. First attempt uses the `gix` library with the provided URL (typically HTTPS)
2. If that fails and it's a GitHub URL, attempt with SSH URL format through `gix`
3. As a final fallback, use the system `git` command directly via process execution

This approach significantly improves reliability across different network configurations and environments.

```rust
// If HTTPS URL fails, try alternative approaches
if fetch_result.is_err() && clone_url.starts_with("https://github.com") {
    // First attempt: Try with SSH URL format
    tracing::info!("HTTPS clone failed, attempting fallback to SSH URL format");
    
    // Convert https://github.com/user/repo to git@github.com:user/repo.git
    let github_path = clone_url.trim_start_matches("https://github.com/");
    let ssh_url = format!("git@github.com:{}.git", github_path);
    
    // Try again with SSH URL through gix library
    fetch_result = PrepareFetch::new(
        ssh_url.as_str(),
        repo_dir.clone(),
        Kind::WithWorktree,
        gix::create::Options::default(),
        OpenOptions::default(),
    );
    
    // If SSH also fails, try with system git command as a last resort
    if fetch_result.is_err() {
        tracing::info!("SSH URL also failed, attempting with system git command");
        
        // Try with system git command
        let output = std::process::Command::new("git")
            .arg("clone")
            .arg(&clone_url)
            .arg(repo_dir.as_os_str())
            .output();
            
        // Handle the result of system git command
        if let Ok(output) = output {
            if output.status.success() {
                tracing::info!("Successfully cloned using system git command");
                return Ok(local_repo);
            }
        }
    }
}
```

### Diagnostic Tools for Git Operations

Added a dedicated diagnostic tool `git_diagnostic.rs` to help diagnose and troubleshoot git-related issues. This tool:

1. Attempts to clone repositories using both HTTPS and SSH URLs
2. Tests repository checkout operations
3. Validates git configuration (proxy settings, SSH keys)
4. Provides detailed output for diagnosing connectivity issues

The diagnostic tool is particularly useful for environments with network restrictions or non-standard git configurations.
    let github_path = clone_url.trim_start_matches("https://github.com/");
    let ssh_url = format!("git@github.com:{}.git", github_path);
    
    tracing::info!("Trying SSH URL: {}", ssh_url);
    
    // Try again with SSH URL
    fetch_result = PrepareFetch::new(
        ssh_url.as_str(),
        repo_dir.clone(),
        Kind::WithWorktree,
        gix::create::Options::default(),
        OpenOptions::default(),
    );
}
```

Improved error messages to provide more specific guidance based on error type:

```rust
// Provide more specific error messages based on error type
let error_message = if e.to_string().contains("I/O error") {
    if clone_url.starts_with("https://github.com") {
        format!("Failed to clone repository via HTTPS: {}. SSH URL format might work better", e)
    } else {
        format!("Failed to clone repository (network error): {}", e)
    }
} else if e.to_string().contains("authentication") {
    format!("Failed to clone repository (authentication error): {}. Check your GitHub token", e)
} else {
    format!("Failed to clone repository: {}", e)
};
```

### Native Git Integration with Two-Phase Clone

- Migrated from direct git command execution to the native Rust `gix` library
- `gix` is a pure Rust implementation of Git with no C dependencies
- Implemented a two-phase clone process (fetch followed by checkout) for more precise control
- Repository operations (clone, fetch, checkout) are handled by `gix` APIs

```rust
// Repository cloning using gix instead of command execution
async fn clone_repository(&self, remote_repository: &GitRemoteRepository) -> Result<LocalRepository, String> {
    use gix::{
        clone::PrepareFetch,
        create::{Kind, Options as CreateOptions},
        open::Options as OpenOptions,
        progress::Discard,
    };
    
    // Initialize a repo for fetching with the authenticated URL
    let mut fetch = PrepareFetch::new(
        auth_url.as_str(),
        repo_dir,
        Kind::WithWorktree,
        CreateOptions::default(),
        OpenOptions::default(),
    )?;
    
    // Configure shallow clone for better performance
    let depth = NonZeroU32::new(1).unwrap();
    fetch = fetch.with_shallow(Shallow::DepthAtRemote(depth));
    
    // Execute the clone operation in two phases for better control
    match fetch.fetch_then_checkout(&mut Discard, &gix::interrupt::IS_INTERRUPTED) {
        Ok((mut checkout, _fetch_outcome)) => {
            // Complete the checkout process
            match checkout.main_worktree(Discard, &gix::interrupt::IS_INTERRUPTED) {
                Ok((_repo, _outcome)) => {
                    tracing::info!("Successfully cloned repository to {}", repo_dir.display());
                    Ok(local_repo)
                }
                Err(e) => {
                    // Clean up failed checkout attempt
                    if repo_dir.exists() {
                        let _ = std::fs::remove_dir_all(repo_dir);
                    }
                    Err(format!("Failed to checkout repository: {}", e))
                }
            }
        }
        Err(e) => {
            // Clean up failed clone attempt
            if repo_dir.exists() {
                let _ = std::fs::remove_dir_all(repo_dir);
            }
            Err(format!("Failed to clone repository: {}", e))
        }
    }
}
```

### Repository Caching Strategy

- Repositories are cloned to local cache directories using shallow clones
- Cache directories follow a deterministic naming pattern based on repository owner and name
- Cache strategy includes:
  - Hash-based directory naming for consistency
  - Process-specific identifiers in directory names to avoid conflicts
  - Automatic reuse of existing repositories
  - Validation of repository integrity before reuse
  - Cleanup of invalid repositories
- Custom cache directory configuration is supported via initialization parameters

### Refactoring: Process Isolation and Global Singleton

The codebase has been extensively refactored to implement two key patterns that work together:

#### Process-Specific Identifiers Pattern

- **Pattern:** Add unique process identifiers to shared resources
- **Example:** `RepositoryManager` includes a `process_id` field used in clone paths
- **Implementation:**
  ```rust
  // In RepositoryManager
  fn generate_process_id() -> String {
      use std::process;
      use uuid::Uuid;
      
      let pid = process::id();
      let uuid = Uuid::new_v4();
      
      format!("{}_{}", pid, uuid.simple())
  }
  
  // In LocalRepository
  pub fn new_local_repository_to_clone(
      remote_repository_info: GitRemoteRepositoryInfo, 
      process_id: Option<&str>
  ) -> Self {
      // Include process_id in directory name if provided
      let dir_name = if let Some(pid) = process_id {
          format!("mcp_gitcodes_{}_{}_{}_pid{}", 
                 remote_repository_info.user, 
                 remote_repository_info.repo, 
                 hash_value, pid)
      } else { /* ... */ }
      
      // Rest of the implementation
  }
  ```
- **Benefits:** Prevents resource conflicts when multiple processes use the same codebase
- **When to apply:** For file system operations, shared resource access, parallel processing

#### Global Singleton Pattern for Per-Process Resources

- **Pattern:** Use a global static instance with lazy initialization for per-process resources
- **Example:** Process-wide `RepositoryManager` singleton using `once_cell`
- **Implementation:**
  ```rust
  // Global singleton definition in repository_manager/instance.rs
  static GLOBAL_REPOSITORY_MANAGER: OnceCell<RepositoryManager> = OnceCell::new();
  
  // Initialization function (called once at process startup)
  pub fn init_repository_manager(
      github_token: Option<String>,
      repository_cache_dir: Option<PathBuf>,
  ) -> &'static RepositoryManager {
      GLOBAL_REPOSITORY_MANAGER.get_or_init(move || {
          RepositoryManager::new(github_token, repository_cache_dir)
              .expect("Failed to initialize global repository manager")
      })
  }
  
  // Access function (safely returns the singleton instance)
  pub fn get_repository_manager() -> &'static RepositoryManager {
      GLOBAL_REPOSITORY_MANAGER
          .get_or_init(|| RepositoryManager::with_default_cache_dir())
  }
  ```
  
  ```rust
  // Server startup code in transport modules
  async fn run_stdio_server(
      debug: bool, 
      github_token: Option<String>,
      repository_cache_dir: Option<std::path::PathBuf>
  ) -> Result<()> {
      // Initialize the global repository manager at startup
      // This ensures a single process_id is used throughout the application lifetime
      let _ = gitcodes_mcp::gitcodes::repository_manager::instance::init_repository_manager(
          github_token.clone(), 
          repository_cache_dir.clone()
      );
      
      // ... rest of the function ...
  }
  ```
  
  ```rust
  // Tool code updated to use global instance
  pub fn new(github_token: Option<String>, repository_cache_dir: Option<PathBuf>) -> Self {
      // Initialize the global repository manager with these parameters
      let manager = repository_manager::instance::init_repository_manager(github_token, repository_cache_dir);
      
      Self {
          manager: manager.clone(),
      }
  }
  
  pub fn with_service(_manager: RepositoryManager) -> Self {
      // Get the global repository manager instead of using the provided one
      let manager = repository_manager::instance::get_repository_manager();
      Self { manager: manager.clone() }
  }
  ```
- **Benefits:** 
  - Ensures consistency across the entire process
  - Preserves unique process identifiers throughout the application lifetime
  - Prevents duplicate resource initialization
  - Simplifies API by hiding process ID management from callers
- **When to apply:** 
  - For resources that should be unique per process
  - When resources have process-specific identifiers
  - To avoid unnecessary duplication of heavyweight resources
  - When configuration should be applied process-wide

### Authentication Methods

### Repository Cleanup Improvements

When cleanup is attempted on an explicitly provided local path, the system now silences warnings about cleanup failures. This improves the command-line experience by avoiding confusing warnings in expected scenarios.

```rust
fn cleanup_repository(repo: LocalRepository) {
    if let Err(err) = repo.cleanup() {
        // Only log warnings for temporary repositories (not user-provided local paths)
        if err.contains("doesn't match temporary repository pattern") {
            // This is an explicitly provided local path, suppress the warning
            tracing::debug!("Skipping cleanup for explicitly provided local path: {}", err);
        } else {
            tracing::warn!("Failed to clean up repository: {}", err);
        }
    } else {
        tracing::debug!("Successfully cleaned up repository");
    }
}
```

This pattern ensures the system: 
1. Attempts cleanup on all repositories for consistency
2. Silences warnings for explicitly provided local paths that shouldn't be cleaned up
3. Provides debug-level logging for skipped cleanups
4. Maintains warnings for actual cleanup failures in temporary repositories

- GitHub API authentication supports multiple methods:
  - Command-line parameter: `--github-token <token>`
  - Environment variable: `GITCODES_MCP_GITHUB_TOKEN`
  - Programmatic API parameter: `GitHubService::new(Some(token))`
- Authentication is applied at the transport level for Git operations
- Tokens are stored in memory and not referenced from environment after startup
- Unauthenticated requests are supported with rate limits (60 requests/hour vs 5,000/hour)

### CLI Relative Path Support

Enhanced the CLI to support relative paths while maintaining security:

1. Implemented a standalone `prevent_directory_traversal` function that doesn't require a `LocalRepository` instance
2. Added relative path to absolute path conversion in the CLI
3. Maintained security against directory traversal attacks by:
   - Rejecting paths containing ".." components
   - Using the standalone security function throughout the codebase
   - Adding thorough testing of path validation
4. Updated documentation and help text to reflect new behavior

### Security Function Refactoring Pattern

Refactored security-critical functions to be more reusable:

```rust
/// Prevents directory traversal attacks in paths
pub fn prevent_directory_traversal(path: &PathBuf) -> Result<(), String> {
    let path_str = path.to_string_lossy();
    
    // Check for directory traversal attempts using ".." in the path
    if path_str.contains("..") {
        return Err(format!(
            "Invalid path: '{}'. Paths containing '..' are not allowed for security reasons", 
            path.display()
        ));
    }
    
    // Check for URL-encoded traversal attempts
    if path_str.contains("%2E%2E") || path_str.contains("%2e%2e") {
        return Err(format!(
            "Invalid path: '{}'. Paths containing encoded traversal sequences are not allowed", 
            path.display()
        ));
    }
    
    Ok(())
}
```

This pattern promotes:
1. Code reuse across different components

### File URL Support for Repository Locations

Enhanced `RepositoryLocation` to support `file://` URLs as local repository paths:

```rust
// Handle file:// URLs by converting them to local paths
let path_str = if sanitized_location.starts_with("file:") {
    // Strip the file: or file:// prefix to get the actual path
    let path_part = sanitized_location.strip_prefix("file://")
        .or_else(|| sanitized_location.strip_prefix("file:"))
        .unwrap_or(sanitized_location);
    path_part
} else {
    sanitized_location
};
```

This implementation:
1. Detects strings starting with `file:` prefix
2. Properly handles both `file://` and `file:` formats
3. Converts file URLs to local paths before checking their existence
4. Updates all relevant documentation

This pattern promotes:
1. Flexibility in repository location specification
2. Support for standard file URL formats
3. Compatibility with tools that generate file:// paths
2. Consistent security implementations
3. Better testability of security functions
4. Clear separation between business logic and security concerns