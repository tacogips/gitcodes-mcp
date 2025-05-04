//! GitHub tools for interacting with repositories and code search
//! 
//! This module provides tools for:
//! - Searching GitHub repositories
//! - Searching code within repositories (grep functionality)
//! - Listing branches and tags of repositories
//!
//! ## Authentication
//!
//! These tools support both authenticated and unauthenticated access to GitHub:
//!
//! ```
//! # Authentication is optional but recommended to avoid rate limiting
//! export GITCODE_MCP_GITHUB_TOKEN=your_github_token
//! ```
//!
//! ### GitHub Token (`GITCODE_MCP_GITHUB_TOKEN`)
//!
//! - **Purpose**: Authenticates requests to GitHub API
//! - **Requirement**: Optional, but strongly recommended to avoid rate limits
//! - **Rate Limits**:
//!   - Without token: 60 requests/hour (unauthenticated)
//!   - With token: 5,000 requests/hour (authenticated)
//! - **Usage**: Set as environment variable before starting the server
//! - **Security**: Token is read once at startup and stored in memory
//! - **Permissions**: For private repositories, token must have `repo` scope
//!
//! ### When Token is NOT Required
//!
//! A GitHub token is not required if:
//! - You're only accessing public repositories
//! - You're making few requests (under 60 per hour)
//! - You don't need to access private repositories
//!
//! All public repository operations work without authentication, but with
//! significantly lower rate limits.

mod git_repository;

use lumin::{search, search::SearchOptions};
use rand::Rng;
use reqwest::Client;

use rmcp::{model::*, schemars, tool, ServerHandler};

/// Repository manager for Git operations
///
/// Handles cloning, updating, and retrieving information from GitHub repositories.
/// Uses system temporary directories to store cloned repositories.
#[derive(Clone)]
pub struct RepositoryManager {
    temp_dir_base: String,
}

impl Default for RepositoryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RepositoryManager {
    pub fn new() -> Self {
        let system_temp = std::env::temp_dir().to_string_lossy().to_string();
        Self {
            temp_dir_base: system_temp,
        }
    }

    // Parse repository URL to extract user and repo name
    pub fn parse_repository_url(&self, url: &str) -> Result<(String, String), String> {
        let user_repo = if url.starts_with("https://github.com/") {
            url.trim_start_matches("https://github.com/")
                .trim_end_matches(".git")
                .to_string()
        } else if url.starts_with("git@github.com:") {
            url.trim_start_matches("git@github.com:")
                .trim_end_matches(".git")
                .to_string()
        } else if url.starts_with("github:") {
            url.trim_start_matches("github:").to_string()
        } else {
            return Err("Invalid GitHub repository URL format".to_string());
        };

        let parts: Vec<&str> = user_repo.split('/').collect();
        if parts.len() != 2 {
            return Err("Invalid GitHub repository URL format".to_string());
        }

        Ok((parts[0].to_string(), parts[1].to_string()))
    }

    // Generate a unique directory name for the repository
    pub fn get_repo_dir(&self, user: &str, repo: &str) -> String {
        format!(
            "{}/mcp_github_{}_{}_{}",
            self.temp_dir_base,
            user,
            repo,
            rand::thread_rng().gen::<u32>() % 10000
        )
    }

    // Check if repository is already cloned
    pub async fn is_repo_cloned(&self, dir: &str) -> bool {
        tokio::fs::metadata(dir).await.is_ok()
    }
}

/// Main router for GitHub and crate documentation functionality
///
/// This struct handles GitHub API requests and provides tools for:
/// - Repository searching
/// - Code searching within repositories
/// - Branch and tag listing
/// - (Planned) Rust crate documentation
///
/// # Authentication
///
/// The router handles GitHub authentication through the `GITCODE_MCP_GITHUB_TOKEN` 
/// environment variable. This token is:
/// - Read once at startup and stored in memory
/// - Used for all GitHub API requests
/// - Optional, but recommended to avoid rate limiting (60 vs 5,000 requests/hour)
/// - Required for accessing private repositories (with `repo` scope)
#[derive(Clone)]
pub struct CargoDocRouter {
    /// HTTP client for API requests
    pub client: Client,
    /// Manager for repository operations
    pub repo_manager: RepositoryManager,
    /// GitHub authentication token (if provided via GITCODE_MCP_GITHUB_TOKEN)
    pub github_token: Option<String>,
}

