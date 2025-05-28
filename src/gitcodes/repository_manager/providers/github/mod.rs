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

/// Sort options for GitHub issue search results
///
/// Controls how issue search results are ordered in the response.
#[derive(Debug, Serialize, Deserialize, Display, EnumString, AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub enum GithubIssueSortOption {
    /// Sort by creation date
    #[strum(serialize = "created")]
    Created,
    /// Sort by last update date
    #[strum(serialize = "updated")]
    Updated,
    /// Sort by number of comments
    #[strum(serialize = "comments")]
    Comments,
    /// Sort by relevance (GitHub's default)
    #[strum(serialize = "best-match")]
    BestMatch,
}

impl GithubIssueSortOption {
    /// Converts the sort option to its API string representation
    pub fn to_str(&self) -> &str {
        self.as_ref()
    }
}

impl Default for GithubIssueSortOption {
    /// Returns the default sort option (BestMatch)
    fn default() -> Self {
        GithubIssueSortOption::BestMatch
    }
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
    /// When None, defaults to 5
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

/// Search parameters for GitHub issue search
///
/// Contains all the parameters needed for configuring an issue search request to GitHub's API.
/// This struct handles both the parameter validation and URL construction for issue searches.
///
/// # Parameter Handling
///
/// - `sort_by`: Uses IssueSortOption::BestMatch if None (GitHub's default sorting)
/// - `order`: Uses OrderOption::Descending if None ("desc" in the URL)
/// - `per_page`: Uses 30 if None, caps at 100 (GitHub API limit)
/// - `page`: Uses 1 if None
/// - `query`: URL encoded to handle special characters
/// # Examples
///
/// ```
/// use gitcodes_mcp::gitcodes::repository_manager::providers::github::{GithubIssueSearchParams, GithubIssueSortOption, GithubOrderOption};
///
/// // Basic search with defaults
/// let params = GithubIssueSearchParams {
///    query: "bug in documentation".to_string(),
///    sort_by: None,
///    order: None,
///    per_page: None,
///    page: None,
///    repository: None,
///    labels: Some("bug".to_string()),
///    state: None,
///    creator: None,
///    mentioned: None,
///    assignee: None,
///    milestone: None,
///    issue_type: None,
///    advanced_search: None,
/// };
///
/// // Advanced search with custom options
/// let advanced_params = GithubIssueSearchParams {
///    query: "performance issue".to_string(),
///    sort_by: Some(GithubIssueSortOption::Updated),
///    order: Some(GithubOrderOption::Descending),
///    per_page: Some(50),
///    page: Some(1),
///    repository: Some("owner/repo".to_string()),
///    labels: Some("enhancement".to_string()),
///    state: Some("open".to_string()),
///    creator: None,
///    mentioned: None,
///    assignee: None,
///    milestone: None,
///    issue_type: None,
///    advanced_search: Some(true),
/// };
/// ```
#[derive(Debug, Serialize, Deserialize)]
pub struct GithubIssueSearchParams {
    /// Sort parameter for search results
    /// When None, defaults to IssueSortOption::BestMatch (GitHub's default sorting)
    pub sort_by: Option<GithubIssueSortOption>,

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

    /// Full-text search query for issues
    /// This should contain only the search terms, not qualifiers
    /// Example: "bug in documentation"
    pub query: String,

    /// Repository specification in the format "owner/repo"
    /// When specified, limits search to this specific repository
    /// Example: "octocat/Hello-World"
    pub repository: Option<String>,

    /// Labels to search for (comma-separated)
    /// Example: "bug,ui,@high"
    pub labels: Option<String>,

    /// State of issues to search for
    /// Can be "open", "closed", or "all"
    pub state: Option<String>,

    /// User who created the issue
    pub creator: Option<String>,

    /// User mentioned in the issue
    pub mentioned: Option<String>,

    /// User assigned to the issue
    /// Can be a username, "none" for unassigned, or "*" for any assignee
    pub assignee: Option<String>,

