use std::path::{Path, PathBuf};

use gix;
use lumin::search::{self, SearchResultLine as LuminSearchResultLine};

use crate::gitcodes::repository_manager::providers::GitRemoteRepositoryInfo;
use crate::gitcodes::repository_manager::RepositoryLocation;

mod search_result;
pub use search_result::CodeSearchResult;

mod reference;
pub use reference::GitRefObject;

mod tree;
pub use tree::{RepositoryTree, TreeEntry, TreeParams};

use super::providers::{self, ReferenceInfo, RepositoryRefs};

/// Prefix constants for various git reference types
const ORIGIN_PREFIX: &str = "refs/remotes/origin/";
const HEAD_PREFIX: &str = "refs/heads/";
const TAG_PREFIX: &str = "refs/tags/";

/// Prevents directory traversal attacks in paths
///
/// This standalone function checks if a path contains directory traversal
/// elements like ".." that could be used to access files outside the intended directory.
/// It is crucial for security when working with user-provided paths.
///
/// # Parameters
///
/// * `path` - The path to check for directory traversal attempts
///
/// # Returns
///
/// * `Result<(), String>` - Ok if safe, Err with message if directory traversal is detected
pub fn prevent_directory_traversal(path: &std::path::Path) -> Result<(), String> {
    let path_str = path.to_string_lossy();

    // Check for directory traversal attempts using ".." in the path
    if path_str.contains("..") {
        return Err(format!(
            "Invalid path: '{}'. Paths containing '..' are not allowed for security reasons",
            path.display()
        ));
    }

    // Check for URL-encoded traversal attempts
    // %2E is the URL-encoded form of '.'
    if path_str.contains("%2E%2E")
        || path_str.contains("%2e%2e")
        || path_str.contains("%2E.")
        || path_str.contains("%2e.")
        || path_str.contains(".%2E")
        || path_str.contains(".%2e")
    {
        return Err(format!(
            "Invalid path: '{}'. Paths containing encoded traversal sequences are not allowed",
            path.display()
        ));
    }

    Ok(())
}

/// Parameters for viewing a file in a repository
///
/// This struct encapsulates the parameters needed to view a file's contents.
///
/// # Path Specification
///
/// The `file_path` parameter supports several formats:
/// - Paths without a leading slash: "README.md" or "src/lib.rs"
/// - Paths with a leading slash: "/README.md" or "/src/lib.rs"
///
/// For security reasons, paths containing ".." (parent directory references)
/// are rejected to prevent directory traversal attacks. All paths must stay
/// within the repository boundaries.
#[derive(Debug, Clone)]
pub struct ViewFileParams {
    /// Relative path to the file within the repository
    ///
    /// This path can be specified with or without a leading slash.
    /// For security, paths containing ".." are rejected.
    ///
    /// Examples: "README.md", "/README.md", "src/lib.rs", "/src/lib.rs"
    pub file_path: PathBuf,

    /// Maximum file size to read in bytes
    /// Files larger than this will be rejected to prevent excessive memory usage.
    /// If None, the default from lumin::view::ViewOptions will be used.
    pub max_size: Option<usize>,

    /// Start viewing from this line number (1-indexed)
    /// If None, starts from the first line.
    pub line_from: Option<usize>,

