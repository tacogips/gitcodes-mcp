use lumin::search;
use std::path::PathBuf;

use crate::gitcodes::repository_manager::providers::GitRemoteRepositoryInfo;
use crate::gitcodes::repository_manager::RepositoryLocation;

mod search_result;
pub use search_result::CodeSearchResult;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct LocalRepository {
    repository_location: PathBuf,
}
/// Code search parameters for searching in a repository
///
/// This struct encapsulates all the parameters needed for a code search.
/// Some fields are optional and have sensible defaults.
///
/// # Pattern Syntax
///
/// The pattern parameter is passed directly to the underlying search engine (lumin),
/// which uses the regex syntax from the `grep` crate for pattern matching.
///
/// ## Regex Pattern Examples
///
/// The search pattern supports standard regex syntax, including:
///
/// - Simple text literals: `function` matches "function" anywhere in text
/// - Any character: `log.txt` matches "log.txt", "log1txt", etc.
/// - Character classes: `[0-9]+` matches one or more digits
/// - Word boundaries: `\bword\b` matches "word" but not "words" or "keyword"
/// - Line anchors: `^function` matches lines starting with "function"
/// - Alternatives: `error|warning` matches either "error" or "warning"
/// - Repetitions: `.*` matches any sequence of characters
/// - Escaped special chars: `\.` to match a literal period
///
/// ## For Literal Text Search
///
/// If you want to search for a text pattern that contains regex special characters
/// but want them interpreted literally, you need to escape those characters:
///
/// ```rust
/// // Helper function to escape regex special characters for literal searches
/// fn escape_regex(pattern: &str) -> String {
///     let mut escaped = String::with_capacity(pattern.len() * 2);
///     for c in pattern.chars() {
///         match c {
///             '.' | '^' | '$' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '\\' | '|' => {
///                 escaped.push('\\');
///                 escaped.push(c);
///             }
///             _ => escaped.push(c),
///         }
///     }
///     escaped
/// }
///
/// // Example: Search for "file.txt" literally (not as a regex pattern)
/// let literal_search_pattern = escape_regex("file.txt"); // Becomes "file\.txt"
/// ```
#[derive(Debug, Clone)]
pub struct CodeSearchParams {
    /// Repository location (URL or local path)
    pub repository_location: RepositoryLocation,

    /// Optional specific branch or tag name
    pub ref_name: Option<String>,

    /// Search pattern (text to find)
    ///
    /// The pattern is passed directly to the underlying search engine and is
    /// interpreted as a regex pattern. If you want to search for text containing
    /// regex special characters literally, you must escape them yourself.
    pub pattern: String,

    /// Whether the search is case-sensitive (default: false)
    pub case_sensitive: bool,

    /// File extensions to include in search (e.g. ["rs", "md"])
    pub file_extensions: Option<Vec<String>>,

    /// Directories to exclude from search (e.g. ["target", "node_modules"])
    ///
    /// These are converted to glob patterns like "target/**" internally.
    pub exclude_dirs: Option<Vec<String>>,
}

impl LocalRepository {
    /// Validates that the repository location is a valid git repository
    ///
    /// This validation checks both that:
    /// 1. The directory exists and is accessible
    /// 2. The directory contains a valid git repository
    ///
    /// If this validation fails, it may mean the git repository hasn't been
    /// cloned yet or that there's another issue with the repository.
    pub fn validate(&self) -> Result<(), String> {
        // Check if the directory exists
        if !self.repository_location.is_dir() {
            return Err(format!(
                "Local path '{}' is not a directory",
                self.repository_location.display()
            ));
        }

        // Check if it's a git repository
        let git_dir = self.repository_location.join(".git");
        let is_bare_repo = self.repository_location.join("HEAD").exists();

        if !git_dir.exists() && !is_bare_repo {
            return Err(format!(
                "The directory '{}' does not appear to be a git repository",
                self.repository_location.display()
            ));
        }

        Ok(())
    }

    /// Creates a new LocalRepository reference with the given path
    ///
    /// This just creates a reference to the repository path, it doesn't
    /// validate or prepare anything. Use validate() if that's needed.
    pub fn new(repository_location: PathBuf) -> Self {
        Self {
            repository_location,
        }
    }

    /// Generate a unique directory name for the repository based on its information
    /// and the manager's process ID
    ///
    /// # Parameters
    ///
    /// * `remote_repository_info` - Information about the remote repository
    /// * `process_id` - Optional unique process ID from the repository manager
    ///                  Used as part of the hash calculation to ensure uniqueness
    pub fn new_local_repository_to_clone(
        remote_repository_info: GitRemoteRepositoryInfo,
        process_id: Option<&str>,
    ) -> Self {
        // Generate hash value with process_id included in the hash calculation
        let hash_value = Self::generate_repository_hash(&remote_repository_info, process_id);

        // Create directory name with the hash that already incorporates process_id
        let dir_name = format!(
            "mcp_gitcodes_{}_{}_{}",
            remote_repository_info.user, remote_repository_info.repo, hash_value
        );

        let mut repo_dir = std::env::temp_dir();
        repo_dir.push(dir_name);

        Self::new(repo_dir)
    }

    /// Get the repository directory path
    pub fn get_repository_dir(&self) -> &PathBuf {
        &self.repository_location
    }

    /// List references in a repository
    ///
    /// Returns a JSON string with all the references in the repository
    pub async fn list_repository_refs(&self, _repository_location: RepositoryLocation) -> String {
        // Temporary implementation
        "Repository ref listing functionality is temporarily disabled during refactoring."
            .to_string()
    }

