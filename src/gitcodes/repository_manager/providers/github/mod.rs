use reqwest::Client;
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumString};

use crate::gitcodes::repository_manager::providers::GitRemoteRepositoryInfo;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct GithubRemoteInfo {
    pub clone_url: String,
    pub repo_info: GitRemoteRepositoryInfo,
}

impl GithubRemoteInfo {
    /// Converts the repository URL to SSH format (git@github.com:user/repo.git) to avoid HTTPS URL handling issues with gitoxide
    ///
    /// This method transforms any GitHub URL (HTTPS or other format) into the standard SSH URL format,
    /// which is more reliable when working with gitoxide for fetch/clone operations due to avoiding HTTP redirect issues.
    ///
    /// # Returns
    ///
    /// A String containing the SSH format URL for the GitHub repository (git@github.com:user/repo.git)
    ///
    /// # Examples
    ///
    /// ```
    /// use gitcodes_mcp::gitcodes::repository_manager::providers::github::parse_github_url;
    ///
    /// let remote_info = parse_github_url("https://github.com/BurntSushi/ripgrep").unwrap();
    /// assert_eq!(remote_info.to_ssh_url(), "git@github.com:BurntSushi/ripgrep.git");
    /// ```
    pub fn to_ssh_url(&self) -> String {
        // Construct SSH URL using the user and repo from the repo_info
        format!(
            "git@github.com:{}/{}.git",
            self.repo_info.user, self.repo_info.repo
        )
    }
}

/// Sort options for GitHub repository search results
///
/// Controls how repository search results are ordered in the response.
#[derive(Debug, Serialize, Deserialize, Display, EnumString, AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub enum GithubSortOption {
    /// No specific sort, use GitHub's default relevance sorting
    #[strum(serialize = "relevance")]
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

/// GitHub-specific search response structure
#[derive(Debug, Deserialize)]
struct GitHubRepositorySearchResponse {
    total_count: u64,
    incomplete_results: bool,
    items: Vec<GitHubRepositoryItem>,
}

/// GitHub-specific repository item
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GitHubRepositoryItem {
    id: u64,
    // node_id: String,
    name: String,
    full_name: String,
    private: bool,
    owner: GitHubRepositoryOwner,
    html_url: String,
    description: Option<String>,
    fork: bool,
    homepage: Option<String>,
    size: u64,
    stargazers_count: u64,
    watchers_count: u64,
    language: Option<String>,
    // has_issues: bool,
    // has_projects: bool,
    // has_downloads: bool,
    // has_wiki: bool,
    // has_pages: bool,
    forks_count: u64,
    archived: bool,
    // disabled: bool,
    open_issues_count: u64,
    license: Option<GitHubRepositoryLicense>,
    topics: Option<Vec<String>>,
    default_branch: String,
    score: f64,
    created_at: String,
    updated_at: String,
    pushed_at: String,
}

/// GitHub-specific owner information
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GitHubRepositoryOwner {
    login: String,
    id: u64,
    // node_id: String,
    avatar_url: String,
    html_url: String,
    #[serde(rename = "type")]
    type_field: String,
    // site_admin: bool,
}

/// GitHub-specific license information
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GitHubRepositoryLicense {
    key: String,
    name: String,
    spdx_id: Option<String>,
    url: Option<String>,
    // node_id: String,
}

/// GitHub reference type returned by the API
#[derive(Debug, Clone, Deserialize)]
struct GitHubRefObject {
    /// The fully qualified name of the reference (e.g., "refs/heads/main")
    #[serde(rename = "ref")]
    ref_name: String,
    /// The target object that this reference points to
    object: GitHubRefTarget,
}