    /// End viewing at this line number (1-indexed, inclusive)
    /// If None, reads to the end of the file.
    pub line_to: Option<usize>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct LocalRepository {
    repository_location: PathBuf,
}

/// Options for performing a code search operation
#[derive(Debug, Clone)]
pub struct CodeSearchOptions {
    pub pattern: String,
    pub case_sensitive: bool,
    pub file_extensions: Option<Vec<String>>,
    pub include_globs: Option<Vec<String>>,
    pub exclude_globs: Option<Vec<String>>,
    pub before_context: Option<usize>,
    pub after_context: Option<usize>,
    pub skip: Option<usize>,
    pub take: Option<usize>,
    pub match_content_omit_num: Option<usize>,
}

/// Code search parameters for searching in a repository
///
/// This struct encapsulates all the parameters needed for a code search.
/// Some fields are optional and have sensible defaults.
///
/// # Repository Location
///
/// The `repository_location` field supports two formats:
/// - Remote repository URLs (e.g., 'github:user/repo', 'git@github.com:user/repo.git', 'https://github.com/user/repo')
/// - Absolute local file paths (e.g., '/path/to/repo', 'C:\\repos\\my-project')
///
/// Relative paths are not supported for security reasons and will be rejected.
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
    /// Repository location (either a remote repository URL or an absolute local file path)
    ///
    /// Supported formats:
    /// - Remote URLs: 'github:user/repo', 'git@github.com:user/repo.git', 'https://github.com/user/repo'
    /// - Local paths: absolute paths only (e.g., '/path/to/repo')
    ///
    /// Note: Relative paths are not supported and will be rejected.
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
    ///
    /// @deprecated Use include_glob instead which provides more flexibility.
    pub file_extensions: Option<Vec<String>>,

    /// Glob patterns to include in search (e.g. ["**/*.rs", "src/**/*.md"])
    ///
    /// This provides more flexibility than file_extensions and is the preferred way
    /// to filter files by pattern.
    pub include_globs: Option<Vec<String>>,

    /// Directories to exclude from search (e.g. ["target", "node_modules"])
    ///
    /// These are converted to glob patterns like "target/**" internally.
    pub exclude_dirs: Option<Vec<String>>,

    /// Number of lines to include before each match (default: 0)
    ///
    /// When this value is greater than zero, the search results will include
    /// the specified number of lines before each matching line. This provides
    /// additional context for understanding the matches.
    pub before_context: Option<usize>,

    /// Number of lines to include after each match (default: 0)
    ///
    /// When this value is greater than zero, the search results will include
    /// the specified number of lines after each matching line. This provides
    /// additional context for understanding the matches.
    pub after_context: Option<usize>,

    /// Number of search results to skip (for pagination)
    ///
    /// This parameter is useful for paginating search results by skipping a certain number
    /// of results before returning. It works in conjunction with `take` to implement
    /// pagination effectively.
    pub skip: Option<usize>,

    /// Maximum number of search results to return (for pagination)
    ///
    /// This parameter limits the number of search results returned and works in conjunction
    /// with `skip` to implement pagination. It's particularly useful when dealing with large
    /// repositories that might have many matches.
    pub take: Option<usize>,

    /// Maximum number of characters to show from matched content (for truncation)
    ///
    /// When matches contain very long lines, this parameter truncates the content to the
    /// specified number of characters to keep responses manageable. If None, shows full
    /// content without truncation.
    pub match_content_omit_num: Option<usize>,
}