    /// Milestone number or special values
    /// Can be a number, "*" for any milestone, or "none" for no milestone
    pub milestone: Option<String>,

    /// Issue type name
    /// Can be a type name, "*" for any type, or "none" for no type
    pub issue_type: Option<String>,

    /// Use advanced search with GraphQL
    /// When true, uses GraphQL with ISSUE_ADVANCED type instead of REST API
    /// Default: false
    pub advanced_search: Option<bool>,
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

/// GitHub-specific issue search response structure
#[derive(Debug, Deserialize)]
struct GitHubIssueSearchResponse {
    total_count: u64,
    incomplete_results: bool,
    items: Vec<GitHubIssueItem>,
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
    //avatar_url: String,
    //html_url: String,
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
    //spdx_id: Option<String>,
    //url: Option<String>,
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

/// GitHub-specific issue item
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GitHubIssueItem {
    id: u64,
    number: u64,
    title: String,
    body: Option<String>,
    state: String,
    user: GitHubIssueUser,
    assignee: Option<GitHubIssueUser>,
    assignees: Vec<GitHubIssueUser>,
    labels: Vec<GitHubIssueLabel>,
    milestone: Option<GitHubIssueMilestone>,
    comments: u64,
    html_url: String,
    created_at: String,
    updated_at: String,
    closed_at: Option<String>,
    score: f64,
    repository_url: String,
}

/// GitHub-specific issue user
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GitHubIssueUser {
    login: String,
    id: u64,
    #[serde(rename = "type")]
    type_field: String,
    html_url: String,
}

/// GitHub-specific issue label
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GitHubIssueLabel {
    id: u64,
    name: String,
    color: String,
    description: Option<String>,
}

/// GitHub-specific issue milestone
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GitHubIssueMilestone {
    id: u64,
    number: u64,
    title: String,
    description: Option<String>,
    state: String,
    created_at: String,
    updated_at: String,
    due_on: Option<String>,
    closed_at: Option<String>,
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
    /// - `per_page`: Uses 5 if None, caps at 100 (GitHub API limit)
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
        let per_page = params.per_page.unwrap_or(5).min(100); // GitHub API limit is 100
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

    /// Constructs the GitHub API URL for issue search
    ///
    /// Builds the complete URL with query parameters for the GitHub issues search API.
    /// This function handles parameter defaults, validation, and proper URL encoding.
    ///
    /// # Returns
    ///
    /// A fully formed URL string ready for HTTP request to GitHub's issues search API
    ///
    /// # Parameter Handling
    ///
    /// - `sort_by`: Uses IssueSortOption::BestMatch if None (GitHub's default sorting)
    /// - `order`: Uses OrderOption::Descending if None ("desc" in the URL)
    /// - `per_page`: Uses 30 if None, caps at 100 (GitHub API limit)
    /// - `page`: Uses 1 if None
    /// - `query`: URL encoded to handle special characters
    fn construct_issue_search_url(params: &GithubIssueSearchParams) -> String {
        // Set up sort parameter using Default implementation
        let default_sort = GithubIssueSortOption::default();
        let sort = params.sort_by.as_ref().unwrap_or(&default_sort).to_str();

        // Set up order parameter using Default implementation
        let default_order = GithubOrderOption::default();
        let order = params.order.as_ref().unwrap_or(&default_order).to_str();

        // Set default values for pagination
        let per_page = params.per_page.unwrap_or(30).min(100); // GitHub API limit is 100
        let page = params.page.unwrap_or(1);

        // Build the search query with qualifiers
        let mut query_parts = vec![params.query.clone()];

        // Add repository qualifier if specified
        if let Some(repo) = &params.repository {
            query_parts.push(format!("repo:{}", repo));
        }

        // Add labels qualifier if specified
        if let Some(labels) = &params.labels {
            query_parts.push(format!("label:{}", labels));
        }

        // Add state qualifier if specified
        if let Some(state) = &params.state {
            query_parts.push(format!("state:{}", state));
        }

        // Add creator qualifier if specified
        if let Some(creator) = &params.creator {
            query_parts.push(format!("author:{}", creator));
        }

        // Add mentioned qualifier if specified
        if let Some(mentioned) = &params.mentioned {
            query_parts.push(format!("mentions:{}", mentioned));
        }

        // Add assignee qualifier if specified
        if let Some(assignee) = &params.assignee {
            query_parts.push(format!("assignee:{}", assignee));
        }

        // Add milestone qualifier if specified
        if let Some(milestone) = &params.milestone {
            query_parts.push(format!("milestone:{}", milestone));
        }

        // Add issue type qualifier if specified
        if let Some(issue_type) = &params.issue_type {
            query_parts.push(format!("type:{}", issue_type));
        }

        // Combine all query parts
        let full_query = query_parts.join(" ");

        let mut url = format!(
            "https://api.github.com/search/issues?q={}",
            urlencoding::encode(&full_query)
        );

        if !sort.is_empty() && sort != "best-match" {
            url.push_str(&format!("&sort={}", sort));
        }

        url.push_str(&format!("&order={}", order));
        url.push_str(&format!("&per_page={}&page={}", per_page, page));

        // Add advanced_search parameter if specified
        if let Some(advanced_search) = params.advanced_search {
            if advanced_search {
                url.push_str("&advanced_search=true");
            }
        }

        url
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

    /// Executes a GitHub API search issues request
    ///
    /// Sends the HTTP request to the GitHub API's issues search endpoint and handles the response.
    /// Returns a structured IssueSearchResults instead of raw JSON.
    pub async fn execute_search_issues_request(
        &self,
        params: &GithubIssueSearchParams,
    ) -> Result<super::models::IssueSearchResults, String> {
        let url = Self::construct_issue_search_url(params);
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
            Err(e) => return Err(format!("Failed to search issues: {}", e)),
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
        let github_response: GitHubIssueSearchResponse = match response.json().await {
            Ok(response) => response,
            Err(e) => return Err(format!("Failed to parse GitHub response: {}", e)),
        };

        // Convert to our common domain model
        let mut items = Vec::new();
        for github_item in github_response.items {
            // Extract repository information from repository_url
            let repo_parts: Vec<&str> = github_item
                .repository_url
                .trim_start_matches("https://api.github.com/repos/")
                .split('/')
                .collect();

            let repository = if repo_parts.len() >= 2 {
                super::IssueRepository {
                    id: "".to_string(), // Not available in search response
                    name: repo_parts[1].to_string(),
                    full_name: format!("{}/{}", repo_parts[0], repo_parts[1]),
                    owner: super::RepositoryOwner {
                        login: repo_parts[0].to_string(),
                        id: "".to_string(), // Not available in search response
                        type_field: "User".to_string(), // Default, not available in search response
                    },
                    private: false, // Not available in search response
                    html_url: format!("https://github.com/{}/{}", repo_parts[0], repo_parts[1]),
                    description: None, // Not available in search response
                }
            } else {
                // Fallback if URL parsing fails
                super::IssueRepository {
                    id: "".to_string(),
                    name: "unknown".to_string(),
                    full_name: "unknown/unknown".to_string(),
                    owner: super::RepositoryOwner {
                        login: "unknown".to_string(),
                        id: "".to_string(),
                        type_field: "User".to_string(),
                    },
                    private: false,
                    html_url: "".to_string(),
                    description: None,
                }
            };

            items.push(super::IssueItem {
                id: github_item.id.to_string(),
                number: github_item.number,
                title: github_item.title,
                body: github_item.body,
                state: github_item.state,
                user: super::IssueUser {
                    login: github_item.user.login,
                    id: github_item.user.id.to_string(),
                    type_field: github_item.user.type_field,
                    html_url: github_item.user.html_url,
                },
                assignee: github_item.assignee.map(|assignee| super::IssueUser {
                    login: assignee.login,
                    id: assignee.id.to_string(),
                    type_field: assignee.type_field,
                    html_url: assignee.html_url,
                }),
                assignees: github_item
                    .assignees
                    .into_iter()
                    .map(|assignee| super::IssueUser {
                        login: assignee.login,
                        id: assignee.id.to_string(),
                        type_field: assignee.type_field,
                        html_url: assignee.html_url,
                    })
                    .collect(),
                labels: github_item
                    .labels
                    .into_iter()
                    .map(|label| super::IssueLabel {
                        id: label.id.to_string(),
                        name: label.name,
                        color: label.color,
                        description: label.description,
                    })
                    .collect(),
                milestone: github_item
                    .milestone
                    .map(|milestone| super::IssueMilestone {
                        id: milestone.id.to_string(),
                        number: milestone.number,
                        title: milestone.title,
                        description: milestone.description,
                        state: milestone.state,
                        created_at: milestone.created_at,
                        updated_at: milestone.updated_at,
                        due_on: milestone.due_on,
                        closed_at: milestone.closed_at,
                    }),
                comments: github_item.comments,
                html_url: github_item.html_url,
                created_at: github_item.created_at,
                updated_at: github_item.updated_at,
                closed_at: github_item.closed_at,
                score: github_item.score,
                repository,
            });
        }

        // Return the common domain model
        Ok(super::IssueSearchResults {
            total_count: github_response.total_count,
            incomplete_results: github_response.incomplete_results,
            items,
        })
    }

    /// Search for GitHub issues using the GitHub API
    ///
    /// This method searches for issues on GitHub based on the provided query.
    /// It supports sorting, pagination, and uses GitHub's issues search API.
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
    ///
    /// # Search Query Syntax
    ///
    /// The query parameter supports GitHub's search syntax:
    /// - `repo:owner/name` - Search within a specific repository
    /// - `state:open` or `state:closed` - Filter by issue state
    /// - `label:bug` - Filter by label
    /// - `assignee:username` - Filter by assignee
    /// - `author:username` - Filter by author
    /// - `created:2021-01-01..2021-12-31` - Filter by creation date range
    /// - `updated:>2021-01-01` - Filter by last update date
    ///
    /// # Examples
    ///
    /// ```
    /// use gitcodes_mcp::gitcodes::repository_manager::providers::github::{GithubIssueSearchParams, GithubIssueSortOption, GithubOrderOption};
    ///
    /// // Search for open bugs in a specific repository
    /// let params = GithubIssueSearchParams {
    ///     query: "memory leak".to_string(),
    ///     sort_by: Some(GithubIssueSortOption::Updated),
    ///     order: Some(GithubOrderOption::Descending),
    ///     per_page: Some(10),
    ///     page: Some(1),
    ///     repository: Some("rust-lang/rust".to_string()),
    ///     labels: Some("bug".to_string()),
    ///     state: Some("open".to_string()),
    ///     creator: None,
    ///     mentioned: None,
    ///     assignee: None,
    ///     milestone: None,
    ///     issue_type: None,
    ///     advanced_search: None,
    /// };
    /// ```
    pub async fn search_issues(
        &self,
        params: GithubIssueSearchParams,
    ) -> Result<super::models::IssueSearchResults, String> {
        // Check if advanced search is enabled
        if params.advanced_search.unwrap_or(false) {
            self.execute_graphql_search_issues_request(&params).await
        } else {
            self.execute_search_issues_request(&params).await
        }
    }

    /// Execute GraphQL search for issues using ISSUE_ADVANCED type
    ///
    /// This method is used when advanced_search is enabled and sends GraphQL queries
    /// to GitHub's GraphQL API endpoint using the ISSUE_ADVANCED search type.
    async fn execute_graphql_search_issues_request(
        &self,
        params: &GithubIssueSearchParams,
    ) -> Result<super::models::IssueSearchResults, String> {
        // Build the search query with qualifiers (same as REST API)
        let mut query_parts = vec![params.query.clone()];

        // Add repository qualifier if specified
        if let Some(repo) = &params.repository {
            query_parts.push(format!("repo:{}", repo));
        }

        // Add labels qualifier if specified
        if let Some(labels) = &params.labels {
            query_parts.push(format!("label:{}", labels));
        }

        // Add state qualifier if specified
        if let Some(state) = &params.state {
            query_parts.push(format!("state:{}", state));
        }

        // Add creator qualifier if specified
        if let Some(creator) = &params.creator {
            query_parts.push(format!("author:{}", creator));
        }

        // Add mentioned qualifier if specified
        if let Some(mentioned) = &params.mentioned {
            query_parts.push(format!("mentions:{}", mentioned));
        }

        // Add assignee qualifier if specified
        if let Some(assignee) = &params.assignee {
            query_parts.push(format!("assignee:{}", assignee));
        }

        // Add milestone qualifier if specified
        if let Some(milestone) = &params.milestone {
            query_parts.push(format!("milestone:{}", milestone));
        }

        // Add issue type qualifier if specified
        if let Some(issue_type) = &params.issue_type {
            query_parts.push(format!("type:{}", issue_type));
        }

        // Combine all query parts
        let full_query = query_parts.join(" ");

        // Set up pagination parameters
        let per_page = params.per_page.unwrap_or(30).min(100); // GitHub API limit is 100

        // Construct GraphQL query
        let graphql_query = format!(
            r#"
            query {{
                search(query: "{}", type: ISSUE_ADVANCED, first: {}) {{
                    issueCount
                    nodes {{
                        ... on Issue {{
                            id
                            number
                            title
                            body
                            state
                            createdAt
                            updatedAt
                            closedAt
                            url
                            author {{
                                login
                                ... on User {{
                                    id
                                }}
                            }}
                            labels(first: 50) {{
                                nodes {{
                                    id
                                    name
                                    color
                                    description
                                }}
                            }}
                            assignees(first: 10) {{
                                nodes {{
                                    login
                                    id
                                }}
                            }}
                            milestone {{
                                id
                                number
                                title
                                description
                                state
                                createdAt
                                updatedAt
                                dueOn
                                closedAt
                            }}
                            repository {{
                                name
                                nameWithOwner
                                url
                            }}
                            comments {{
                                totalCount
                            }}
                        }}
                    }}
                }}
            }}
            "#,
            full_query.replace('"', r#"\""#), // Escape quotes in the query
            per_page
        );

        // Create GraphQL request payload
        let graphql_payload = serde_json::json!({
            "query": graphql_query
        });

        // Set up request headers
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_static("gitcodes-mcp/0.1.0"),
        );

        // Add authentication header if token is available
        if let Some(token) = &self.github_token {
            let auth_value = format!("Bearer {}", token);
            headers.insert(
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&auth_value)
                    .map_err(|e| format!("Invalid auth header: {}", e))?,
            );
        }

        // Send GraphQL request to GitHub's GraphQL API
        let response = self
            .client
            .post("https://api.github.com/graphql")
            .headers(headers)
            .json(&graphql_payload)
            .send()
            .await
            .map_err(|e| format!("GraphQL request failed: {}", e))?;

        // Check if the request was successful
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(format!("GraphQL request failed with status {}: {}", status, error_text));
        }

        // Parse the GraphQL response
        let graphql_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse GraphQL response: {}", e))?;

        // Check for GraphQL errors
        if let Some(errors) = graphql_response.get("errors") {
            return Err(format!("GraphQL query errors: {}", errors));
        }

        // Extract search results from GraphQL response
        let search_data = graphql_response
            .get("data")
            .and_then(|data| data.get("search"))
            .ok_or("Missing search data in GraphQL response")?;

        let total_count = search_data
            .get("issueCount")
            .and_then(|count| count.as_u64())
            .unwrap_or(0) as u32;

        let empty_vec = vec![];
        let nodes = search_data
            .get("nodes")
            .and_then(|nodes| nodes.as_array())
            .unwrap_or(&empty_vec);

        // Convert GraphQL response to our issue format
        let mut issues = Vec::new();
        for node in nodes {
            if let Ok(issue) = self.parse_graphql_issue_node(node) {
                issues.push(issue);
            }
        }

        Ok(super::models::IssueSearchResults {
            total_count: total_count as u64,
            incomplete_results: false, // GraphQL doesn't provide this field
            items: issues,
        })
    }

