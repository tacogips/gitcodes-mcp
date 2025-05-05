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
/// use gitcodes_mcp::tools::gitcodes::github_service::params::{SearchParams, SortOption, OrderOption};
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
/// use gitcodes_mcp::tools::gitcodes::github_service::params::GrepParams;
///
/// // Basic search with defaults
/// let params = GrepParams {
///    repository: "https://github.com/rust-lang/rust".to_string(),
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
///    repository: "github:tokio-rs/tokio".to_string(),
///    pattern: "async fn".to_string(),
///    ref_name: Some("master".to_string()),
///    case_sensitive: Some(true),
///    use_regex: Some(true),
///    file_extensions: Some(vec!["rs".to_string()]),
///    exclude_dirs: Some(vec!["target".to_string(), "examples".to_string()]),
/// };
/// ```
#[derive(Debug, schemars::JsonSchema, serde::Serialize, serde::Deserialize)]
pub struct GrepParams {
    /// Repository URL (required) - supports GitHub formats:
    /// - <https://github.com/user/repo>
    /// - git@github.com:user/repo.git
    /// - github:user/repo
    pub repository: String,
    
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

impl Default for SortOption {
    /// Returns the default sort option (Relevance)
    fn default() -> Self {
        SortOption::Relevance
    }
}

impl SortOption {
    /// Converts the sort option to its API string representation
    pub fn to_str(&self) -> &str {
        self.as_ref()
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

impl Default for OrderOption {
    /// Returns the default order option (Descending)
    fn default() -> Self {
        OrderOption::Descending
    }
}

impl OrderOption {
    /// Converts the order option to its API string representation
    pub fn to_str(&self) -> &str {
        self.as_ref()
    }
}

impl SearchParams {
    /// Constructs the GitHub API URL for repository search
    ///
    /// Builds the complete URL with query parameters for the GitHub search API.
    /// This method handles parameter defaults, validation, and proper URL encoding.
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
    /// use gitcodes_mcp::tools::gitcodes::github_service::params::{SearchParams, SortOption, OrderOption};
    ///
    /// let params = SearchParams {
    ///     query: "rust web framework".to_string(),
    ///     sort_by: Some(SortOption::Stars),
    ///     order: Some(OrderOption::Descending),
    ///     per_page: Some(50),
    ///     page: Some(1),
    /// };
    ///
    /// let url = params.construct_search_url();
    /// // Result: "https://api.github.com/search/repositories?q=rust%20web%20framework&sort=stars&order=desc&per_page=50&page=1"
    /// ```
    pub fn construct_search_url(&self) -> String {
        // Set up sort parameter using Default implementation
        let default_sort = SortOption::default();
        let sort = self.sort_by.as_ref().unwrap_or(&default_sort).to_str();

        // Set up order parameter using Default implementation
        let default_order = OrderOption::default();
        let order = self.order.as_ref().unwrap_or(&default_order).to_str();

        // Set default values for pagination
        let per_page = self.per_page.unwrap_or(30).min(100); // GitHub API limit is 100
        let page = self.page.unwrap_or(1);

        let mut url = format!(
            "https://api.github.com/search/repositories?q={}",
            urlencoding::encode(&self.query)
        );

        if !sort.is_empty() {
            url.push_str(&format!("&sort={}", sort));
        }

        url.push_str(&format!("&order={}", order));
        url.push_str(&format!("&per_page={}&page={}", per_page, page));

        url
    }
}