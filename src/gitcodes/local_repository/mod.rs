use gix;
use gix::progress::Discard;
use lumin::search::{self, SearchOptions};
use rmcp::schemars;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};

use crate::gitcodes::repository_manager::RepositoryLocation;
use crate::gitcodes::repository_manager::providers::GitRemoteRepositoryInfo;

// Simple type to represent a Git reference (branch, tag, etc.)
#[derive(Debug, Clone)]
pub struct GitRef {
    pub name: String,
}

impl GitRef {
    pub fn new(name: String) -> Self {
        Self { name }
    }
    
    // For convenience in formatting
    pub fn as_str(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct LocalRepository {
    repository_location: PathBuf,
}
/// Code search parameters for searching in a repository
///
/// This struct encapsulates all the parameters needed for a code search.
/// Some fields are optional and have sensible defaults.
#[derive(Debug, Clone)]
pub struct CodeSearchParams {
    /// Repository location (URL or local path)
    pub repository_location: RepositoryLocation,

    /// Optional specific branch or tag name
    pub ref_name: Option<String>,

    /// Search pattern (text to find)
    pub pattern: String,

    /// Whether the search is case-sensitive (default: false)
    pub case_sensitive: bool,

    /// Whether to use regex for searching (default: true)
    pub use_regex: bool,

    /// File extensions to include in search (e.g. ["rs", "md"])
    pub file_extensions: Option<Vec<String>>,

    /// Directories to exclude from search (e.g. ["target", "node_modules"])
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
    pub fn new_local_repository_to_clone(remote_repository_info: GitRemoteRepositoryInfo) -> Self {
        let hash_value = Self::generate_repository_hash(&remote_repository_info);
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
        "Repository ref listing functionality is temporarily disabled during refactoring.".to_string()
    }

    /// Update a local repository by pulling from remote
    ///
    /// This operation ensures the local repository is up-to-date with the remote
    /// by fetching and checking out the specified reference.
    ///
    /// # Parameters
    ///
    /// * `repo_dir` - The directory containing the repository
    /// * `git_ref` - Branch or tag name to checkout as a GitRef
    async fn update_repository(&self, _git_ref: &GitRef) -> Result<(), String> {
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
    /// * `Result<String, String>` - JSON results or an error message
    ///
    /// # Examples
    ///
    /// ```
    /// let params = CodeSearchParams {
    ///     repository_location: "https://github.com/user/repo".parse()?,
    ///     ref_name: Some("main".to_string()),
    ///     pattern: "fn main".to_string(),
    ///     case_sensitive: false,
    ///     use_regex: true,
    ///     file_extensions: Some(vec!["rs".to_string()]),
    ///     exclude_dirs: Some(vec!["target".to_string()]),
    /// };
    ///
    /// let results = search_code(params).await?;
    /// ```
    pub async fn search_code(&self, _params: CodeSearchParams) -> Result<String, String> {
        //let repo_info = match self
        //    .repo_manager
        //    .parse_and_prepare_repository(&params.repository_location, params.ref_name.clone())
        //    .await
        //{
        //    Ok(info) => info,
        //    Err(e) => return Err(e),
        //};

        // Execute code search and return raw results
        // Temporarily returning empty string until code_search is fixed
        Ok("Code search feature temporarily disabled".to_string())
    }

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
    /// * `file_extensions` - Optional array of file extensions to include (e.g. ["js", "ts"])
    ///
    /// # Returns
    ///
    /// * `String` - JSON string of search results
    pub async fn perform_code_search(
        &self,
        _pattern: &str,
        _case_sensitive: bool,
        _use_regex: bool,
        _file_extensions: Option<Vec<String>>,
    ) -> Result<String, String> {
        // Implementation temporarily disabled
        Ok("Code search feature temporarily disabled".to_string())
    }

    /// Generate a 12-character hash value from repository information
    ///
    /// Creates a deterministic hash based on the user and repository name.
    /// This ensures that the same repository always gets the same hash value.
    fn generate_repository_hash(remote_repository_info: &GitRemoteRepositoryInfo) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Create a string combining user and repo
        let hash_input = format!(
            "{}{}",
            remote_repository_info.user, remote_repository_info.repo
        );

        // Hash the string to get a unique value
        let mut hasher = DefaultHasher::new();
        hash_input.hash(&mut hasher);
        let hash_value = hasher.finish();

        // Format as a 12-character hex string
        format!("{:012x}", hash_value)
    }
}