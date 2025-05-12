use super::git_repository::RepositoryLocation;
use rmcp::schemars;
use strum::{AsRefStr, Display, EnumString};

/// Search parameters for GitHub repository search
///
/// Contains all the parameters needed for configuring a repository search request to GitHub's API.
/// This struct handles both the parameter validation and URL construction for repository searches.
///
/// # Examples
///
/// ```
/// use gitcodes_mcp::tools::gitcodes::git_service::params::{SearchParams, SortOption, OrderOption};
///
/// // Basic search with defaults
/// let params = SearchParams {
///    query: "rust http client".to_string(),
///    sort_by: None,
///    order: None,
///    per_page: None,
///    page: None,
/// };
///
/// // Advanced search with custom options
/// let advanced_params = SearchParams {
///    query: "language:rust stars:>1000".to_string(),
///    sort_by: Some(SortOption::Stars),
///    order: Some(OrderOption::Descending),
///    per_page: Some(50),
///    page: Some(2),
/// };
/// ```
#[derive(Debug, schemars::JsonSchema, serde::Serialize, serde::Deserialize)]
pub struct SearchParams {
    /// Sort parameter for search results
    /// When None, defaults to SortOption::Relevance (GitHub's default sorting)
    pub sort_by: Option<SortOption>,

    /// Order parameter for sorting results (ascending or descending)
    /// When None, defaults to OrderOption::Descending
    pub order: Option<OrderOption>,

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

/// Parameters for GitHub repository code search (grep)
///
/// Contains all the parameters needed for configuring a code search request within a GitHub repository.
/// This struct encapsulates repository and search parameters for the grep_repository method.
///
/// # Examples
///
/// ```
/// use gitcodes_mcp::tools::gitcodes::git_service::params::GrepParams;
/// use gitcodes_mcp::tools::gitcodes::git_service::git_repository::RepositoryLocation;
/// use std::path::PathBuf;
///
/// // Basic search with defaults for GitHub URL
/// let params = GrepParams {
///    repository_location: RepositoryLocation::GitHubUrl("https://github.com/rust-lang/rust".to_string()),
///    pattern: "fn main".to_string(),
///    ref_name: None,
///    case_sensitive: None,
///    use_regex: None,
///    file_extensions: None,
///    exclude_dirs: None,
/// };
///
/// // Advanced search with custom options
/// let advanced_params = GrepParams {
///    repository_location: RepositoryLocation::GitHubUrl("github:tokio-rs/tokio".to_string()),
///    pattern: "async fn".to_string(),
///    ref_name: Some("master".to_string()),
///    case_sensitive: Some(true),
///    use_regex: Some(true),
///    file_extensions: Some(vec!["rs".to_string()]),
///    exclude_dirs: Some(vec!["target".to_string(), "examples".to_string()]),
/// };
///
/// // Search in a local directory
/// let local_params = GrepParams {
///    repository_location: RepositoryLocation::LocalPath(PathBuf::from("/path/to/local/repo")),
///    pattern: "struct Config".to_string(),
///    ref_name: None,
///    case_sensitive: Some(false),
///    use_regex: None,
///    file_extensions: Some(vec!["rs".to_string()]),
///    exclude_dirs: None,
/// };
/// ```
#[derive(Debug, schemars::JsonSchema, serde::Serialize, serde::Deserialize)]
pub struct GrepParams {
    /// Repository location (required)
    /// Can be either a GitHub URL or a local filesystem path
    /// GitHub URL formats: https://github.com/user/repo, git@github.com:user/repo.git, github:user/repo
    /// Local path: Direct path to a local directory
    pub repository_location: RepositoryLocation,

    /// Branch or tag (optional, default is 'main' or 'master')
    /// Specifies which branch or tag to search in
    pub ref_name: Option<String>,

    /// Search pattern (required) - the text pattern to search for in the code
    /// Supports regular expressions by default
    pub pattern: String,

    /// Whether to be case-sensitive (optional, default is false)
    /// When true, matching is exact with respect to letter case
    pub case_sensitive: Option<bool>,

    /// Whether to use regex (optional, default is true)
    /// Controls whether the pattern is interpreted as a regular expression or literal text
    pub use_regex: Option<bool>,

    /// File extensions to search (optional, e.g., ["rs", "toml"])
    /// Limits search to files with specified extensions
    pub file_extensions: Option<Vec<String>>,

    /// Directories to exclude from search (optional, e.g., ["target", "node_modules"])
    /// Skips specified directories during search
    pub exclude_dirs: Option<Vec<String>>,
}

/// Sort options for GitHub repository search results
///
/// Controls how repository search results are ordered in the response.
#[derive(
    Debug, schemars::JsonSchema, serde::Serialize, serde::Deserialize, Display, EnumString, AsRefStr,
)]
#[strum(serialize_all = "lowercase")]
pub enum SortOption {
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

impl SortOption {
    /// Converts the sort option to its API string representation
    pub fn to_str(&self) -> &str {
        self.as_ref()
    }
}

impl Default for SortOption {
    /// Returns the default sort option (Relevance)
    fn default() -> Self {
        SortOption::Relevance
    }
}

/// Sort direction options for GitHub repository search results
///
/// Controls whether results are displayed in ascending or descending order.
#[derive(
    Debug, schemars::JsonSchema, serde::Serialize, serde::Deserialize, Display, EnumString, AsRefStr,
)]
#[strum(serialize_all = "lowercase")]
pub enum OrderOption {
    /// Sort in ascending order (lowest to highest, oldest to newest)
    #[strum(serialize = "asc")]
    Ascending,
    /// Sort in descending order (highest to lowest, newest to oldest)
    #[strum(serialize = "desc")]
    Descending,
}

impl OrderOption {
    /// Converts the order option to its API string representation
    pub fn to_str(&self) -> &str {
        self.as_ref()
    }
}

impl Default for OrderOption {
    /// Returns the default order option (Descending)
    fn default() -> Self {
        OrderOption::Descending
    }
}