impl Default for CargoDocRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[tool(tool_box)]
impl CargoDocRouter {
    /// Creates a new router instance
    ///
    /// Initializes:
    /// - HTTP client for API requests
    /// - Repository manager for Git operations
    /// - GitHub token from GITCODE_MCP_GITHUB_TOKEN environment variable (if available)
    ///
    /// # Authentication
    ///
    /// The GitHub token is read from the `GITCODE_MCP_GITHUB_TOKEN` environment variable.
    /// If not provided, the system still works but with lower rate limits (60 requests/hour).
    pub fn new() -> Self {
        // Read GitHub token from environment variable
        let github_token = std::env::var("GITCODE_MCP_GITHUB_TOKEN").ok();

        Self {
            client: Client::new(),
            repo_manager: RepositoryManager::new(),
            github_token,
        }
    }

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
    #[tool(description = "Search for GitHub repositories. Searches GitHub's API for repositories matching your query. Supports sorting by stars, forks, or update date, and pagination for viewing more results. Example usage: `{\"name\": \"search_repositories\", \"arguments\": {\"query\": \"rust http client\"}}`. With sorting: `{\"name\": \"search_repositories\", \"arguments\": {\"query\": \"game engine\", \"sort_by\": \"Stars\", \"order\": \"Descending\"}}`. With pagination: `{\"name\": \"search_repositories\", \"arguments\": {\"query\": \"machine learning\", \"per_page\": 50, \"page\": 2}}`")]
    async fn search_repositories(
        &self,
        #[tool(param)]
        #[schemars(description = "Search query (required) - keywords to search for repositories. Can include advanced search qualifiers like 'language:rust' or 'stars:>1000'. Maximum length is 256 characters.")]
        query: String,

        #[tool(param)]
        #[schemars(description = "How to sort results (optional, default is 'relevance'). Options: Stars (most starred), Forks (most forked), Updated (most recently updated). When unspecified, results are sorted by best match to the query.")]
        sort_by: Option<SortOption>,

        #[tool(param)]
        #[schemars(description = "Sort order (optional, default is 'descending'). Options: Ascending (lowest to highest), Descending (highest to lowest). For date-based sorting like 'Updated', Descending means newest first.")]
        order: Option<OrderOption>,

        #[tool(param)]
        #[schemars(description = "Results per page (optional, default is 30, max 100). Controls how many repositories are returned in a single response. Higher values provide more comprehensive results but may include less relevant items.")]
        per_page: Option<u8>,

        #[tool(param)]
        #[schemars(description = "Result page number (optional, default is 1). Used for pagination to access results beyond the first page. GitHub limits search results to 1000 items total (across all pages).")]
        page: Option<u32>,
    ) -> String {
        // Set up request parameters
        let sort = match sort_by {
            Some(SortOption::Stars) => "stars",
            Some(SortOption::Forks) => "forks",
            Some(SortOption::Updated) => "updated",
            None => "", // Default is relevance
        };
        let order_param = match order {
            Some(OrderOption::Ascending) => "asc",
            Some(OrderOption::Descending) => "desc",
            None => "desc", // Default is descending
        };
        // Ensure per_page is within limits
        let per_page = per_page.unwrap_or(30).min(100);
        let page = page.unwrap_or(1);

        // Construct the API URL
        let mut url = format!(
            "https://api.github.com/search/repositories?q={}",
            urlencoding::encode(&query)
        );

        if !sort.is_empty() {
            url.push_str(&format!("&sort={}", sort));
        }

        url.push_str(&format!("&order={}", order_param));
        url.push_str(&format!("&per_page={}&page={}", per_page, page));

        // Set up the API request
        let mut req_builder = self.client.get(&url).header(
            "User-Agent",
            "gitcodes-mcp/0.1.0 (https://github.com/d6e/gitcodes-mcp)",
        );

        // Add authentication token if available
        if let Some(token) = &self.github_token {
            req_builder = req_builder.header("Authorization", format!("token {}", token));
        }

        // Execute API request
        let response = match req_builder.send().await {
            Ok(resp) => resp,
            Err(e) => return format!("Failed to search repositories: {}", e),
        };

        // Check if the request was successful
        let status = response.status();
        if !status.is_success() {
            let error_text = match response.text().await {
                Ok(text) => text,
                Err(_) => "Unknown error".to_string(),
            };

            return format!("GitHub API error {}: {}", status, error_text);
        }

        // Return the raw JSON response
        match response.text().await {
            Ok(text) => text,
            Err(e) => format!("Failed to read response body: {}", e),
        }
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
    #[tool(description = "Search code in a GitHub repository. Clones the repository locally and searches for pattern matches in the code. Supports public and private repositories, branch/tag selection, and regex search. Example usage: `{\"name\": \"grep_repository\", \"arguments\": {\"repository\": \"https://github.com/rust-lang/rust\", \"pattern\": \"fn main\"}}`. With branch: `{\"name\": \"grep_repository\", \"arguments\": {\"repository\": \"github:tokio-rs/tokio\", \"ref_name\": \"master\", \"pattern\": \"async fn\"}}`. With search options: `{\"name\": \"grep_repository\", \"arguments\": {\"repository\": \"https://github.com/serde-rs/serde\", \"pattern\": \"Deserialize\", \"case_sensitive\": true, \"file_extensions\": [\"rs\"]}}`")]
    async fn grep_repository(
        &self,
        #[tool(param)]
        #[schemars(description = "Repository URL (required) - supports GitHub formats: 'https://github.com/user/repo', 'git@github.com:user/repo.git', or 'github:user/repo'. For private repositories, the GITCODE_MCP_GITHUB_TOKEN environment variable must be set with a token having 'repo' scope.")]
        repository: String,

        #[tool(param)]
        #[schemars(description = "Branch or tag (optional, default is 'main' or 'master'). Specifies which branch or tag to search in. If the specified branch doesn't exist, falls back to 'main' or 'master'.")]
        ref_name: Option<String>,

        #[tool(param)]
        #[schemars(description = "Search pattern (required) - the text pattern to search for in the code. Supports regular expressions by default.")]
        pattern: String,

        #[tool(param)]
        #[schemars(description = "Whether to be case-sensitive (optional, default is false). When true, matching is exact with respect to letter case. When false, matches any letter case.")]
        case_sensitive: Option<bool>,

        #[tool(param)]
        #[schemars(description = "Whether to use regex (optional, default is true). Controls whether the pattern is interpreted as a regular expression or literal text.")]
        use_regex: Option<bool>,

        #[tool(param)]
        #[schemars(description = "File extensions to search (optional, e.g., [\"rs\", \"toml\"]). Limits search to files with specified extensions. Omit to search all text files.")]
        file_extensions: Option<Vec<String>>,

        #[tool(param)]
        #[schemars(
            description = "Directories to exclude from search (optional, e.g., [\"target\", \"node_modules\"]). Skips specified directories during search. Common build directories are excluded by default."
        )]
        _exclude_dirs: Option<Vec<String>>,
    ) -> String {
        // Parse repository URL
        let (user, repo) = match self.repo_manager.parse_repository_url(&repository) {
            Ok(result) => result,
            Err(e) => return format!("Error: {}", e),
        };

        // Default branch if not specified
        let ref_name = ref_name.unwrap_or_else(|| "main".to_string());

        // Get a temporary directory for the repository
        let repo_dir = self.repo_manager.get_repo_dir(&user, &repo);

        // Check if repo is already cloned
        let is_cloned = self.repo_manager.is_repo_cloned(&repo_dir).await;

        // If repo is not cloned, clone it
        if !is_cloned {
            let result = self
                .clone_repository(&repo_dir, &user, &repo, &ref_name)
                .await;
            if let Err(e) = result {
                return e;
            }
        } else {
            let result = self.update_repository(&repo_dir, &ref_name).await;
            if let Err(e) = result {
                return e;
            }
        }

        // Use lumin for search
        let repo_dir_clone = repo_dir.clone();
        let pattern_clone = pattern.clone();
        let search_result = tokio::task::spawn_blocking(move || {
            // Create search options
            let mut search_options = SearchOptions::default();

            // Configure case sensitivity
            search_options.case_sensitive = case_sensitive.unwrap_or(false);

            // Execute the search
            match search::search_files(&pattern_clone, std::path::Path::new(&repo_dir_clone), &search_options) {
                Ok(results) => {
                    // Format results
                    let mut output = String::new();

                    for result in results {
                        output.push_str(&format!(
                            "{}:{}: {}\n",
                            result.file_path.display(),
                            result.line_number,
                            result.line_content
                        ));
                    }

                    output
                }
                Err(e) => format!("Lumin search failed: {}", e),
            }
        })
        .await
        .map_err(|e| format!("Search task failed: {}", e));

        // Handle search errors
        if let Err(e) = &search_result {
            return format!("Search failed: {}", e);
        }

        let search_output = search_result.unwrap();
        if search_output.trim().is_empty() {
            format!(
                "No matches found for pattern '{}' in repository {}",
                pattern, repository
            )
        } else {
            format!(
                "Search results for '{}' in repository {}:\n\n{}",
                pattern, repository, search_output
            )
        }
    }

    // Function to fetch repository refs (branches and tags)
    async fn fetch_repository_refs(
        &self,
        repo_dir: &str,
        user: &str,
        repo: &str,
    ) -> Result<String, String> {
        // Get branches and tags
        let repo_dir_clone = repo_dir.to_string();
        let user_clone = user.to_string();
        let repo_clone = repo.to_string();

        // Change to the repository directory
        let current_dir = match std::env::current_dir() {
            Ok(dir) => dir,
            Err(e) => return Err(format!("Failed to get current directory: {}", e)),
        };

        if let Err(e) = std::env::set_current_dir(&repo_dir_clone) {
            return Err(format!("Failed to change directory: {}", e));
        }

        // First run git fetch to make sure we have all refs
        let fetch_status = std::process::Command::new("git")
            .args(["fetch", "--all"])
            .status();

        if let Err(e) = fetch_status {
            let _ = std::env::set_current_dir(current_dir);
            return Err(format!("Git fetch failed: {}", e));
        }

        if !fetch_status.unwrap().success() {
            let _ = std::env::set_current_dir(current_dir);
            return Err("Git fetch failed".to_string());
        }

        // Get branches
        let branches_output = std::process::Command::new("git")
            .args(["branch", "-r"])
            .output();

        let branches_output = match branches_output {
            Ok(output) => output,
            Err(e) => {
                let _ = std::env::set_current_dir(current_dir);
                return Err(format!("Failed to list branches: {}", e));
            }
        };

        let branches_str = String::from_utf8_lossy(&branches_output.stdout).to_string();

        // Get tags
        let tags_output = std::process::Command::new("git").args(["tag"]).output();

        let tags_output = match tags_output {
            Ok(output) => output,
            Err(e) => {
                let _ = std::env::set_current_dir(current_dir);
                return Err(format!("Failed to list tags: {}", e));
            }
        };

        let tags_str = String::from_utf8_lossy(&tags_output.stdout).to_string();

        // Change back to the original directory
        if let Err(e) = std::env::set_current_dir(current_dir) {
            return Err(format!("Failed to restore directory: {}", e));
        }

        // Format the output
        let mut result = String::new();
        result.push_str(&format!(
            "Repository: {}/{}

",
            user_clone, repo_clone
        ));

        // Extract and format branches
        let branches: Vec<String> = branches_str
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.starts_with("origin/") && !line.contains("HEAD") {
                    Some(line.trim_start_matches("origin/").to_string())
                } else {
                    None
                }
            })
            .collect();

        // Extract and format tags
        let tags: Vec<String> = tags_str
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();

        // Add branches section
        result.push_str(
            "## Branches
",
        );
        if branches.is_empty() {
            result.push_str(
                "No branches found
",
            );
        } else {
            for branch in branches {
                result.push_str(&format!("- {}\n", branch));
            }
        }

        // Add tags section
        result.push_str(
            "
## Tags
",
        );
        if tags.is_empty() {
            result.push_str(
                "No tags found
",
            );
        } else {
            for tag in tags {
                result.push_str(&format!("- {}\n", tag));
            }
        }

        Ok(result)
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
    #[tool(description = "List branches and tags for a GitHub repository. Clones the repository locally and retrieves all branches and tags. Returns a formatted list of available references. Example usage: `{\"name\": \"list_repository_refs\", \"arguments\": {\"repository\": \"https://github.com/rust-lang/rust\"}}`. Another example: `{\"name\": \"list_repository_refs\", \"arguments\": {\"repository\": \"github:tokio-rs/tokio\"}}`")]
    async fn list_repository_refs(
        &self,
        #[tool(param)]
        #[schemars(description = "Repository URL (required) - supports GitHub formats: 'https://github.com/user/repo', 'git@github.com:user/repo.git', or 'github:user/repo'. For private repositories, the GITCODE_MCP_GITHUB_TOKEN environment variable must be set with a token having 'repo' scope.")]
        repository: String,
    ) -> String {
        // Parse repository URL
        let (user, repo) = match self.repo_manager.parse_repository_url(&repository) {
            Ok(result) => result,
            Err(e) => return format!("Error: {}", e),
        };

        // Get a temporary directory for the repository
        let repo_dir = self.repo_manager.get_repo_dir(&user, &repo);

        // Check if repo is already cloned
        let is_cloned = self.repo_manager.is_repo_cloned(&repo_dir).await;

        // If repo is not cloned, clone it
        if !is_cloned {
            match self.clone_repository(&repo_dir, &user, &repo, "main").await {
                Ok(_) => {}
                Err(e) => return e,
            }
        }

        // Fetch repository refs using the extracted function
        match self.fetch_repository_refs(&repo_dir, &user, &repo).await {
            Ok(result) => result,
            Err(e) => format!("Failed to list refs: {}", e),
        }
    }

    ////
    // Look up documentation for a Rust crate
    //    #[tool(description = "Look up documentation for a Rust crate")]
    //    async fn lookup_crate(
    //        &self,
    //        #[tool(param)]
    //        #[schemars(description = "The name of the crate to look up")]
    //        crate_name: String,
    //
    //        #[tool(param)]
    //        #[schemars(description = "The version of the crate (optional, defaults to latest)")]
    //        version: Option<String>,
    //    ) -> String {
    //        // Check cache first
    //        let cache_key = if let Some(ver) = &version {
    //            format!("{}}:{}", crate_name, ver)
    //        } else {
    //            crate_name.clone()
    //        };
    //
    //        // Construct the docs.rs URL for the crate
    //        let url = if let Some(ver) = version {
    //            format!("https://docs.rs/crate/{}/{}/", crate_name, ver)
    //        } else {
    //            format!("https://docs.rs/crate/{}/", crate_name)
    //        };
    //
    //        // Fetch the documentation page
    //        let response = match self
    //            .client
    //            .get(&url)
    //            .header(
    //                "User-Agent",
    //                "gitcodes/0.1.0 (https://github.com/d6e/gitcodes-mcp)",
    //            )
    //            .send()
    //            .await
    //        {
    //            Ok(resp) => resp,
    //            Err(e) => return format!("Failed to fetch documentation: {}", e),
    //        };
    //
    //        if !response.status().is_success() {
    //            return format!(
    //                "Failed to fetch documentation. Status: {}",
    //                response.status()
    //            );
    //        }
    //
    //        let html_body = match response.text().await {
    //            Ok(body) => body,
    //            Err(e) => return format!("Failed to read response body: {}", e),
    //        };
    //
    //        // Convert HTML to markdown
    //        let markdown_body = parse_html(&html_body);
    //
    //        // Cache the markdown result
    //        self.cache.set(cache_key, markdown_body.clone()).await;
    //
    //        markdown_body
    //    }

    // Clone repository function
    async fn clone_repository(
        &self,
        repo_dir: &str,
        user: &str,
        repo: &str,
        ref_name: &str,
    ) -> Result<(), String> {
        // Create directory if it doesn't exist
        if let Err(e) = tokio::fs::create_dir_all(repo_dir).await {
            return Err(format!("Failed to create directory: {}", e));
        }

        // Clone repository
        let clone_url = format!("https://github.com/{}/{}.git", user, repo);

        // Clone with git command
        let repo_dir_clone = repo_dir.to_string();
        let ref_name_clone = ref_name.to_string();
        let clone_result = tokio::task::spawn_blocking(move || {
            let status = std::process::Command::new("git")
                .args([
                    "clone",
                    "--depth=1",
                    "--branch",
                    &ref_name_clone,
                    &clone_url,
                    &repo_dir_clone,
                ])
                .status();

            match status {
                Ok(exit_status) if exit_status.success() => Ok(()),
                Ok(exit_status) => Err(format!("Git clone failed with status: {}", exit_status)),
                Err(e) => Err(format!("Failed to execute git clone: {}", e)),
            }
        })
        .await;

        // Handle errors during cloning
        if let Err(e) = clone_result {
            return Err(format!("Failed to run git clone: {}", e));
        }

        clone_result.unwrap()
    }

    // Update repository function
    async fn update_repository(&self, repo_dir: &str, ref_name: &str) -> Result<(), String> {
        // Repository exists, update it
        let repo_dir_clone = repo_dir.to_string();
        let ref_name_clone = ref_name.to_string();
        let update_result = tokio::task::spawn_blocking(move || {
            // Change to the repository directory
            let current_dir = match std::env::current_dir() {
                Ok(dir) => dir,
                Err(e) => return Err(format!("Failed to get current directory: {}", e)),
            };

            if let Err(e) = std::env::set_current_dir(&repo_dir_clone) {
                return Err(format!("Failed to change directory: {}", e));
            }

            // Fetch updates
            let fetch_status = std::process::Command::new("git")
                .args(["fetch", "--depth=1", "origin"])
                .status();

            if let Err(e) = fetch_status {
                let _ = std::env::set_current_dir(current_dir);
                return Err(format!("Git fetch failed: {}", e));
            }

            if !fetch_status.unwrap().success() {
                let _ = std::env::set_current_dir(current_dir);
                return Err("Git fetch failed".to_string());
            }

            // Try to checkout the requested branch
            let checkout_status = std::process::Command::new("git")
                .args(["checkout", &ref_name_clone])
                .status();

            if let Err(e) = checkout_status {
                let _ = std::env::set_current_dir(current_dir);
                return Err(format!("Git checkout failed: {}", e));
            }

            if !checkout_status.unwrap().success() {
                // Try origin/branch_name
                let origin_checkout = std::process::Command::new("git")
                    .args(["checkout", &format!("origin/{}", ref_name_clone)])
                    .status();

                if let Err(e) = origin_checkout {
                    let _ = std::env::set_current_dir(current_dir);
                    return Err(format!("Git checkout failed: {}", e));
                }

                if !origin_checkout.unwrap().success() {
                    let _ = std::env::set_current_dir(current_dir);
                    return Err(format!("Branch/tag not found: {}", ref_name_clone));
                }
            }

            // Change back to the original directory
            if let Err(e) = std::env::set_current_dir(current_dir) {
                return Err(format!("Failed to restore directory: {}", e));
            }

            Ok(())
        })
        .await;

        // Handle update errors
        if let Err(e) = update_result {
            return Err(format!("Failed to update repository: {}", e));
        }

        update_result.unwrap()
    }
}

