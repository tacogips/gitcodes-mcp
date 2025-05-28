use reqwest::Client;
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumString};

use crate::gitcodes::repository_manager::providers::GitRemoteRepositoryInfo;

// Octocrab-based client
pub mod octocrab_client;
pub use octocrab_client::OctocrabGithubClient;

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
/// GitHub issue search parameters
///
/// This struct represents the parameters for searching issues in GitHub repositories.
/// It provides a clean separation between full-text search queries and GitHub-specific
/// qualifiers, making it easier to construct complex search queries programmatically.
///
/// The query field should contain only the search terms, while repository-specific
/// qualifiers like labels, state, etc. are specified through dedicated fields.
/// This design follows GitHub's search API structure and provides type safety.
///
/// ## Search Methods
///
/// ### GraphQL (Default)
/// When `advanced_search` is `None` or `Some(true)`, uses GitHub's GraphQL API:
/// - Endpoint: `https://api.github.com/graphql`
/// - Uses `ISSUE_ADVANCED` search type
/// - Supports complex boolean logic (AND, OR, parentheses)
/// - Better performance through precise field selection
/// - Current default behavior
///
/// ### REST API (Legacy)
/// When `advanced_search` is `Some(false)`, uses GitHub's REST API:
/// - Endpoint: `https://api.github.com/search/issues`
/// - Standard search syntax
/// - Limited boolean operations
/// - Provides relevance scores
///
/// ## Advanced Search Examples
///
/// The GraphQL mode supports complex queries like:
/// - `"memory leak AND (label:bug OR label:performance)"`
/// - `"assignee:@me AND (created:>2024-01-01 OR updated:>2024-06-01)"`
/// - `"(state:open OR state:closed) AND mentions:security"`
///
/// # Examples
///
/// ```
/// use gitcodes_mcp::gitcodes::repository_manager::providers::github::{GithubIssueSearchParams, GithubIssueSortOption, GithubOrderOption};
///
/// // Basic GraphQL search (default)
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
///    advanced_search: None, // Uses default (GraphQL)
/// };
///
/// // Advanced GraphQL search with boolean operations
/// let advanced_params = GithubIssueSearchParams {
///    query: "performance AND (memory OR cpu)".to_string(),
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
///    advanced_search: Some(true), // Explicitly enables GraphQL
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
    /// This enables support for advanced search syntax like AND, OR operators and nested queries
    /// Note: After September 4, 2025, this will become the default behavior
    /// Default: true
    pub advanced_search: Option<bool>,
}

pub struct GithubClient {
    octocrab_client: OctocrabGithubClient,
}





impl GithubClient {
    pub fn new(_client: Client, github_token: Option<String>) -> Result<Self, String> {
        let octocrab_client = OctocrabGithubClient::new(github_token)?;
        Ok(GithubClient {
            octocrab_client,
        })
    }

    /// Search for GitHub repositories using the GitHub API
    ///
    /// This method executes a search query against GitHub's repository search endpoint
    /// and returns structured results that are part of the common domain model.
    ///
    /// # Authentication
    ///
    /// - Uses the `GITCODES_MCP_GITHUB_TOKEN` if available for authentication
    /// - Unauthenticated requests have lower rate limits
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
        self.octocrab_client.search_repositories(params).await
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
        self.octocrab_client.list_repository_refs(repo_info).await
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
    /// Search for issues in a GitHub repository
    ///
    /// /// This method sends a request to GitHub's search issues API to find issues
    /// matching the specified criteria. It uses GraphQL by default for enhanced
    /// search capabilities, with optional fallback to REST API.
    ///
    /// By default (advanced_search: None or true), the method uses GitHub's GraphQL API with
    /// the ISSUE_ADVANCED type, which supports complex queries with AND/OR operators
    /// and nested searches. Set advanced_search to false to use the legacy REST API.
    ///
    /// # Examples
    ///
    /// ```
    /// use gitcodes_mcp::gitcodes::repository_manager::providers::github::{GithubIssueSearchParams, GithubIssueSortOption, GithubOrderOption};
    ///
    /// // Search for open bugs in a specific repository using GraphQL (default)
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
    ///     advanced_search: None, // Uses default (GraphQL)
    /// };
    /// ```
    pub async fn search_issues(
        &self,
        params: GithubIssueSearchParams,
    ) -> Result<super::models::IssueSearchResults, String> {
        self.octocrab_client.search_issues(params).await
    }

