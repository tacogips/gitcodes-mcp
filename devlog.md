# Development Log

This file documents the architectural decisions and implementation patterns for the GitCodes MCP project. It's organized by pattern categories rather than chronological history to better guide future code generation.

**IMPORTANT NOTE:** This devlog contains only changes made by AI agents and may not include modifications made directly by human programmers. There may be discrepancies between the current source code and the patterns documented here.

## Type System Patterns

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

### Deterministic Resource Naming

- **Pattern:** Generate consistent identifiers for resources
- **Example:** Repository cache directory naming using hash of repository info
- **Implementation:**
  ```rust
  fn generate_repository_hash(info: &RemoteGitRepositoryInfo) -> String {
      use std::collections::hash_map::DefaultHasher;
      use std::hash::{Hash, Hasher};
      
      let repo_key = format!("{}/{}", info.user, info.repo);
      let mut hasher = DefaultHasher::new();
      repo_key.hash(&mut hasher);
      let hash_value = hasher.finish();
      
      format!("{:x}", hash_value)[0..12].to_string()
  }
  ```
- **Benefits:** Predictable caching, consistent paths, no need for random UUID generation

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
- **Benefits:** Centralized conversion logic, consistent string representations

## Implementation Notes

### Native Git Integration

- Migrated from direct git command execution to the native Rust `gix` library
- `gix` is a pure Rust implementation of Git with no C dependencies
- This eliminates the need for external git command execution and simplifies error handling
- Repository operations (clone, fetch, checkout) are handled by `gix` APIs

```rust
// Repository cloning using gix instead of command execution
async fn clone_repository(&self, remote_repository: &GitRemoteRepository) -> Result<LocalRepository, String> {
    use gix::{
        clone::PrepareFetch,
        create::{Kind, Options as CreateOptions},
        open::Options as OpenOptions},
        progress::Discard,
    };
    
    // Initialize a repo for fetching
    let mut fetch = PrepareFetch::new(
        &clone_url,
        repo_dir,
        Kind::WorkTree,
        CreateOptions::default(),
        OpenOptions::default(),
    )?;
    
    // Configure authentication if available
    if let Some(token) = &self.github_token {
        fetch = fetch.configure_remote(|remote| {
            // Add GitHub authentication to remote URL
            Ok(remote.with_url(url_with_auth)?)
        });
    }
    
    // Execute the clone operation
    let (repository, _outcome) = fetch.fetch_only(Discard, &AtomicBool::new(false))?;
    Ok(local_repo)
}
```

### Repository Caching Strategy

- Repositories are cloned to local cache directories using shallow clones
- Cache directories follow a deterministic naming pattern based on repository owner and name
- Cache strategy includes:
  - Hash-based directory naming for consistency
  - Automatic reuse of existing repositories
  - Validation of repository integrity before reuse
  - Cleanup of invalid repositories
- Custom cache directory configuration is supported via initialization parameters

### Authentication Methods

- GitHub API authentication supports multiple methods:
  - Command-line parameter: `--github-token <token>`
  - Environment variable: `GITCODE_MCP_GITHUB_TOKEN`
  - Programmatic API parameter: `GitHubService::new(Some(token))`
- Authentication is applied at the transport level for Git operations
- Tokens are stored in memory and not referenced from environment after startup
- Unauthenticated requests are supported with rate limits (60 requests/hour vs 5,000/hour)