impl LocalRepository {
    /// Prevents directory traversal attacks by checking for '..' in paths
    ///
    /// This function checks if a path contains directory traversal patterns
    /// that could be used to access files outside the repository.
    ///
    /// # Parameters
    ///
    /// * `path` - The path to check for directory traversal attempts
    ///
    /// Normalizes and validates a repository-relative path for security
    ///
    /// This function normalizes paths with or without leading slashes and performs
    /// security checks to prevent directory traversal attacks.
    ///
    /// # Parameters
    ///
    /// * `path` - The path to normalize relative to the repository root
    ///
    /// # Returns
    ///
    /// * `Result<PathBuf, String>` - The normalized path or an error message
    ///
    /// # Rules
    ///
    /// 1. Paths containing ".." are rejected to prevent directory traversal
    /// 2. Paths with a leading slash have the slash removed
    /// 3. The resulting path must stay within the repository boundaries
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::PathBuf;
    /// use gitcodes_mcp::gitcodes::LocalRepository;
    ///
    /// # fn example() -> Result<(), String> {
    /// let repo = LocalRepository::new(PathBuf::from("/path/to/repo"));
    ///
    /// // Valid paths
    /// let path1 = repo.normalize_repository_path(&PathBuf::from("file.txt"))?;    // Without leading slash
    /// let path2 = repo.normalize_repository_path(&PathBuf::from("/file.txt"))?;   // With leading slash
    /// let path3 = repo.normalize_repository_path(&PathBuf::from("dir/file.txt"))?; // Subdirectory
    ///
    /// // Invalid paths will return errors
    /// let err1 = repo.normalize_repository_path(&PathBuf::from("../file.txt"));     // Contains ".."
    /// let err2 = repo.normalize_repository_path(&PathBuf::from("dir/../file.txt")); // Contains ".."
    /// # Ok(())
    /// # }
    /// ```
    /// The method is public to allow direct normalization of paths when needed
    pub fn normalize_repository_path(&self, path: &Path) -> Result<PathBuf, String> {
        let path_str = path.to_string_lossy();

        // Check for directory traversal attempts using the standalone function
        prevent_directory_traversal(path)?;

        // Handle paths with or without leading slash
        let normalized_path = if let Some(stripped) = path_str.strip_prefix("/") {
            // If path starts with /, remove it for joining
            PathBuf::from(stripped)
        } else {
            // Otherwise use the path as-is
            path.to_path_buf()
        };

        // Verify the path doesn't attempt to escape the repository
        // This is a secondary check that should never fail if the '..' check passes,
        // but we include it as a defense-in-depth measure
        let full_path = self.repository_location.join(&normalized_path);
        if !full_path.starts_with(&self.repository_location) {
            return Err(format!(
                "Invalid path: '{}'. Path must remain within the repository",
                path.display()
            ));
        }

        Ok(normalized_path)
    }

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
    pub async fn list_repository_refs(&self) -> Result<providers::RepositoryRefs, String> {
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
                return Err(format!("Failed to access repository references: {}", e));
            }
        };

        // Get all references from the platform
        let all_refs = match refs_platform.all() {
            Ok(refs) => refs,
            Err(e) => {
                return Err(format!("Failed to list repository references: {}", e));
            }
        };

        // Process the references into structured objects
        let mut reference_infos = Vec::new();

        for r in all_refs.flatten() {
            // Get reference name - fully qualified name (e.g., refs/heads/main)
            let ref_name = r.name().as_bstr().to_string();

            let target = r.target();
            if let gix::refs::TargetRef::Object(oid) = target {
                let sha = oid.to_hex().to_string();

                // Extract branch name by stripping known prefixes
                // Try each prefix in sequence and use the first successful strip
                let name = ref_name
                    .strip_prefix(ORIGIN_PREFIX)
                    .or_else(|| ref_name.strip_prefix(HEAD_PREFIX))
                    .or_else(|| ref_name.strip_prefix(TAG_PREFIX))
                    .unwrap_or(&ref_name)
                    .to_string();

                reference_infos.push(ReferenceInfo {
                    // Extract short name from full ref path
                    name,
                    full_ref: ref_name.clone(),
                    commit_id: sha,
                });
            }
            // Skip references that couldn't be read
        }

        // Transform into our domain model structure
        let mut branches = Vec::new();
        let mut tags = Vec::new();

        for ref_obj in reference_infos.into_iter() {
            // Sort into branches and tags based on path
            if ref_obj.full_ref.starts_with("refs/remotes/") // Any remote, not just origin
                || ref_obj.full_ref.starts_with(HEAD_PREFIX)
            {
                branches.push(ref_obj);
            } else if ref_obj.full_ref.starts_with(TAG_PREFIX) {
                tags.push(ref_obj);
            }
            // Ignore other ref types
        }
        branches.dedup_by(|l, r| l.name == r.name);
        tags.dedup_by(|l, r| l.name == r.name);

        // Convert to JSON string
        Ok(RepositoryRefs { branches, tags })
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
                                    match prepare
                                        .receive(&mut progress, &gix::interrupt::IS_INTERRUPTED)
                                    {
                                        Ok(_outcome) => {
                                            successful_fetches += 1;
                                        }
                                        Err(e) => {
                                            last_error = Some(format!(
                                                "Failed to fetch from remote '{}': {}",
                                                remote_name_str, e
                                            ));
                                        }
                                    }
                                }
                                Err(e) => {
                                    last_error = Some(format!(
                                        "Failed to prepare fetch for remote '{}': {}",
                                        remote_name_str, e
                                    ));
                                }
                            }
                        }
                        Err(e) => {
                            last_error = Some(format!(
                                "Failed to connect to remote '{}': {}",
                                remote_name_str, e
                            ));
                        }
                    }
                }
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

    /// View a file's contents from the repository
    ///
    /// This function reads a file from the repository, detects its type,
    /// and returns the contents in an appropriate format (text, binary, or image).
    ///
    /// # Path Handling
    ///
    /// The file path is normalized to ensure security and consistency:
    /// - Paths can be specified with or without a leading slash (both "README.md" and "/README.md" work)
    /// - Paths containing ".." are rejected to prevent directory traversal attacks
    /// - The path must stay within the repository boundaries
    ///
    /// # Parameters
    ///
    /// * `params` - The parameters for the view operation, including file path and line range
    ///
    /// # Returns
    ///
    /// * `Result<lumin::view::FileContents, String>` - The file contents or an error message
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The repository is invalid
    /// - The path contains directory traversal patterns ("..")
    /// - The file doesn't exist in the repository
    /// - The file is too large (exceeds the max_size limit in params)
    /// - There's an error reading or processing the file
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use gitcodes_mcp::gitcodes::local_repository::{LocalRepository, ViewFileParams};
    /// use std::path::{Path, PathBuf};
    ///
    /// async fn example() -> Result<(), String> {
    ///     let repo = LocalRepository::new(PathBuf::from("/path/to/repo"));
    ///
    ///     // View a text file with default settings
    ///     // Note that both "README.md" and "/README.md" would work as the file_path
    ///     let params = ViewFileParams {
    ///         file_path: PathBuf::from("README.md"),
    ///         max_size: None,
    ///         line_from: None,
    ///         line_to: None,
    ///     };
    ///
    ///     let contents = repo.view_file_contents(params).await?;
    ///
    ///     // Access subdirectory files using regular path format
    ///     let src_file_params = ViewFileParams {
    ///         file_path: PathBuf::from("src/lib.rs"),
    ///         max_size: None,
    ///         line_from: None,
    ///         line_to: None,
    ///     };
    ///
    ///     // Process the contents based on its type
    ///     match contents {
    ///         lumin::view::FileContents::Text { content, metadata } => {
    ///             println!("File content: {:?}", content);
    ///             println!("Line count: {}", metadata.line_count);
    ///         },
    ///         lumin::view::FileContents::Binary { message, metadata } => {
    ///             println!("Binary file: {}", message);
    ///         },
    ///         lumin::view::FileContents::Image { message, metadata } => {
    ///             println!("Image file: {}", message);
    ///         },
    ///     }
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn view_file_contents(
        &self,
        params: ViewFileParams,
    ) -> Result<lumin::view::FileContents, String> {
        // Validate the repository before attempting to view a file
        if let Err(e) = self.validate() {
            return Err(format!("Cannot view file: Invalid repository: {}", e));
        }

        // First, explicitly check for directory traversal attempts using the standalone function
        prevent_directory_traversal(&params.file_path)?;

        // Validate and normalize the file path
        let file_path = match self.normalize_repository_path(&params.file_path) {
            Ok(path) => path,
            Err(e) => return Err(e),
        };

        // Construct the full path to the file
        let full_path = self.repository_location.join(&file_path);

        // Check if the file exists in the repository
        if !full_path.exists() {
            return Err(format!(
                "File not found in repository: {}",
                params.file_path.display()
            ));
        }

        // Create view options from the parameters
        let options = lumin::view::ViewOptions {
            max_size: params.max_size,
            line_from: params.line_from,
            line_to: params.line_to,
        };

        // Use lumin's view_file function to retrieve and process the file
        match lumin::view::view_file(&full_path, &options) {
            Ok(file_view) => Ok(file_view.contents),
            Err(e) => Err(format!("Failed to view file: {}", e)),
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
    ///         include_globs: None,
    ///         exclude_dirs: Some(vec!["target".to_string()]),
    ///         before_context: None,
    ///         after_context: None,
    ///         skip: None,
    ///         take: None,
    ///         match_content_omit_num: Some(150),
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
    ///         include_globs: None,
    ///         exclude_dirs: None,
    ///         before_context: None,
    ///         after_context: None,
    ///         skip: None,
    ///         take: None,
    ///         match_content_omit_num: Some(150),
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

        // If exclude_dirs contains paths, verify they don't have directory traversal patterns
        if let Some(dirs) = &params.exclude_dirs {
            for dir in dirs {
                // Check each directory path for security issues using the standalone function
                if let Err(e) = prevent_directory_traversal(&PathBuf::from(dir)) {
                    return Err(format!("Invalid exclude_dir path: {}", e));
                }
            }
        }

        // Convert file_extensions to include_glob for backward compatibility
        // This creates glob patterns like "**/*.rs" for each extension
        // NOTE: file_extensions is considered deprecated in favor of include_glob in params
        let include_globs = if params.file_extensions.is_some() && params.include_globs.is_none() {
            // Only convert if file_extensions exists but include_glob doesn't
            params.file_extensions.as_ref().map(|exts| {
                exts.iter()
                    .map(|ext| format!("**/*.{}", ext))
                    .collect::<Vec<String>>()
            })
        } else {
            // Otherwise use the provided include_glob (preferred)
            params.include_globs
        };

        // Make sure we clone file_extensions for both parameters since we need
        // to ensure post-filtering happens even if the lumin glob filtering fails

        // Perform the actual code search using lumin
        let search_options = CodeSearchOptions {
            pattern: pattern.to_string(),
            case_sensitive: params.case_sensitive,
            file_extensions: params.file_extensions.clone(), // Keep file_extensions for backward compatibility
            include_globs,                  // Pass file_extensions as include_glob
            exclude_globs: params.exclude_dirs,
            before_context: params.before_context,
            after_context: params.after_context,
            skip: params.skip, // Skip parameter for pagination
            take: params.take, // Take parameter for pagination
            match_content_omit_num: params.match_content_omit_num,
        };
        
        self.perform_code_search(search_options)
        .await
    }

    /// Get the directory tree structure of the repository
    ///
    /// This method returns a hierarchical representation of all files and directories
    /// in the repository, using the lumin library's tree functionality.
    ///
    /// # Parameters
    ///
    /// * `options` - Optional configuration parameters for the tree generation
    ///
    /// # Returns
    ///
    /// * `Result<Vec<RepositoryTree>, String>` - The directory tree structure or an error message
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::PathBuf;
    /// use gitcodes_mcp::gitcodes::LocalRepository;
    /// use lumin::tree::TreeOptions;
    ///
    /// # async fn example() -> Result<(), String> {
    /// let repo = LocalRepository::new(PathBuf::from("/path/to/repo"));
    ///
    /// // Get tree with default options
    /// let tree = repo.get_tree(None).await?;
    ///
    /// // Get tree with custom options
    /// let mut options = TreeOptions::default();
    /// options.respect_gitignore = true;
    /// options.omit_path_prefix = Some(repo.get_repository_dir().clone());
    /// let tree = repo.get_tree(Some(options)).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Get the directory tree structure of the repository using TreeParams
    ///
    /// This method returns a hierarchical representation of all files and directories
    /// in the repository, using parameters from the TreeParams struct.
    ///
    /// # Parameters
    ///
    /// * `params` - Optional configuration parameters for the tree generation
    ///
    /// # Returns
    ///
    /// * `Result<Vec<RepositoryTree>, String>` - The directory tree structure or an error message
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::PathBuf;
    /// use gitcodes_mcp::gitcodes::LocalRepository;
    /// use gitcodes_mcp::gitcodes::TreeParams;
    ///
    /// # async fn example() -> Result<(), String> {
    /// let repo = LocalRepository::new(PathBuf::from("/path/to/repo"));
    ///
    /// // Get tree with default parameters (respects .gitignore)
    /// let tree = repo.get_tree_with_params(None).await?;
    ///
    /// // Get tree with custom parameters, respecting gitignore
    /// let params_clean = TreeParams {
    ///     case_sensitive: Some(false),
    ///     search_relative_path: None,
    ///     respect_gitignore: Some(true), // Excludes files in .gitignore
    ///     depth: Some(3),  // Only go 3 levels deep
    ///     strip_path_prefix: Some(true),
    /// };
    /// let clean_tree = repo.get_tree_with_params(Some(params_clean)).await?;
    /// // Result: Excludes target/, *.log, .env files, etc.
    ///
    /// // Get complete tree including ignored files
    /// let params_full = TreeParams {
    ///     case_sensitive: Some(false),
    ///     search_relative_path: None,
    ///     respect_gitignore: Some(false), // Includes all files
    ///     depth: None, // No depth limit
    ///     strip_path_prefix: Some(true),
    /// };
    /// let full_tree = repo.get_tree_with_params(Some(params_full)).await?;
    /// // Result: Includes target/, build artifacts, log files, etc.
    ///
    /// // Get tree from a specific subdirectory
    /// let params_subdir = TreeParams {
    ///     case_sensitive: None,
    ///     search_relative_path: Some(PathBuf::from("src")),
    ///     respect_gitignore: Some(true),
    ///     depth: None,
    ///     strip_path_prefix: Some(true),
    /// };
    /// let src_tree = repo.get_tree_with_params(Some(params_subdir)).await?;
    /// // Result: Tree starting from src/ directory, respecting .gitignore
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Gitignore Handling
    ///
    /// The `respect_gitignore` parameter controls whether .gitignore rules are applied:
    ///
    /// - **Default/True**: Excludes files and directories listed in .gitignore files
    ///   - Cleaner output focused on source code and tracked files
    ///   - Faster processing by skipping ignored directories
    ///   - Recommended for most use cases
    ///
    /// - **False**: Includes all files regardless of .gitignore rules
    ///   - Shows complete filesystem structure
    ///   - Useful for debugging or when you need to see ignored files
    ///   - May result in larger trees and longer processing times
    pub async fn get_tree_with_params(
        &self,
        params: Option<TreeParams>,
    ) -> Result<Vec<RepositoryTree>, String> {
        // Validate the repository before generating the tree
        if let Err(e) = self.validate() {
            return Err(format!("Cannot generate tree: Invalid repository: {}", e));
        }

        // Extract search_relative_path and convert params to options
        let (tree_root_path, options) = if let Some(params) = params {
            let search_relative_path = params.search_relative_path.clone();
            let root_path = match search_relative_path {
                Some(relative_path) => self.repository_location.join(relative_path),
                None => self.repository_location.clone(),
            };
            (root_path, params.to_tree_options(&self.repository_location))
        } else {
            // Create default options
            let opts = lumin::tree::TreeOptions {
                omit_path_prefix: Some(self.repository_location.clone()),
                respect_gitignore: true,
                ..Default::default()
            };
            (self.repository_location.clone(), opts)
        };
        // Generate the directory tree using lumin
        match lumin::tree::generate_tree(tree_root_path.as_path(), &options) {
            Ok(trees) => Ok(trees.into_iter().map(RepositoryTree::from).collect()),
            Err(e) => Err(format!("Failed to generate directory tree: {}", e)),
        }
    }

    /// Get the directory tree structure of the repository using TreeOptions
    ///
    /// This method returns a hierarchical representation of all files and directories
    /// in the repository, using the lumin library's tree functionality.
    ///
    /// # Parameters
    ///
    /// * `options` - Optional configuration parameters for the tree generation
    ///
    /// # Returns
    ///
    /// * `Result<Vec<RepositoryTree>, String>` - The directory tree structure or an error message
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::PathBuf;
    /// use gitcodes_mcp::gitcodes::LocalRepository;
    /// use lumin::tree::TreeOptions;
    ///
    /// # async fn example() -> Result<(), String> {
    /// let repo = LocalRepository::new(PathBuf::from("/path/to/repo"));
    ///
    /// // Get tree with default options
    /// let tree = repo.get_tree(None).await?;
    ///
    /// // Get tree with custom options
    /// let mut options = TreeOptions::default();
    /// options.respect_gitignore = true;
    /// options.omit_path_prefix = Some(repo.get_repository_dir().clone());
    /// let tree = repo.get_tree(Some(options)).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_tree(
        &self,
        options: Option<lumin::tree::TreeOptions>,
    ) -> Result<Vec<RepositoryTree>, String> {
        // Validate the repository before generating the tree
        if let Err(e) = self.validate() {
            return Err(format!("Cannot generate tree: Invalid repository: {}", e));
        }

        // Create options with sensible defaults if not provided
        let options = options.unwrap_or_else(|| {
            lumin::tree::TreeOptions {
                // By default, strip the repository path prefix for cleaner output
                omit_path_prefix: Some(self.repository_location.clone()),
                // By default, respect .gitignore files
                respect_gitignore: true,
                ..Default::default()
            }
        });

        // Generate the directory tree using lumin
        match lumin::tree::generate_tree(self.repository_location.as_path(), &options) {
            Ok(trees) => Ok(trees.into_iter().map(RepositoryTree::from).collect()),
            Err(e) => Err(format!("Failed to generate directory tree: {}", e)),
        }
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
    /// * `file_extensions` - DEPRECATED: Use `include_globs` instead. Optional array of file extensions to include (e.g. ["js", "ts"])
    /// * `include_globs` - Optional glob patterns to include in search (e.g. ["**/*.js", "**/*.ts"])
    /// * `exclude_globs` - Optional directories to exclude from search (converted to glob patterns internally)
    /// * `before_context` - Optional number of lines to include before each match
    /// * `after_context` - Optional number of lines to include after each match
    /// * `skip` - Optional number of results to skip (for pagination)
    /// * `take` - Optional maximum number of results to return (for pagination)
    ///
    /// Perform a code search in the repository with the given parameters.
    ///
    /// # Returns
    ///
    /// * `Result<CodeSearchResult, String>` - Structured search results or an error message
    ///
    /// This method searches for a pattern in the repository's code, with various filtering options.
    /// It uses lumin's search functionality internally, with additional filtering capabilities.
    ///
    /// # Arguments
    ///
    /// * `pattern` - The regex pattern to search for
    /// * `case_sensitive` - Whether the search should be case-sensitive
    /// * `file_extensions` - DEPRECATED: Use `include_globs` instead. Optional list of file extensions to include (e.g., `["rs", "toml"]`)
    /// * `include_globs` - Optional list of glob patterns for files to include (e.g., `["**/*.rs", "**/*.toml"]`)
    /// * `exclude_globs` - Optional list of directory names to exclude (automatically formatted as glob patterns)
    /// * `before_context` - Optional number of lines to include before each match
    /// * `after_context` - Optional number of lines to include after each match
    /// * `skip` - Optional number of results to skip (for pagination)
    /// * `take` - Optional maximum number of results to return (for pagination)
    ///
    /// # Notes on Glob Patterns
    ///
    /// The `include_globs` parameter should follow these guidelines for lumin 0.1.13:
    ///
    /// - To include files with a specific extension: `"**/*.ext"` (e.g., `"**/*.rs"`)
    /// - To include files in a specific directory: `"**/dirname/**"` (not just `"dirname/**"`)
    /// - For nested directories, always prefix with `"**/"` to match at any level
    ///
    /// The `exclude_globs` parameter works differently:
    ///
    /// - Just provide directory names like `["node_modules", "target"]`
    /// - The method will format them into glob patterns: `["**/node_modules/**", "**/target/**"]`
    ///
    /// In lumin 0.1.13, glob filtering works reliably, but we still keep the legacy `file_extensions`
    /// filtering for backward compatibility. New code should use `include_globs` instead.
    ///
    /// # Returns
    ///
    /// A `CodeSearchResult` containing the matches found and the search parameters used.
    pub async fn perform_code_search(
        &self,
        options: CodeSearchOptions,
    ) -> Result<CodeSearchResult, String> {
        // Clone include_glob for use in multiple places
        let include_glob_clone = options.include_globs.clone();

        // Configure search options for lumin 0.1.13
        // Note: in lumin 0.1.13, glob patterns work reliably for both include_glob and exclude_glob
        let search_options = search::SearchOptions {
            case_sensitive: options.case_sensitive,
            respect_gitignore: true,
            // Convert directory names to glob patterns by adding "**/" prefix and "/**" suffix
            // This format ensures that directories at any level in the hierarchy will be excluded
            exclude_glob: options.exclude_globs
                .as_ref()
                .map(|dirs| dirs.iter().map(|dir| format!("**/{}/**", dir)).collect()),
            match_content_omit_num: options.match_content_omit_num,
            before_context: options.before_context.unwrap_or(0), // Default to 0 if None
            after_context: options.after_context.unwrap_or(0), // Default to 0 if None
            depth: None,                  // Use default depth (no limit)
            include_glob: include_glob_clone, // Pattern for filtering files by glob
            skip: options.skip,                         // For pagination
            take: options.take,                         // For pagination
            omit_path_prefix: Some(self.repository_location.clone()), // Omit repository path prefix from results
        };

        // Get repository path
        let repo_path = self.repository_location.as_path();

        // Execute the search directly with the provided pattern
        // The caller is responsible for properly formatting the regex pattern
        let search_result = search::search_files(&options.pattern, repo_path, &search_options)
            .map_err(|e| format!("Code search failed: {}", e))?;

        // Process the search results
        // Even though lumin 0.1.13 has good glob support, we still apply extension filtering
        // for maximum reliability and backward compatibility
        let all_results = if let Some(extensions) = &options.file_extensions {
            // Extract matches from all the matched files, filtering by extension
            let filtered_results: Vec<LuminSearchResultLine> = search_result
                .lines
                .into_iter()
                .filter(|result| {
                    // Only keep files with matching extensions
                    if let Some(ext) = result.file_path.extension() {
                        if let Some(ext_str) = ext.to_str() {
                            return extensions.iter().any(|e| e == ext_str);
                        }
                    }
                    false // Filter out files with no extension
                })
                .collect();

            filtered_results
        } else {
            // No extension filtering needed
            search_result.lines
        };

        // Directory exclusion is now handled via SearchOptions exclude_glob

        // Create a CodeSearchResult - make sure we preserve the original pattern
        // from the request, not a potentially modified version from lumin
        let params = crate::gitcodes::local_repository::search_result::CodeSearchParams {
            total_match_line_number: search_result.total_number,
            search_result_lines: all_results,
            pattern: options.pattern.clone(), // Use the original pattern string
            repository: repo_path.to_path_buf(),
            case_sensitive: options.case_sensitive,
            file_extensions: options.file_extensions,
            include_globs: options.include_globs,
            exclude_globs: options.exclude_globs,
            before_context: options.before_context,
            after_context: options.after_context,
        };
        Ok(CodeSearchResult::new(params))
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
