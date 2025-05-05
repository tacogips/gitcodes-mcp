use rmcp::{model::*, schemars, tool, ServerHandler};
use std::path::PathBuf;
use super::github_service::{
    params::{GrepParams, OrderOption, SearchParams, SortOption},
    GitHubService,
};

/// Wrapper for GitHub code tools exposed through the MCP protocol
///
/// This struct is a thin wrapper around the GitHubService, specifically
/// designed to expose functionality through the MCP tool protocol.
#[derive(Clone)]
pub struct GitHubCodeTools {
    /// The underlying GitHub service implementation
    service: GitHubService,
}

impl GitHubCodeTools {
    /// Creates a new GitHubCodeTools instance with optional authentication and custom temp dir
    ///
    /// # Authentication
    ///
    /// Authentication can be provided in two ways:
    /// 1. Explicitly via the `github_token` parameter (highest priority)
    /// 2. Environment variable `GITCODE_MCP_GITHUB_TOKEN` (used as fallback)
    ///
    /// # Parameters
    ///
    /// * `github_token` - Optional GitHub token for authentication. If None, will attempt to read from environment.
    /// * `temp_dir` - Optional path to a directory for storing temporary repositories.
    pub fn new(github_token: Option<String>, temp_dir: Option<PathBuf>) -> Self {
        Self {
            service: GitHubService::new(github_token, temp_dir),
        }
    }
    
    /// Creates a new GitHubCodeTools instance with default temp directory
    ///
    /// This is a convenience constructor that uses the system's temporary directory.
    ///
    /// # Parameters
    ///
    /// * `github_token` - Optional GitHub token for authentication.
    pub fn with_default_temp_dir(github_token: Option<String>) -> Self {
        Self::new(github_token, None)
    }

    /// Creates a new GitHubCodeTools with a specific GitHubService
    pub fn with_service(service: GitHubService) -> Self {
        Self { service }
    }
}

impl Default for GitHubCodeTools {
    fn default() -> Self {
        Self::new(None)
    }
}

