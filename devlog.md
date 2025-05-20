# Development Log

This file documents the architectural decisions and implementation patterns for the GitCodes MCP project. It's organized by pattern categories rather than chronological history to better guide future code generation.

**IMPORTANT NOTE:** This devlog contains only changes made by AI agents and may not include modifications made directly by human programmers. There may be discrepancies between the current source code and the patterns documented here.

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

- GitHub API authentication supports multiple methods:
  - Command-line parameter: `--github-token <token>`
  - Environment variable: `GITCODE_MCP_GITHUB_TOKEN`
  - Programmatic API parameter: `GitHubService::new(Some(token))`
- Authentication is applied at the transport level for Git operations
- Tokens are stored in memory and not referenced from environment after startup
- Unauthenticated requests are supported with rate limits (60 requests/hour vs 5,000/hour)