    /// Parse a GraphQL issue node into our Issue format
    fn parse_graphql_issue_node(&self, node: &serde_json::Value) -> Result<super::models::IssueItem, String> {
        let id = node
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or("Missing issue id")?
            .to_string();

        let number = node
            .get("number")
            .and_then(|v| v.as_u64())
            .ok_or("Missing issue number")?;

        let title = node
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let body = node
            .get("body")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let state = node
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("open")
            .to_string();

        let html_url = node
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let created_at = node
            .get("createdAt")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let updated_at = node
            .get("updatedAt")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let closed_at = node
            .get("closedAt")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Parse author
        let user = if let Some(author) = node.get("author") {
            super::models::IssueUser {
                login: author
                    .get("login")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                id: author
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                type_field: "User".to_string(),
                html_url: format!("https://github.com/{}", 
                    author.get("login").and_then(|v| v.as_str()).unwrap_or("")),
            }
        } else {
            super::models::IssueUser {
                login: "unknown".to_string(),
                id: "unknown".to_string(),
                type_field: "User".to_string(),
                html_url: "".to_string(),
            }
        };

        // Parse labels
        let labels = if let Some(labels_data) = node.get("labels").and_then(|l| l.get("nodes")).and_then(|n| n.as_array()) {
            labels_data
                .iter()
                .filter_map(|label| {
                    Some(super::models::IssueLabel {
                        id: label.get("id").and_then(|v| v.as_str())?.to_string(),
                        name: label.get("name").and_then(|v| v.as_str())?.to_string(),
                        color: label.get("color").and_then(|v| v.as_str())?.to_string(),
                        description: label.get("description").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    })
                })
                .collect()
        } else {
            vec![]
        };

        // Parse assignees
        let assignees = if let Some(assignees_data) = node.get("assignees").and_then(|a| a.get("nodes")).and_then(|n| n.as_array()) {
            assignees_data
                .iter()
                .filter_map(|assignee| {
                    Some(super::models::IssueUser {
                        login: assignee.get("login").and_then(|v| v.as_str())?.to_string(),
                        id: assignee.get("id").and_then(|v| v.as_str())?.to_string(),
                        type_field: "User".to_string(),
                        html_url: format!("https://github.com/{}", 
                            assignee.get("login").and_then(|v| v.as_str())?),
                    })
                })
                .collect()
        } else {
            vec![]
        };

        // Parse milestone
        let milestone = if let Some(milestone_data) = node.get("milestone") {
            Some(super::models::IssueMilestone {
                id: milestone_data
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                number: milestone_data
                    .get("number")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0),
                title: milestone_data
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                description: milestone_data
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                state: milestone_data
                    .get("state")
                    .and_then(|v| v.as_str())
                    .unwrap_or("open")
                    .to_string(),
                created_at: milestone_data
                    .get("createdAt")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                updated_at: milestone_data
                    .get("updatedAt")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                due_on: milestone_data
                    .get("dueOn")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                closed_at: milestone_data
                    .get("closedAt")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            })
        } else {
            None
        };

