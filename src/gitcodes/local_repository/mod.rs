use std::path::PathBuf;

use gix;
use lumin::search;
use serde_json;

use crate::gitcodes::repository_manager::providers::GitRemoteRepositoryInfo;
use crate::gitcodes::repository_manager::RepositoryLocation;

mod search_result;
pub use search_result::CodeSearchResult;

mod reference;
pub use reference::GitRefObject;

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
    /// Prefix used for temporary repository directories
    ///
    /// This prefix helps identify directories that were created by this library
    /// and can be safely deleted during cleanup operations.
    pub const REPOSITORY_DIR_PREFIX: &'static str = "mcp_gitcodes";
    /// Cleans up the repository by deleting its directory
    ///
    /// This method should be called when the repository is no longer needed
    /// to free up disk space. It removes the entire directory containing
    /// the repository.
    ///
    /// # Returns
    ///
    /// * `Result<(), String>` - Success or error message
    ///
    /// # Safety
    ///
    /// This method permanently deletes files from the filesystem.
    /// It is designed to be safe since it only removes temporary cloned repositories
    /// in system temp directories with specific naming patterns, but should be used
    /// with caution.
    pub fn cleanup(&self) -> Result<(), String> {
        // Get the repository directory
        let repo_dir = self.get_repository_dir();

        // Only delete directories that appear to be temporary GitCodes repositories
        // These have a specific naming pattern defined by REPOSITORY_DIR_PREFIX
        let dir_name = repo_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");

        if !dir_name.starts_with(Self::REPOSITORY_DIR_PREFIX) {
            return Err(format!(
                "Refusing to delete directory '{}' that doesn't match temporary repository pattern",
                repo_dir.display()
            ));
        }

        // Delete the directory recursively using std::fs::remove_dir_all
        if repo_dir.exists() {
            match std::fs::remove_dir_all(repo_dir) {
                Ok(_) => Ok(()),
                Err(e) => Err(format!("Failed to delete repository directory: {}", e)),
            }
        } else {
            // If directory doesn't exist, consider it a success
            Ok(())
        }
    }

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
            "{}_{}_{}_{}",
            Self::REPOSITORY_DIR_PREFIX,
            remote_repository_info.user,
            remote_repository_info.repo,
            hash_value
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
    /// Returns a JSON string with all the references in the repository.
    /// This function uses the gix library to open the repository and enumerate all its references.
    /// References include branches and tags, and are returned with their names and SHA values.
    ///
    /// # Returns
    ///
    /// A Result containing either a JSON string with an array of objects representing repository references,
    /// or an error message as a String if something went wrong.
    /// The JSON array contains objects where each object has the reference name and its corresponding commit SHA.
    ///
    /// # Format
    ///
    /// The returned JSON is an array of GitRefObject structs with the following structure:
    /// ```json
    /// [
    ///   {
    ///     "ref": "refs/heads/main",
    ///     "object": {
    ///       "sha": "abcdef1234567890",
    ///       "type": "commit"
    ///     }
    ///   },
    ///   ...
    /// ]
    /// ```
    ///
    /// This matches the GitHub API format for references.
    pub async fn list_repository_refs(&self) -> Result<String, String> {
        // Open the repository
        let repo = match gix::open(&self.repository_location) {
            Ok(repo) => repo,
            Err(e) => {
                return Err(format!(
                    "Failed to open repository at {}: {}",
                    self.repository_location.display(),
                    e
                ));
            }
        };

        // Get the references platform
        let refs_platform = match repo.references() {
            Ok(platform) => platform,
            Err(e) => {
                return Err(format!(
                    "Failed to access repository references: {}",
                    e
                ));
            }
        };

        // Get all references from the platform
        let all_refs = match refs_platform.all() {
            Ok(refs) => refs,
            Err(e) => {
                return Err(format!(
                    "Failed to list repository references: {}",
                    e
                ));
            }
        };

        // Process the references into structured objects
        let mut result = Vec::new();
        for reference in all_refs {
            if let Ok(r) = reference {
                // Get reference name - fully qualified name (e.g., refs/heads/main)
                let ref_name = r.name().as_bstr().to_string();
                
                // Get target information (the SHA)
                let target = r.target();
                let sha = target.id().to_hex().to_string();
                
                // Create a properly structured reference object
                let ref_obj = GitRefObject::new(ref_name, sha);
                
                result.push(ref_obj);
            }
            // Skip references that couldn't be read
        }

        // Convert to JSON string
        match serde_json::to_string(&result) {
            Ok(json) => Ok(json),
            Err(e) => Err(format!(
                "Failed to serialize repository references: {}",
                e
            )),
        }
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
    #[allow(dead_code)]
    async fn update_repository(&self, _ref_name: &str) -> Result<(), String> {
        // This functionality is temporarily disabled during refactoring
        // TODO: Reimplement with current gix API
        Err("Repository updating is temporarily disabled during refactoring.".to_string())
    }
    
    /// Fetch updates from the remote repository
    ///
    /// This function fetches the latest changes from all remotes of the repository
    /// without merging or rebasing them. It's useful to ensure that the local repository
    /// has up-to-date information about remote branches and tags before listing them.
    ///
    /// # Returns
    ///
    /// * `Result<(), String>` - Success or error message
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use gitcodes_mcp::gitcodes::local_repository::LocalRepository;
    /// use std::path::PathBuf;
    ///
    /// async fn example() {
    ///     let repo = LocalRepository::new(PathBuf::from("/path/to/repo"));
    ///     
    ///     // Fetch latest changes from remote
    ///     if let Err(e) = repo.fetch_remote().await {
    ///         eprintln!("Failed to fetch from remote: {}", e);
    ///     }
    ///     
    ///     // Now list references (which will include remote refs)
    ///     let refs = repo.list_repository_refs().await.unwrap();
    /// }
    /// ```
    ///
    /// # Implementation Details
    ///
    /// This method fetches updates from all configured remotes in the repository using
    /// the native `gix` library. For each remote:
    ///
    /// 1. It uses `find_fetch_remote` to get a properly configured remote
    /// 2. Connects to the remote with `connect(gix::remote::Direction::Fetch)`
    /// 3. Prepares the fetch operation with `prepare_fetch`
    /// 4. Executes the fetch with `receive`
    ///
    /// If at least one remote was successfully fetched, the operation is considered
    /// successful. If all remote fetches fail, the last error message is returned.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The repository is invalid
    /// - The repository has no configured remotes
    /// - All remote fetch operations failed
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), String> {
    /// # use gitcodes_mcp::gitcodes::local_repository::LocalRepository;
    /// # use std::path::PathBuf;
    /// #
    /// # let repo_path = PathBuf::from("/path/to/repo");
    /// # let repo = LocalRepository::new(repo_path);
    /// // Fetch latest changes from all remotes
    /// repo.fetch_remote().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn fetch_remote(&self) -> Result<(), String> {
        // Verify the repository exists and is valid
        if let Err(e) = self.validate() {
            return Err(format!("Cannot fetch: Invalid repository: {}", e));
        }
        
        // Open the repository using gix
        let repo = match gix::open(&self.repository_location) {
            Ok(repo) => repo,
            Err(e) => {
                return Err(format!(
                    "Failed to open repository at {} for fetching: {}",
                    self.repository_location.display(),
                    e
                ));
            }
        };
        
        // Get the remote configurations - this returns a BTreeSet directly, not a Result
        let remote_names = repo.remote_names();
        
        // If there are no remotes, return early with a message
        if remote_names.is_empty() {
            return Err("Repository has no configured remotes".to_string());
        }
        
        // For each remote, try to fetch using the native gix library
        let mut successful_fetches = 0;
        let mut last_error = None;
        
        // Create a simple progress reporter that does nothing (we could implement a real one later)
        let mut progress = gix::progress::Discard;
        
        for remote_name in remote_names {
            // Convert from Cow<BStr> to regular String for error messages
            let remote_name_str = remote_name.to_string();
            
            // Use find_fetch_remote to get a properly configured remote
            let remote_result = repo.find_fetch_remote(Some(&*remote_name));
            
            match remote_result {
                Ok(remote) => {
                    // Connect to the remote for fetching
                    match remote.connect(gix::remote::Direction::Fetch) {
                        Ok(connection) => {
                            // Prepare the fetch operation
                            match connection.prepare_fetch(&mut progress, Default::default()) {
                                Ok(prepare) => {
                                    // Execute the fetch operation
                                    match prepare.receive(&mut progress, &gix::interrupt::IS_INTERRUPTED) {
                                        Ok(_outcome) => {
                                            successful_fetches += 1;
                                        },
                                        Err(e) => {
                                            last_error = Some(format!(
                                                "Failed to fetch from remote '{}': {}", 
                                                remote_name_str, e
                                            ));
                                        }
                                    }
                                },
                                Err(e) => {
                                    last_error = Some(format!(
                                        "Failed to prepare fetch for remote '{}': {}", 
                                        remote_name_str, e
                                    ));
                                }
                            }
                        },
                        Err(e) => {
                            last_error = Some(format!(
                                "Failed to connect to remote '{}': {}", 
                                remote_name_str, e
                            ));
                        }
                    }
                },
                Err(e) => {
                    last_error = Some(format!(
                        "Failed to initialize remote '{}' for fetching: {}", 
                        remote_name_str, e
                    ));
                }
            }
        }
        
        // If we successfully fetched at least one remote, consider it a success
        if successful_fetches > 0 {
            Ok(())
        } else if let Some(error) = last_error {
            // If all fetches failed, return the last error message
            Err(error)
        } else {
            // This should never happen since we check for empty remote_names above
            Err("No remotes were fetched".to_string())
        }
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
    /// ```no_run
    /// use gitcodes_mcp::gitcodes::local_repository::CodeSearchParams;
    /// use gitcodes_mcp::gitcodes::repository_manager::RepositoryLocation;
    /// use std::str::FromStr;
    ///
    /// async fn example() -> Result<(), String> {
    ///     // Using regex pattern directly
    ///     let params = CodeSearchParams {
    ///         repository_location: RepositoryLocation::from_str("https://github.com/user/repo").unwrap(),
    ///         ref_name: Some("main".to_string()),
    ///         pattern: "fn main".to_string(),
    ///         case_sensitive: false,
    ///         file_extensions: Some(vec!["rs".to_string()]),
    ///         exclude_dirs: Some(vec!["target".to_string()]),
    ///     };
    ///
    ///     // Create a repository instance (mock for example)
    ///     let repo = gitcodes_mcp::gitcodes::local_repository::LocalRepository::new(std::path::PathBuf::from("/tmp/example"));
    ///
    ///     // Using literal text search (with escaped regex)
    ///     let literal_pattern = "file.txt".replace(".", "\\."); // Escape the period
    ///     let params2 = CodeSearchParams {
    ///         repository_location: RepositoryLocation::from_str("https://github.com/user/repo").unwrap(),
    ///         ref_name: None,
    ///         pattern: literal_pattern,
    ///         case_sensitive: true,
    ///         file_extensions: None,
    ///         exclude_dirs: None,
    ///     };
    ///
    ///     // Search the code
    ///     let results = repo.search_code(params).await?;
    ///     Ok(())
    /// }
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
