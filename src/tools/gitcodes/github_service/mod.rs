//! GitHub service for interacting with repositories and code search
//!
//! This module provides a service for:
//! - Searching GitHub repositories
//! - Searching code within repositories (grep functionality)
//! - Listing branches and tags of repositories
//!
//! ## Authentication
//!
//! The service supports both authenticated and unauthenticated access to GitHub.
//! Authentication can be provided in two ways:
//!
//! ### 1. Environment Variable
//!
//! ```bash
//! # Authentication is optional but recommended to avoid rate limiting
//! export GITCODE_MCP_GITHUB_TOKEN=your_github_token
//! ```
//!
//! ### 2. Programmatic API
//!
//! ```no_run
//! // Provide a token directly when creating the service
//! use gitcodes_mcp::tools::gitcodes::github_service::GitHubService;
//!
//! let github_service = GitHubService::new(Some("your_github_token".to_string()));
//! ```

pub mod git_repository;
mod github_api;
mod code_search;
pub mod params;

pub use git_repository::*;
pub use params::*;

use reqwest::Client;

/// Repository information after URL parsing and preparation
#[derive(Debug)]
struct RepositoryInfo {
    /// GitHub username or organization
    user: String,
    /// Repository name
    repo: String,
    /// Local directory where repository is cloned
    repo_dir: String,
    /// Branch or tag name to use
    ref_name: String,
}

/// Service for GitHub repository operations
///
/// This struct provides integrated tools for GitHub operations:
/// - Repository searching
/// - Code searching within repositories
/// - Branch and tag listing
///
/// # Authentication
///
/// The service handles GitHub authentication through the `GITCODE_MCP_GITHUB_TOKEN`
/// environment variable. This token is:
/// - Read once at startup and stored in memory
/// - Used for all GitHub API requests
/// - Optional, but recommended to avoid rate limiting (60 vs 5,000 requests/hour)
/// - Required for accessing private repositories (with `repo` scope)
#[derive(Clone)]
pub struct GitHubService {
    /// HTTP client for API requests
    pub client: Client,
    /// Manager for repository operations
    pub repo_manager: RepositoryManager,
    /// GitHub authentication token (if provided via GITCODE_MCP_GITHUB_TOKEN)
    pub github_token: Option<String>,
}

impl Default for GitHubService {
    fn default() -> Self {
        Self::new(None)
    }
}

impl GitHubService {
    /// Creates a new GitHub service instance
    ///
    /// Initializes:
    /// - HTTP client for API requests
    /// - Repository manager for Git operations
    /// - GitHub token from the provided parameter or environment variable
    ///
    /// # Authentication
    ///
    /// Authentication can be provided in three ways:
    /// 1. Command line argument `--github-token` (highest priority)
    /// 2. Explicitly via the `github_token` parameter in code (second priority)
    /// 3. Environment variable `GITCODE_MCP_GITHUB_TOKEN` (used as fallback)
    ///
    /// If no token is provided through either method, the system will work with
    /// lower rate limits (60 requests/hour vs 5,000 requests/hour).
    ///
    /// # Parameters
    ///
    /// * `github_token` - Optional GitHub token for authentication. If None, will attempt to read from environment.
    pub fn new(github_token: Option<String>) -> Self {
        Self {
            client: Client::new(),
            repo_manager: RepositoryManager::new(),
            github_token,
        }
    }

    /// Get the authentication status for display
    pub fn get_auth_status(&self) -> String {
        if self.github_token.is_some() {
            "Authenticated GitHub API access enabled (5,000 requests/hour)".to_string()
        } else {
            "Unauthenticated GitHub API access (60 requests/hour limit). Set GITCODE_MCP_GITHUB_TOKEN for higher limits.".to_string()
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
    pub async fn search_repositories(&self, params: SearchParams) -> String {
        //TODO(tacogips) this method should return anyhow::Result<String> instead of String
        // Execute the search request
        github_api::execute_search_request(&params, &self.client, self.github_token.as_ref()).await
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
    pub async fn grep_repository(&self, params: GrepParams) -> String {
        // Parse repository information from URL
        let repo_info = match self
            .parse_and_prepare_repository(&params.repository, params.ref_name)
            .await
        {
            Ok(info) => info,
            Err(e) => return e,
        };

        // Execute code search
        let search_result = code_search::perform_code_search(
            &repo_info.repo_dir,
            &params.pattern,
            params.case_sensitive,
            params.use_regex,
            params.file_extensions.clone(),
        )
        .await;

        // Format and return results
        code_search::format_search_results(&search_result, &params.pattern, &params.repository)
    }

    /// Parses a repository URL and prepares it for operations
    ///
    /// This helper function:
    /// 1. Extracts user and repo name from the URL
    /// 2. Creates or determines the repository directory
    /// 3. Ensures the repository is cloned or updated locally
    async fn parse_and_prepare_repository(
        &self,
        repository: &str,
        ref_name: Option<String>,
    ) -> Result<RepositoryInfo, String> {
        // Parse repository URL
        let (user, repo) =
            match git_repository::parse_repository_url(&self.repo_manager, repository) {
                Ok(result) => result,
                Err(e) => return Err(format!("Error: {}", e)),
            };

        // Default branch if not specified
        let ref_name = ref_name.unwrap_or_else(|| "main".to_string());

        // Get a temporary directory for the repository
        let repo_dir = git_repository::get_repo_dir(&self.repo_manager, &user, &repo);

        // Check if repo is already cloned
        let is_cloned = git_repository::is_repo_cloned(&self.repo_manager, &repo_dir).await;

        // If repo is not cloned, clone it
        if !is_cloned {
            git_repository::clone_repository(&repo_dir, &user, &repo, &ref_name).await?
        } else {
            git_repository::update_repository(&repo_dir, &ref_name).await?
        }

        Ok(RepositoryInfo {
            user,
            repo,
            repo_dir,
            ref_name,
        })
    }

    // Code search methods have been moved to code_search.rs

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
    pub async fn list_repository_refs(&self, repository: String) -> String {
        // Parse repository URL
        let (user, repo) =
            match git_repository::parse_repository_url(&self.repo_manager, &repository) {
                Ok(result) => result,
                Err(e) => return format!("Error: {}", e),
            };

        // Get a temporary directory for the repository
        let repo_dir = git_repository::get_repo_dir(&self.repo_manager, &user, &repo);

        // Check if repo is already cloned
        let is_cloned = git_repository::is_repo_cloned(&self.repo_manager, &repo_dir).await;

        // If repo is not cloned, clone it
        if !is_cloned {
            match git_repository::clone_repository(&repo_dir, &user, &repo, "main").await {
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

    // Now using git_repository functions instead of local implementations
    // Functions have been moved to git_repository.rs
}