        // Parse repository information
        let repository = if let Some(repo_data) = node.get("repository") {
            super::models::IssueRepository {
                id: repo_data.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                name: repo_data.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                full_name: repo_data.get("nameWithOwner").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                owner: super::models::RepositoryOwner {
                    login: repo_data.get("nameWithOwner")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .split('/')
                        .next()
                        .unwrap_or("")
                        .to_string(),
                    id: "".to_string(),
                    type_field: "User".to_string(),
                },
                private: false,
                html_url: repo_data.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                description: None,
            }
        } else {
            super::models::IssueRepository {
                id: "".to_string(),
                name: "".to_string(),
                full_name: "".to_string(),
                owner: super::models::RepositoryOwner {
                    login: "".to_string(),
                    id: "".to_string(),
                    type_field: "User".to_string(),
                },
                private: false,
                html_url: "".to_string(),
                description: None,
            }
        };

        // Parse comments count
        let comments = node
            .get("comments")
            .and_then(|c| c.get("totalCount"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        Ok(super::models::IssueItem {
            id,
            number,
            title,
            body,
            state,
            user,
            assignee: assignees.first().cloned(),
            assignees,
            labels,
            milestone,
            comments,
            html_url,
            created_at,
            updated_at,
            closed_at,
            score: 1.0, // GraphQL doesn't provide score
            repository,
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_construct_issue_search_url_basic() {
        let params = GithubIssueSearchParams {
            query: "memory leak".to_string(),
            sort_by: None,
            order: None,
            per_page: None,
            page: None,
            repository: None,
            labels: None,
            state: None,
            creator: None,
            mentioned: None,
            assignee: None,
            milestone: None,
            issue_type: None,
            advanced_search: None,
        };

        let url = GithubClient::construct_issue_search_url(&params);
        assert!(url.contains("q=memory%20leak"));
        assert!(url.contains("&order=desc"));
        assert!(url.contains("&per_page=30"));
        assert!(url.contains("&page=1"));
    }

    #[test]
    fn test_construct_issue_search_url_with_qualifiers() {
        let params = GithubIssueSearchParams {
            query: "performance issue".to_string(),
            sort_by: Some(GithubIssueSortOption::Updated),
            order: Some(GithubOrderOption::Ascending),
            per_page: Some(50),
            page: Some(2),
            repository: Some("rust-lang/rust".to_string()),
            labels: Some("bug,performance".to_string()),
            state: Some("open".to_string()),
            creator: Some("octocat".to_string()),
            mentioned: Some("maintainer".to_string()),
            assignee: Some("developer".to_string()),
            milestone: Some("1".to_string()),
            issue_type: Some("enhancement".to_string()),
            advanced_search: None,
        };

        let url = GithubClient::construct_issue_search_url(&params);
        
        // The URL should be properly encoded and contain all qualifiers
        assert!(url.contains("performance%20issue"));
        assert!(url.contains("repo%3Arust-lang%2Frust"));
        assert!(url.contains("label%3Abug%2Cperformance"));
        assert!(url.contains("state%3Aopen"));
        assert!(url.contains("author%3Aoctocat"));
        assert!(url.contains("mentions%3Amaintainer"));
        assert!(url.contains("assignee%3Adeveloper"));
        assert!(url.contains("milestone%3A1"));
        assert!(url.contains("type%3Aenhancement"));
        assert!(url.contains("&sort=updated"));
        assert!(url.contains("&order=asc"));
        assert!(url.contains("&per_page=50"));
        assert!(url.contains("&page=2"));
    }

    #[test]
    fn test_construct_issue_search_url_separates_text_from_qualifiers() {
        let params = GithubIssueSearchParams {
            query: "documentation".to_string(),
            sort_by: None,
            order: None,
            per_page: None,
            page: None,
            repository: Some("owner/repo".to_string()),
            labels: Some("docs".to_string()),
            state: None,
            creator: None,
            mentioned: None,
            assignee: None,
            milestone: None,
            issue_type: None,
            advanced_search: None,
        };

        let url = GithubClient::construct_issue_search_url(&params);
        
        // Ensure text search and qualifiers are properly combined
        assert!(url.contains("documentation"));
        assert!(url.contains("repo%3Aowner%2Frepo"));
        assert!(url.contains("label%3Adocs"));
        // Should not contain bare qualifiers in the search text
        assert!(!url.contains("repo:owner/repo"));
        assert!(!url.contains("label:docs"));
    }
}