/// GitHub ref target object
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct GitHubRefTarget {
    /// The SHA1 hash of the target object
    sha: String,
    /// The type of the target object, usually "commit"
    r#type: String,
    /// URL of the target object
    url: String,
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

    /// Executes a GitHub API search repository request
    ///
    /// Sends the HTTP request to the GitHub API's repository search endpoint and handles the response.
    /// Returns a structured RepositorySearchResults instead of raw JSON.
    pub async fn execute_search_repository_request(
        &self,
        params: &GithubSearchParams,
    ) -> Result<super::RepositorySearchResults, String> {
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

        // Deserialize the response into GitHub-specific types
        let github_response: GitHubRepositorySearchResponse = match response.json().await {
            Ok(response) => response,
            Err(e) => return Err(format!("Failed to parse GitHub response: {}", e)),
        };

        // Convert to our common domain model
        let mut items = Vec::new();
        for github_item in github_response.items {
            items.push(super::RepositoryItem {
                id: github_item.id.to_string(),
                name: github_item.name,
                full_name: github_item.full_name,
                private: github_item.private,
                owner: super::RepositoryOwner {
                    login: github_item.owner.login,
                    id: github_item.owner.id.to_string(),
                    avatar_url: github_item.owner.avatar_url,
                    html_url: github_item.owner.html_url,
                    type_field: github_item.owner.type_field,
                },
                html_url: github_item.html_url,
                description: github_item.description,
                fork: github_item.fork,
                homepage: github_item.homepage,
                size: github_item.size,
                stargazers_count: github_item.stargazers_count,
                watchers_count: github_item.watchers_count,
                language: github_item.language,
                forks_count: github_item.forks_count,
                archived: github_item.archived,
                open_issues_count: github_item.open_issues_count,
                license: github_item.license.map(|license| super::RepositoryLicense {
                    key: license.key,
                    name: license.name,
                    spdx_id: license.spdx_id,
                    url: license.url,
                }),
                topics: github_item.topics.unwrap_or_default(),
                default_branch: github_item.default_branch,
                score: github_item.score,
                created_at: github_item.created_at,
                updated_at: github_item.updated_at,
                pushed_at: github_item.pushed_at,
            });
        }

        // Return the common domain model
        Ok(super::RepositorySearchResults {
            total_count: github_response.total_count,
            incomplete_results: github_response.incomplete_results,
            items,
        })
    }

    /// Search for GitHub repositories using the GitHub API
    ///
    /// This method searches for repositories on GitHub based on the provided query.
    /// It supports sorting, pagination, and uses GitHub's search API.
    ///
    /// # Authentication
    ///
    /// - Uses the `GITCODES_MCP_GITHUB_TOKEN` if available for authentication
    /// - Without a token, limited to 60 requests/hour
    /// - With a token, allows 5,000 requests/hour
    ///
    /// # Rate Limiting
    ///
    /// GitHub API has rate limits that vary based on authentication:
    /// - Unauthenticated: 60 requests/hour
    /// - Authenticated: 5,000 requests/hour
    pub async fn search_repositories(
        &self,
        params: GithubSearchParams,
    ) -> Result<super::RepositorySearchResults, String> {
        // Execute the search repository request
        self.execute_search_repository_request(&params).await
    }

    /// List branches and tags for a GitHub repository using the GitHub API
    ///
    /// This method retrieves all references (branches and tags) for a specified repository
    /// using GitHub's Git References API endpoint.
    ///
    /// # Authentication
    ///
    /// - Uses the `GITCODES_MCP_GITHUB_TOKEN` if available for authentication
    /// - Without a token, limited to 60 requests/hour
    /// - With a token, allows 5,000 requests/hour
    ///
    /// # API References
    ///
    /// - [GitHub API: Git References](https://docs.github.com/en/rest/git/refs?apiVersion=2022-11-28)
    ///
    /// # Returns
    ///
    /// A structured RepositoryRefs object containing branches and tags.
    pub async fn list_repository_refs(
        &self,
        repo_info: &GitRemoteRepositoryInfo,
    ) -> Result<super::RepositoryRefs, String> {
        // Construct the API URL for listing refs
        let url_str = format!(
            "https://api.github.com/repos/{}/{}/git/refs",
            repo_info.user, repo_info.repo
        );

        // Parse the URL to ensure it's valid
        let url = url_str
            .parse::<reqwest::Url>()
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

        // Deserialize the JSON array of GitHub references
        let github_refs: Vec<GitHubRefObject> = match response.json().await {
            Ok(refs) => refs,
            Err(e) => return Err(format!("Failed to parse GitHub response: {}", e)),
        };

        // Transform into our domain model structure
        let mut branches = Vec::new();
        let mut tags = Vec::new();

        for ref_obj in github_refs {
            // Create a ReferenceInfo object
            let ref_info = super::ReferenceInfo {
                // Extract short name from full ref path
                name: ref_obj
                    .ref_name
                    .split('/')
                    .last()
                    .unwrap_or(&ref_obj.ref_name)
                    .to_string(),
                full_ref: ref_obj.ref_name.clone(),
                commit_id: ref_obj.object.sha,
            };

            // Sort into branches and tags based on path
            if ref_obj.ref_name.starts_with("refs/heads/") {
                branches.push(ref_info);
            } else if ref_obj.ref_name.starts_with("refs/tags/") {
                tags.push(ref_info);
            }
            // Ignore other ref types like refs/remotes
        }

        // Return the common domain model
        Ok(super::RepositoryRefs { branches, tags })
    }
}