#[tool(tool_box)]
impl ServerHandler for CargoDocRouter {
    /// Provides information about this MCP server
    ///
    /// Returns server capabilities, protocol version, and usage instructions
    fn get_info(&self) -> ServerInfo {
        let auth_status = if self.github_token.is_some() {
            "Authenticated GitHub API access enabled (5,000 requests/hour)"
        } else {
            "Unauthenticated GitHub API access (60 requests/hour limit). Set GITCODE_MCP_GITHUB_TOKEN for higher limits."
        };
        
        let instructions = format!(
            "# GitHub and Rust Documentation MCP Server
            
## Authentication Status
{}

## Available Tools
- `search_repositories`: Search for GitHub repositories
- `grep_repository`: Search code within a GitHub repository
- `list_repository_refs`: List branches and tags for a repository

## Authentication
To increase rate limits and access private repositories:
```
export GITCODE_MCP_GITHUB_TOKEN=your_github_token
```

GitHub token is optional for public repositories but required for:
- Higher rate limits (5,000 vs 60 requests/hour)
- Accessing private repositories (requires 'repo' scope)
", auth_status);

        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(instructions),
        }
    }
}

/// Sort options for GitHub repository search results
///
/// Controls how repository search results are ordered in the response.
#[derive(Debug, schemars::JsonSchema, serde::Serialize, serde::Deserialize)]
pub enum SortOption {
    /// Sort by number of stars (popularity)
    Stars,
    /// Sort by number of forks (derived projects)
    Forks,
    /// Sort by most recently updated
    Updated,
}

/// Sort direction options for GitHub repository search results
///
/// Controls whether results are displayed in ascending or descending order.
#[derive(Debug, schemars::JsonSchema, serde::Serialize, serde::Deserialize)]
pub enum OrderOption {
    /// Sort in ascending order (lowest to highest, oldest to newest)
    Ascending,
    /// Sort in descending order (highest to lowest, newest to oldest)
    Descending,
}