#[tool(tool_box)]
impl ServerHandler for GitHubCodeTools {
    /// Provides information about this MCP server
    ///
    /// Returns server capabilities, protocol version, and usage instructions
    fn get_info(&self) -> ServerInfo {
        let auth_status = self.service.get_auth_status();

        let instructions = format!(
            "# GitHub and Rust Documentation MCP Server

## Authentication Status
{}

## Available Tools
- `search_repositories`: Search for GitHub repositories
- `grep_repository`: Search code within a GitHub repository
- `list_repository_refs`: List branches and tags for a repository

## Authentication
You can authenticate in three ways:

### Option 1: Command Line Argument (highest priority)
```
gitcodes-mcp stdio --github-token=your_token
gitcodes-mcp http --github-token=your_token
```

### Option 2: Environment Variable (used as fallback)
```
export GITCODE_MCP_GITHUB_TOKEN=your_github_token
```

### Option 3: Programmatic via new() method
Initialize with token using:
GitHubCodeTools::new(Some(\"token\"))

GitHub token is optional for public repositories but required for:
- Higher rate limits (5,000 vs 60 requests/hour)
- Accessing private repositories (requires 'repo' scope)
",
            auth_status
        );

        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(instructions),
        }
    }
}

#[tool(tool_box)]
impl GitHubCodeTools {
    /// Search for GitHub repositories using the GitHub API
    ///
    /// This method searches for repositories on GitHub based on the provided query.
    /// It supports sorting, pagination, and uses GitHub's search API.
    ///
    /// # Authentication
    ///
    /// - Uses the `GITCODE_MCP_GITHUB_TOKEN` if available for authentication
    /// - Without a token, limited to 60 requests/hour
    /// - With a token, allows 5,000 requests/hour
    ///
    /// # Rate Limiting
    ///
    /// GitHub API has rate limits that vary based on authentication:
    /// - Unauthenticated: 60 requests/hour
    /// - Authenticated: 5,000 requests/hour
    #[tool(
        description = "Search for GitHub repositories. Searches GitHub's API for repositories matching your query. Supports sorting by stars, forks, or update date, and pagination for viewing more results. Example usage: `{\"name\": \"search_repositories\", \"arguments\": {\"query\": \"rust http client\"}}`. With sorting: `{\"name\": \"search_repositories\", \"arguments\": {\"query\": \"game engine\", \"sort_by\": \"Stars\", \"order\": \"Descending\"}}`. With pagination: `{\"name\": \"search_repositories\", \"arguments\": {\"query\": \"machine learning\", \"per_page\": 50, \"page\": 2}}`"
    )]
    async fn search_repositories(
        &self,
        #[tool(param)]
        #[schemars(
            description = "Search query (required) - keywords to search for repositories. Can include advanced search qualifiers like 'language:rust' or 'stars:>1000'. Maximum length is 256 characters."
        )]
        query: String,

        #[tool(param)]
        #[schemars(
            description = "How to sort results (optional, default is 'relevance'). Options: Stars (most starred), Forks (most forked), Updated (most recently updated). When unspecified, results are sorted by best match to the query."
        )]
        sort_by: Option<SortOption>,

        #[tool(param)]
        #[schemars(
            description = "Sort order (optional, default is 'descending'). Options: Ascending (lowest to highest), Descending (highest to lowest). For date-based sorting like 'Updated', Descending means newest first."
        )]
        order: Option<OrderOption>,

        #[tool(param)]
        #[schemars(
            description = "Results per page (optional, default is 30, max 100). Controls how many repositories are returned in a single response. Higher values provide more comprehensive results but may include less relevant items."
        )]
        per_page: Option<u8>,

        #[tool(param)]
        #[schemars(
            description = "Result page number (optional, default is 1). Used for pagination to access results beyond the first page. GitHub limits search results to 1000 items total (across all pages)."
        )]
        page: Option<u32>,
    ) -> String {
        // Create a SearchParams struct from the individual parameters
        let params = SearchParams {
            query,
            sort_by,
            order,
            per_page,
            page,
        };

        self.service.search_repositories(params).await
    }

    /// Search code in a GitHub repository
    ///
    /// This tool clones or updates the repository locally, then performs a code search
    /// using the specified pattern. It supports both public and private repositories.
    ///
    /// # Authentication
    ///
    /// - For public repositories: No authentication needed
    /// - For private repositories: Requires `GITCODE_MCP_GITHUB_TOKEN` with `repo` scope
    ///
    /// # Implementation Note
    ///
    /// This tool uses a combination of git operations and the lumin search library:
    /// 1. Repository is cloned or updated locally
    /// 2. Code search is performed on the local files
    /// 3. Results are formatted and returned
    #[tool(
        description = "Search code in a GitHub repository. Clones the repository locally and searches for pattern matches in the code. Supports public and private repositories, branch/tag selection, and regex search. Example usage: `{\"name\": \"grep_repository\", \"arguments\": {\"repository\": \"https://github.com/rust-lang/rust\", \"pattern\": \"fn main\"}}`. With branch: `{\"name\": \"grep_repository\", \"arguments\": {\"repository\": \"github:tokio-rs/tokio\", \"ref_name\": \"master\", \"pattern\": \"async fn\"}}`. With search options: `{\"name\": \"grep_repository\", \"arguments\": {\"repository\": \"https://github.com/serde-rs/serde\", \"pattern\": \"Deserialize\", \"case_sensitive\": true, \"file_extensions\": [\"rs\"]}}`"
    )]
    async fn grep_repository(
        &self,
        #[tool(param)]
        #[schemars(
            description = "Repository URL (required) - supports GitHub formats: 'https://github.com/user/repo', 'git@github.com:user/repo.git', or 'github:user/repo'. For private repositories, the GITCODE_MCP_GITHUB_TOKEN environment variable must be set with a token having 'repo' scope."
        )]
        repository: String,

        #[tool(param)]
        #[schemars(
            description = "Branch or tag (optional, default is 'main' or 'master'). Specifies which branch or tag to search in. If the specified branch doesn't exist, falls back to 'main' or 'master'."
        )]
        ref_name: Option<String>,

        #[tool(param)]
        #[schemars(
            description = "Search pattern (required) - the text pattern to search for in the code. Supports regular expressions by default."
        )]
        pattern: String,

        #[tool(param)]
        #[schemars(
            description = "Whether to be case-sensitive (optional, default is false). When true, matching is exact with respect to letter case. When false, matches any letter case."
        )]
        case_sensitive: Option<bool>,

        #[tool(param)]
        #[schemars(
            description = "Whether to use regex (optional, default is true). Controls whether the pattern is interpreted as a regular expression or literal text."
        )]
        use_regex: Option<bool>,

        #[tool(param)]
        #[schemars(
            description = "File extensions to search (optional, e.g., [\"rs\", \"toml\"]). Limits search to files with specified extensions. Omit to search all text files."
        )]
        file_extensions: Option<Vec<String>>,

        #[tool(param)]
        #[schemars(
            description = "Directories to exclude from search (optional, e.g., [\"target\", \"node_modules\"]). Skips specified directories during search. Common build directories are excluded by default."
        )]
        exclude_dirs: Option<Vec<String>>,
    ) -> String {
        // Create a git_repositorys struct from the individual parameters
        let params = GrepParams {
            repository,
            ref_name,
            pattern,
            case_sensitive,
            use_regex,
            file_extensions,
            exclude_dirs,
        };

        self.service.grep_repository(params).await
    }

    /// List branches and tags for a GitHub repository
    ///
    /// This tool retrieves a list of all branches and tags for the specified repository.
    /// It supports both public and private repositories.
    ///
    /// # Authentication
    ///
    /// - For public repositories: No authentication needed
    /// - For private repositories: Requires `GITCODE_MCP_GITHUB_TOKEN` with `repo` scope
    ///
    /// # Implementation Note
    ///
    /// This tool:
    /// 1. Clones or updates the repository locally
    /// 2. Fetches all branches and tags
    /// 3. Formats the results into a readable format
    #[tool(
        description = "List branches and tags for a GitHub repository. Clones the repository locally and retrieves all branches and tags. Returns a formatted list of available references. Example usage: `{\"name\": \"list_repository_refs\", \"arguments\": {\"repository\": \"https://github.com/rust-lang/rust\"}}`. Another example: `{\"name\": \"list_repository_refs\", \"arguments\": {\"repository\": \"github:tokio-rs/tokio\"}}`"
    )]
    async fn list_repository_refs(
        &self,
        #[tool(param)]
        #[schemars(
            description = "Repository URL (required) - supports GitHub formats: 'https://github.com/user/repo', 'git@github.com:user/repo.git', or 'github:user/repo'. For private repositories, the GITCODE_MCP_GITHUB_TOKEN environment variable must be set with a token having 'repo' scope."
        )]
        repository: String,
    ) -> String {
        self.service.list_repository_refs(repository).await
    }
}