/// Parse a GitHub URL to extract the user and repository name
///
/// This function handles various GitHub URL formats including:
/// - `https://github.com/user/repo`
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
///
/// # Examples
///
/// ```rust
/// use gitcodes_mcp::gitcodes::repository_manager::providers::github::parse_github_url;
///
/// let github_info = parse_github_url("https://github.com/user/repo").unwrap();
/// assert_eq!(github_info.repo_info.user, "user");
/// assert_eq!(github_info.repo_info.repo, "repo");
/// assert_eq!(github_info.to_ssh_url(), "git@github.com:user/repo.git");
/// ```
pub fn parse_github_url(url: &str) -> Result<GithubRemoteInfo, String> {
    parse_github_repository_url_internal(url)
}

/// Internal implementation of GitHub URL parsing
///
/// This function should not be called directly outside the crate.
pub(crate) fn parse_github_repository_url_internal(url: &str) -> Result<GithubRemoteInfo, String> {
    // Handle various GitHub URL formats
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
    } else if url.starts_with("git@github.com:") {
        url.trim_start_matches("git@github.com:")
            .trim_end_matches(".git")
            .trim_end_matches('/')
            .to_string()
    } else if url.starts_with("github:") {
        url.trim_start_matches("github:")
            .trim_start_matches('/')
            .trim_end_matches('/')
            .trim_end_matches(".git")
            .to_string()
    } else {
        return Err("Invalid GitHub repository URL format".to_string());
    };

    let parts: Vec<&str> = user_repo.split('/').collect();
    if parts.len() != 2 {
        return Err("Invalid GitHub repository URL format".to_string());
    }

    let user = parts[0].to_string();
    let repo = parts[1].to_string();

    // Clone user and repo for URL generation (avoid ownership issues)
    let user_clone = user.clone();
    let repo_clone = repo.clone();

    // Create RemoteGitRepositoryInfo with extracted user and repo
    let repo_info = GitRemoteRepositoryInfo {
        user,
        repo,
        ref_name: None, // Default to None for ref_name
    };

    // Generate proper clone URL with .git suffix for GitHub URLs
    // We prefer HTTPS URLs as the standard format in GithubRemoteInfo
    // but will use SSH for actual cloning via the to_ssh_url method
    let original_url = url.to_string();
    let clone_url = if url.starts_with("https://github.com") {
        if url.ends_with(".git") {
            tracing::debug!("URL already has .git suffix: {}", url);
            url.to_string() // Already has .git suffix
        } else {
            let new_url = format!("https://github.com/{}/{}.git", user_clone, repo_clone);
            tracing::debug!("Adding .git suffix to URL: {} -> {}", url, new_url);
            new_url // Add .git suffix
        }
    } else if url.starts_with("git@github.com:") {
        // Keep SSH URLs as they are for users who prefer them
        tracing::debug!("Using original SSH URL format: {}", url);
        url.to_string()
    } else if url.starts_with("github:") {
        // Convert github:user/repo to https://github.com/user/repo.git
        let new_url = format!("https://github.com/{}/{}.git", user_clone, repo_clone);
        tracing::debug!(
            "Converting github: URL to HTTPS with .git suffix: {} -> {}",
            url,
            new_url
        );
        new_url
    } else {
        tracing::debug!("Using original URL format: {}", url);
        url.to_string() // Keep original URL for other formats
    };

    tracing::info!(
        "Repository URL transformation: {} -> {}",
        original_url,
        clone_url
    );

    // Create and return GithubRemoteInfo
    Ok(GithubRemoteInfo {
        clone_url,
        repo_info,
    })
}
