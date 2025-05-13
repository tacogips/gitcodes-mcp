use lumin::{search, search::SearchOptions};
use std::path::Path;

/// Performs a code search on a prepared repository
///
/// This function executes the search using the lumin search library
/// and processes the results.
///
/// # Parameters
///
/// * `repo_dir` - The directory containing the repository
/// * `pattern` - The search pattern to look for
/// * `case_sensitive` - Whether the search should be case-sensitive (default: false)
/// * `_use_regex` - Whether to use regex for the search (not currently implemented)
/// * `_file_extensions` - Filter by file extensions (not currently implemented)
pub async fn perform_code_search(
    repo_dir: &Path,
    pattern: &str,
    case_sensitive: Option<bool>,
    _use_regex: Option<bool>,
    _file_extensions: Option<Vec<String>>,
) -> Result<String, String> {
    // Create search options
    let search_options = SearchOptions {
        case_sensitive: case_sensitive.unwrap_or(false),
        ..SearchOptions::default()
    };

    // Execute the search
    match search::search_files(pattern, repo_dir, &search_options) {
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

            Ok(output)
        }
        Err(e) => Err(format!("Lumin search failed: {}", e)),
    }
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