    /// Update a local repository by pulling from remote
    ///
    /// This operation ensures the local repository is up-to-date with the remote
    /// by fetching and checking out the specified reference.
    ///
    /// # Parameters
    ///
    /// * `repo_dir` - The directory containing the repository
    /// * `ref_name` - Branch or tag name to checkout
    async fn update_repository(&self, _ref_name: &str) -> Result<(), String> {
        // This functionality is temporarily disabled during refactoring
        // TODO: Reimplement with current gix API
        Err("Repository updating is temporarily disabled during refactoring.".to_string())
    }

    /// Search code in a repository by pattern
    ///
    /// This function handles searching code within a repository. It takes
    /// a CodeSearchParams struct with all the necessary search parameters.
    ///
    /// # Parameters
    ///
    /// * `params` - Parameters for the code search including repository and pattern
    ///
    /// # Returns
    ///
    /// * `Result<CodeSearchResult, String>` - Structured search results or an error message
    ///
    /// # Examples
    ///
    /// ```
    /// // Using regex pattern directly
    /// let params = CodeSearchParams {
    ///     repository_location: "https://github.com/user/repo".parse()?,
    ///     ref_name: Some("main".to_string()),
    ///     pattern: "fn main".to_string(),
    ///     case_sensitive: false,
    ///     file_extensions: Some(vec!["rs".to_string()]),
    ///     exclude_dirs: Some(vec!["target".to_string()]),
    /// };
    ///
    /// // Using literal text search (with escaped regex)
    /// let literal_pattern = "file.txt".replace(".", "\\."); // Escape the period
    /// let params = CodeSearchParams {
    ///     repository_location: "https://github.com/user/repo".parse()?,
    ///     ref_name: None,
    ///     pattern: literal_pattern,
    ///     case_sensitive: true,
    ///     file_extensions: None,
    ///     exclude_dirs: None,
    /// };
    ///
    /// let results = search_code(params).await?;
    /// ```
    pub async fn search_code(&self, params: CodeSearchParams) -> Result<CodeSearchResult, String> {
        // Validate the repository before searching
        if let Err(e) = self.validate() {
            return Err(format!("Repository validation failed: {}", e));
        }

        // If a specific ref is requested, try to update the repository
        if let Some(ref_name) = &params.ref_name {
            // Temporarily disabled due to ongoing refactoring
            // Just log the request rather than attempting update
            eprintln!("Note: Reference '{}' was requested, but repository updates are temporarily disabled", ref_name);
        }

        // Get the pattern - the caller is responsible for properly escaping regex special characters
        // if they want to perform a literal text search
        let pattern = &params.pattern;

        // Perform the actual code search using lumin
        self.perform_code_search(
            pattern,
            params.case_sensitive,
            params.file_extensions,
            params.exclude_dirs,
        )
        .await
    }

    /// Performs a code search on a prepared repository
    ///
    /// This function executes the search using the lumin search library
    /// and processes the results.
    ///
    /// # Parameters
    ///
    /// * `pattern` - The search pattern to look for (regex pattern passed directly to lumin)
    /// * `case_sensitive` - Whether the search should be case-sensitive (default: false)
    /// * `file_extensions` - Optional array of file extensions to include (e.g. ["js", "ts"])
    /// * `exclude_dirs` - Optional directories to exclude from search
    ///
    /// # Returns
    ///
    /// * `Result<CodeSearchResult, String>` - Structured search results or an error message
    pub async fn perform_code_search(
        &self,
        pattern: &str,
        case_sensitive: bool,
        file_extensions: Option<Vec<String>>,
        exclude_dirs: Option<Vec<String>>,
    ) -> Result<CodeSearchResult, String> {
        // Configure search options
        let search_options = search::SearchOptions {
            case_sensitive,
            respect_gitignore: true,
            exclude_glob: exclude_dirs
                .as_ref()
                .map(|dirs| dirs.iter().map(|dir| format!("**/{}/**", dir)).collect()),
            match_content_omit_num: None, // Default to None (no omission)
        };

        // Get repository path
        let repo_path = self.repository_location.as_path();

        // Execute the search directly with the provided pattern
        // The caller is responsible for properly formatting the regex pattern
        let mut all_results = search::search_files(pattern, repo_path, &search_options)
            .map_err(|e| format!("Code search failed: {}", e))?;

        // Post-process to filter by file extension if needed
        if let Some(extensions) = &file_extensions {
            all_results.retain(|result| {
                if let Some(ext) = result.file_path.extension() {
                    if let Some(ext_str) = ext.to_str() {
                        return extensions.iter().any(|e| e == ext_str);
                    }
                }
                false
            });
        }

        // Directory exclusion is now handled via SearchOptions exclude_glob

        // Create a CodeSearchResult
        Ok(CodeSearchResult::new(
            all_results,
            pattern,
            repo_path.to_path_buf(),
            case_sensitive,
            file_extensions,
            exclude_dirs,
        ))
    }

    /// Generate a 12-character hash value from repository information
    ///
    /// Creates a deterministic hash based on the user and repository name.
    /// If process_id is provided, it will be included in the hash calculation to ensure
    /// uniqueness across different processes for the same repository.
    fn generate_repository_hash(
        remote_repository_info: &GitRemoteRepositoryInfo,
        process_id: Option<&str>,
    ) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Create a string combining user, repo, and process_id if available
        let hash_input = if let Some(pid) = process_id {
            format!(
                "{}{}{}",
                remote_repository_info.user, remote_repository_info.repo, pid
            )
        } else {
            format!(
                "{}{}",
                remote_repository_info.user, remote_repository_info.repo
            )
        };

        // Hash the string to get a unique value
        let mut hasher = DefaultHasher::new();
        hash_input.hash(&mut hasher);
        let hash_value = hasher.finish();

        // Format as a 12-character hex string
        format!("{:012x}", hash_value)
    }
}
