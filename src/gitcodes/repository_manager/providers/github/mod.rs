use reqwest::Client;
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumString};

use crate::gitcodes::repository_manager::providers::GitRemoteRepositoryInfo;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct GithubRemoteInfo {
    pub clone_url: String,
    pub repo_info: GitRemoteRepositoryInfo,
}

/// Sort options for GitHub repository search results
///
/// Controls how repository search results are ordered in the response.
#[derive(Debug, Serialize, Deserialize, Display, EnumString, AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub enum GithubSortOption {
    /// No specific sort, use GitHub's default relevance sorting
    #[strum(serialize = "")]
    Relevance,
    /// Sort by number of stars (popularity)
    #[strum(serialize = "stars")]
    Stars,
    /// Sort by number of forks (derived projects)
    #[strum(serialize = "forks")]
    Forks,
    /// Sort by most recently updated
    #[strum(serialize = "updated")]
    Updated,
}

impl GithubSortOption {
    /// Converts the sort option to its API string representation
    pub fn to_str(&self) -> &str {
        self.as_ref()
    }
}

impl Default for GithubSortOption {
    /// Returns the default sort option (Relevance)
    fn default() -> Self {
        GithubSortOption::Relevance
    }
}

/// Sort direction options for GitHub repository search results
///
/// Controls whether results are displayed in ascending or descending order.
#[derive(Debug, serde::Serialize, serde::Deserialize, Display, EnumString, AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub enum GithubOrderOption {
    /// Sort in ascending order (lowest to highest, oldest to newest)
    #[strum(serialize = "asc")]
    Ascending,
    /// Sort in descending order (highest to lowest, newest to oldest)
    #[strum(serialize = "desc")]
    Descending,
}

impl GithubOrderOption {
    /// Converts the order option to its API string representation
    pub fn to_str(&self) -> &str {
        self.as_ref()
    }
}

impl Default for GithubOrderOption {
    /// Returns the default order option (Descending)
    fn default() -> Self {
        GithubOrderOption::Descending
    }
}

/// Search parameters for GitHub repository search
///
/// Contains all the parameters needed for configuring a repository search request to GitHub's API.
/// This struct handles both the parameter validation and URL construction for repository searches.
///
/// # Parameter Handling
///
/// - `sort_by`: Uses SortOption::Relevance if None (empty string in the URL)
/// - `order`: Uses OrderOption::Descending if None ("desc" in the URL)
/// - `per_page`: Uses 30 if None, caps at 100 (GitHub API limit)
/// - `page`: Uses 1 if None
/// - `query`: URL encoded to handle special characters
/// # Examples
///
/// ```
/// use gitcodes_mcp::gitcodes::repository_manager::providers::github::{GithubSearchParams, GithubSortOption, GithubOrderOption};
///
/// // Basic search with defaults
/// let params = GithubSearchParams {
///    query: "rust http client".to_string(),
///    sort_by: None,
///    order: None,
///    per_page: None,
///    page: None,
/// };
///
/// // Advanced search with custom options
/// let advanced_params = GithubSearchParams {
///    query: "language:rust stars:>1000".to_string(),
///    sort_by: Some(GithubSortOption::Stars),
///    order: Some(GithubOrderOption::Descending),
///    per_page: Some(50),
///    page: Some(2),
/// };
/// ```
#[derive(Debug, Serialize, Deserialize)]
pub struct GithubSearchParams {
    /// Sort parameter for search results
    /// When None, defaults to SortOption::Relevance (GitHub's default sorting)
    pub sort_by: Option<GithubSortOption>,

    /// Order parameter for sorting results (ascending or descending)
    /// When None, defaults to OrderOption::Descending
    pub order: Option<GithubOrderOption>,

    /// Number of results per page (1-100)
    /// When None, defaults to 30
    /// Values over 100 will be capped at 100 (GitHub API limit)
    pub per_page: Option<u8>,

    /// Page number for pagination (starts at 1)
    /// When None, defaults to 1
    pub page: Option<u32>,

    /// Search query for repositories
    /// This is the only required parameter
    /// Supports GitHub's search syntax, e.g., "language:rust stars:>1000"
    pub query: String,
}

pub struct GithubClient {
    client: Client,
    github_token: Option<String>,
}

impl GithubClient {
    pub fn new(client: Client, github_token: Option<String>) -> Self {
        GithubClient {
            client,
            github_token,
        }
    }

    /// Constructs the GitHub API URL for repository search
    ///
    /// Builds the complete URL with query parameters for the GitHub search API.
    /// This function handles parameter defaults, validation, and proper URL encoding.
    ///
    /// # Returns
    ///
    /// A fully formed URL string ready for HTTP request to GitHub's search API
    ///
    /// # Parameter Handling
    ///
    /// - `sort_by`: Uses SortOption::Relevance if None (empty string in the URL)
    /// - `order`: Uses OrderOption::Descending if None ("desc" in the URL)
    /// - `per_page`: Uses 30 if None, caps at 100 (GitHub API limit)
    /// - `page`: Uses 1 if None
    /// - `query`: URL encoded to handle special characters
    ///
    /// # Examples
    ///
    /// ```
    /// use gitcodes_mcp::gitcodes::repository_manager::providers::github::{GithubSearchParams, GithubSortOption, GithubOrderOption, GithubClient};
    ///
    /// let params = GithubSearchParams {
    ///     query: "rust web framework".to_string(),
    ///     sort_by: Some(GithubSortOption::Stars),
    ///     order: Some(GithubOrderOption::Descending),
    ///     per_page: Some(50),
    ///     page: Some(1),
    /// };
    ///
    /// // In the actual implementation, we'd call GithubClient::construct_search_url
    /// // Example URL: "https://api.github.com/search/repositories?q=rust%20web%20framework&sort=stars&order=desc&per_page=50&page=1"
    /// ```
    fn construct_search_url(params: &GithubSearchParams) -> String {
        // Set up sort parameter using Default implementation
        let default_sort = GithubSortOption::default();
        let sort = params.sort_by.as_ref().unwrap_or(&default_sort).to_str();

        // Set up order parameter using Default implementation
        let default_order = GithubOrderOption::default();
        let order = params.order.as_ref().unwrap_or(&default_order).to_str();

        // Set default values for pagination
        let per_page = params.per_page.unwrap_or(30).min(100); // GitHub API limit is 100
        let page = params.page.unwrap_or(1);

        let mut url = format!(
            "https://api.github.com/search/repositories?q={}",
            urlencoding::encode(&params.query)
        );

        if !sort.is_empty() {
            url.push_str(&format!("&sort={}", sort));
        }

        url.push_str(&format!("&order={}", order));
        url.push_str(&format!("&per_page={}&page={}", per_page, page));

        url
    }