    /// Execute GraphQL search for issues using ISSUE_ADVANCED type
    ///
    /// This method is used when advanced_search is enabled and sends GraphQL queries
    /// to GitHub's GraphQL API endpoint using the ISSUE_ADVANCED search type.
    ///
    /// The GraphQL approach provides several advantages over the REST API:
    /// - Support for complex boolean logic with AND, OR operators
    /// - Nested search queries with parentheses
    /// - More precise field selection for better performance
    /// - Future-proof as GitHub plans to make this the default after September 4, 2025
    ///
    /// Example query transformations:
    /// - REST: "bug AND (label:urgent OR assignee:@me)"
    /// - GraphQL: Uses the same syntax but processes it through the ISSUE_ADVANCED type
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
        let _graphql_payload = serde_json::json!({
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
        headers.insert(
            reqwest::header::HeaderName::from_static("x-github-api-version"),
            reqwest::header::HeaderValue::from_static("2022-11-28"),
        );

        // Add authentication header if token is available
        // TODO: This method is deprecated and should be removed
        // if let Some(token) = &self.github_token {
        //     let auth_value = format!("Bearer {}", token);
        //     headers.insert(
        //         reqwest::header::AUTHORIZATION,
        //         reqwest::header::HeaderValue::from_str(&auth_value)
        //             .map_err(|e| format!("Invalid auth header: {}", e))?,
        //     );
        // }

        // GraphQL method is deprecated - return error for now
        return Err("GraphQL search method is deprecated. Use octocrab-based search instead.".to_string());
    }

    // Method body commented out for now
    /*
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
            return Err(format!(
                "GraphQL request failed with status {}: {}",
                status, error_text
            ));
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
    fn parse_graphql_issue_node(
        &self,
        node: &serde_json::Value,
    ) -> Result<super::models::IssueItem, String> {
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
                html_url: format!(
                    "https://github.com/{}",
                    author.get("login").and_then(|v| v.as_str()).unwrap_or("")
                ),
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
        let labels = if let Some(labels_data) = node
            .get("labels")
            .and_then(|l| l.get("nodes"))
            .and_then(|n| n.as_array())
        {
            labels_data
                .iter()
                .filter_map(|label| {
                    Some(super::models::IssueLabel {
                        id: label.get("id").and_then(|v| v.as_str())?.to_string(),
                        name: label.get("name").and_then(|v| v.as_str())?.to_string(),
                        color: label.get("color").and_then(|v| v.as_str())?.to_string(),
                        description: label
                            .get("description")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                    })
                })
                .collect()
        } else {
            vec![]
        };

        // Parse assignees
        let assignees = if let Some(assignees_data) = node
            .get("assignees")
            .and_then(|a| a.get("nodes"))
            .and_then(|n| n.as_array())
        {
            assignees_data
                .iter()
                .filter_map(|assignee| {
                    Some(super::models::IssueUser {
                        login: assignee.get("login").and_then(|v| v.as_str())?.to_string(),
                        id: assignee.get("id").and_then(|v| v.as_str())?.to_string(),
                        type_field: "User".to_string(),
                        html_url: format!(
                            "https://github.com/{}",
                            assignee.get("login").and_then(|v| v.as_str())?
                        ),
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
                id: repo_data
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                name: repo_data
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                full_name: repo_data
                    .get("nameWithOwner")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                owner: super::models::RepositoryOwner {
                    login: repo_data
                        .get("nameWithOwner")
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
                html_url: repo_data
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
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
            score: None, // GraphQL doesn't provide score
            repository,
        })
    }
    */
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