    //TODO(tacogips) implthis
    pub async fn get_default_branch(&self, _target_repository: &GitRemoteRepositoryInfo) -> String {
        unimplemented!()
    }

    /// Executes a GitHub API search request
    ///
    /// Sends the HTTP request to the GitHub API and handles the response.
    pub async fn execute_search_request(&self, params: &GithubSearchParams) -> Result<String, String> {
        let url = Self::construct_search_url(params);
        // Set up the API request
        let mut req_builder = self.client.get(url).header(
            "User-Agent",
            "gitcodes-mcp/0.1.0 (https://github.com/tacogips/gitcodes-mcp)",
        );

        // Add authentication token if available
        if let Some(token) = &self.github_token.as_ref() {
            req_builder = req_builder.header("Authorization", format!("token {}", token));
        }

        // Execute API request
        let response = match req_builder.send().await {
            Ok(resp) => resp,
            Err(e) => return Err(format!("Failed to search repositories: {}", e)),
        };

        // Check if the request was successful
        let status = response.status();
        if !status.is_success() {
            let error_text = match response.text().await {
                Ok(text) => text,
                Err(_) => "Unknown error".to_string(),
            };

            return Err(format!("GitHub API error {}: {}", status, error_text));
        }

        // Return the raw JSON response
        match response.text().await {
            Ok(text) => Ok(text),
            Err(e) => Err(format!("Failed to read response body: {}", e)),
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
    pub async fn search_repositories(&self, params: GithubSearchParams) -> Result<String, String> {
        // Execute the search request
        self.execute_search_request(&params).await
    }

    /// List branches and tags for a GitHub repository using the GitHub API
    ///
    /// This method retrieves all references (branches and tags) for a specified repository
    /// using GitHub's Git References API endpoint.
    ///
    /// # Authentication
    ///
    /// - Uses the `GITCODE_MCP_GITHUB_TOKEN` if available for authentication
    /// - Without a token, limited to 60 requests/hour
    /// - With a token, allows 5,000 requests/hour
    ///
    /// # API References
    ///
    /// - https://docs.github.com/en/rest/git/refs?apiVersion=2022-11-28
    ///
    /// # Returns
    ///
    /// A JSON string containing all references in the repository, including branches and tags.
    /// The response includes ref names and their corresponding SHA values.
    pub async fn list_repository_refs(&self, repo_info: &GitRemoteRepositoryInfo) -> Result<String, String> {
        // Construct the API URL for listing refs
        let url_str = format!(
            "https://api.github.com/repos/{}/{}/git/refs",
            repo_info.user,
            repo_info.repo
        );

        // Parse the URL to ensure it's valid
        let url = url_str.parse::<reqwest::Url>()
            .map_err(|e| format!("Failed to parse GitHub API URL: {}", e))?;

        // Set up the API request
        let mut req_builder = self.client.get(url).header(
            "User-Agent",
            "gitcodes-mcp/0.1.0 (https://github.com/d6e/gitcodes-mcp)",
        );

        // Add authentication token if available
        if let Some(token) = &self.github_token.as_ref() {
            req_builder = req_builder.header("Authorization", format!("token {}", token));
        }

        // Execute API request
        let response = match req_builder.send().await {
            Ok(resp) => resp,
            Err(e) => return Err(format!("Failed to list repository refs: {}", e)),
        };

        // Check if the request was successful
        let status = response.status();
        if !status.is_success() {
            let error_text = match response.text().await {
                Ok(text) => text,
                Err(_) => "Unknown error".to_string(),
            };

            return Err(format!("GitHub API error {}: {}", status, error_text));
        }

        // Return the raw JSON response
        match response.text().await {
            Ok(text) => Ok(text),
            Err(e) => Err(format!("Failed to read response body: {}", e)),
        }
    }
}

/// Parse a GitHub URL to extract the user and repository name
///
/// This function handles various GitHub URL formats including:
/// - https://github.com/user/repo
/// - git@github.com:user/repo
/// - github:user/repo
///
/// # Parameters
///
/// * `url` - The GitHub URL to parse
///
/// # Returns
///
/// * `Result<GithubRemoteInfo, String>` - A GithubRemoteInfo object or an error message
pub(crate) fn parse_github_repository_url(url: &str) -> Result<GithubRemoteInfo, String> {
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

    let user = parts[0].to_string();
    let repo = parts[1].to_string();

    // Create RemoteGitRepositoryInfo with extracted user and repo
    let repo_info = GitRemoteRepositoryInfo {
        user,
        repo,
        ref_name: None, // Default to None for ref_name
    };

    // Create and return GithubRemoteInfo
    Ok(GithubRemoteInfo {
        clone_url: url.to_string(),
        repo_info,
    })
}
