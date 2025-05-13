use gix;
use gix::bstr::ByteSlice;
use gix::progress::Discard;
use lumin::search;
use rand::Rng;
use rmcp::schemars;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use thiserror::Error;
use uuid;

use super::{code_search, RemoteGitRepositoryInfo, RepositoryLocation};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct LocalRepository {
    repository_location: PathBuf,
}
impl LocalRepository {
    /// Generate a 12-character hash value from repository information
    ///
    /// Creates a deterministic hash based on the user and repository name.
    /// This ensures that the same repository always gets the same hash value.
    fn generate_repository_hash(remote_repository_info: &RemoteGitRepositoryInfo) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Create a string combining user and repo
        let repo_key = format!("{}/{}", remote_repository_info.user, remote_repository_info.repo);
        
        // Create a hash using DefaultHasher
        let mut hasher = DefaultHasher::new();
        repo_key.hash(&mut hasher);
        let hash_value = hasher.finish();
        
        // Convert to a 12-character hexadecimal string
        // We'll take 12 characters from the hex representation
        let hex = format!("{:x}", hash_value);
        
        // Ensure we have at least 12 characters
        if hex.len() >= 12 {
            hex[0..12].to_string()
        } else {
            // Pad with zeros if needed (unlikely with a 64-bit hash)
            format!("{:0>12}", hex)
        }
    }
    /// if this validation is failed, it may means it just not cloned the git repository yet, otherwise someting wrong
    pub fn validate(&self) -> Result<(), String> {
        // For local paths, use the path directly
        if !self.repository_location.is_dir() {
            return Err(format!(
                "Local path '{}' is not a directory",
                self.repository_location.display()
            ));
        }
        //TODO(check is git)
        Ok(())
    }

    /// Generate a unique directory name for the repository based on its information
    pub fn new_local_repository_to_clone(remote_repository_info: RemoteGitRepositoryInfo) -> Self {
        let hash_value = Self::generate_repository_hash(&remote_repository_info);
        let dir_name = format!("mcp_gitcodes_{}_{}_{}", 
            remote_repository_info.user, 
            remote_repository_info.repo, 
            hash_value
        );

        Self::new(PathBuf::from(dir_name))
    }

    /// Generate a unique directory name for the repository
    pub fn new(repository_location: PathBuf) -> Self {
        //TODO(tacogips) check the path is valid
        Self {
            repository_location,
        }
    }

    /// Check if repository is already cloned
    async fn is_exists(&self, dir: &Path) -> bool {
        tokio::fs::metadata(dir).await.is_ok()
        //TODO(tacogips) check is git repository
    }

    /// List branches and tags for a GitHub repository or local git directory
    ///
    /// This tool retrieves a list of all branches and tags for the specified repository.
    /// It supports both public and private repositories as well as local git directories.
    ///
    /// # Authentication
    ///
    /// - For public repositories: No authentication needed
    /// - For private repositories: Requires `GITCODE_MCP_GITHUB_TOKEN` with `repo` scope
    /// - For local directories: No authentication needed
    ///
    /// # Implementation Note
    ///
    /// This tool:
    /// 1. Clones or updates the repository locally (for GitHub URLs) or uses the local directory directly
    /// 2. Fetches all branches and tags
    /// 3. Formats the results into a readable format
    pub async fn list_repository_refs(&self, repository_location: RepositoryLocation) -> String {
        //// Parse repository information from URL or local path
        //let repo_info = match self
        //    .parse_and_prepare_repository(&repository_location ))
        //    .await
        //{
        //    Ok(info) => info,
        //    Err(e) => return e,
        //};

        //// Fetch repository refs using the extracted function
        //match fetch_repository_refs(&repo_info.repo_dir, &repo_info.user, &repo_info.repo).await {
        //    Ok(result) => result,
        //    Err(e) => format!("Failed to list refs: {}", e),
        //}
        unimplemented!()
    }

    // parse_and_prepare_repository method has been moved to git_repository.rs

    // Code search methods have been moved to code_search.rs

    // Function to fetch repository refs (branches and tags)
    async fn fetch_repository_refs(&self) -> Result<String, String> {
        //unimplemented!()
        // Change to the repository directory
        //let current_dir = match std::env::current_dir() {
        //    Ok(dir) => dir,
        //    Err(e) => return Err(format!("Failed to get current directory: {}", e)),
        //};

        //if let Err(e) = std::env::set_current_dir(&self.repository_cache_dir_base) {
        //    return Err(format!("Failed to change directory: {}", e));
        //}

        //// First run git fetch to make sure we have all refs
        //let fetch_status = std::process::Command::new("git")
        //    .args(["fetch", "--all"])
        //    .status();

        //if let Err(e) = fetch_status {
        //    let _ = std::env::set_current_dir(current_dir);
        //    return Err(format!("Git fetch failed: {}", e));
        //}

        //if !fetch_status.unwrap().success() {
        //    let _ = std::env::set_current_dir(current_dir);
        //    return Err("Git fetch failed".to_string());
        //}

        //// Get branches
        //let branches_output = std::process::Command::new("git")
        //    .args(["branch", "-r"])
        //    .output();

        //let branches_output = match branches_output {
        //    Ok(output) => output,
        //    Err(e) => {
        //        let _ = std::env::set_current_dir(current_dir);
        //        return Err(format!("Failed to list branches: {}", e));
        //    }
        //};

        //let branches_str = String::from_utf8_lossy(&branches_output.stdout).to_string();

        //// Get tags
        //let tags_output = std::process::Command::new("git").args(["tag"]).output();

        //let tags_output = match tags_output {
        //    Ok(output) => output,
        //    Err(e) => {
        //        let _ = std::env::set_current_dir(current_dir);
        //        return Err(format!("Failed to list tags: {}", e));
        //    }
        //};

        //let tags_str = String::from_utf8_lossy(&tags_output.stdout).to_string();

        //// Change back to the original directory
        //if let Err(e) = std::env::set_current_dir(current_dir) {
        //    return Err(format!("Failed to restore directory: {}", e));
        //}

        //// Format the output
        //let mut result = String::new();
        //result.push_str(&format!(
        //    "Repository: {}/{}

        //",
        //    user_clone, repo_clone
        //));

        //// Extract and format branches
        //let branches: Vec<String> = branches_str
        //    .lines()
        //    .filter_map(|line| {
        //        let line = line.trim();
        //        if line.starts_with("origin/") && !line.contains("HEAD") {
        //            Some(line.trim_start_matches("origin/").to_string())
        //        } else {
        //            None
        //        }
        //    })
        //    .collect();

        //// Extract and format tags
        //let tags: Vec<String> = tags_str
        //    .lines()
        //    .map(|line| line.trim().to_string())
        //    .filter(|line| !line.is_empty())
        //    .collect();

        //// Add branches section
        //result.push_str(
        //    "## Branches
        //z",
        //z    //);
        //z    //if branches.is_empty() {
        //z    //    result.push_str(
        //z    //        "No branches found
        //z",
        //    );
        //} else {
        //    for branch in branches {
        //        result.push_str(&format!("- {}\n", branch));
        //    }
        //}

        //// Add tags section
        //result.push_str(
        //    " ## Tags ",
        //);
        //if tags.is_empty() {
        //    result.push_str(
        //        "No tags found
        //zz",
        //    );
        //} else {
        //    for tag in tags {
        //        result.push_str(&format!("- {}\n", tag));
        //    }
        //}

        //Ok(result)
        unimplemented!()
    }

    /// Update an existing repository
    ///
    /// Fetches the latest changes and checks out the specified branch/tag.
    ///
    /// # Parameters
    ///
    /// * `repo_dir` - The directory containing the repository
    /// * `git_ref` - Branch or tag name to checkout as a GitRef
    async fn update_repository(&self, git_ref: &GitRef) -> Result<(), String> {
        let repo_dir = self.repository_cache_dir_base;
        // Open the existing repository
        let repo = gix::open(repo_dir).map_err(|e| format!("Failed to open repository: {}", e))?;

        // Find the origin remote
        let remote = repo
            .find_remote("origin")
            .map_err(|e| format!("Could not find origin remote: {}", e))?;

        // Configure fetch operation
        let depth = NonZeroU32::new(1).unwrap();
        let shallow = gix::remote::fetch::Shallow::DepthAtRemote(depth);

        // Prepare the fetch params
        let mut remote_ref_specs = Vec::new(); // Empty means fetch default refs
        let progress = Discard;

        // Create a transport for the fetch
        let transport = remote
            .connect(gix::remote::Direction::Fetch)
            .map_err(|e| format!("Failed to connect to remote: {}", e))?;

        // Create fetch delegate with our shallow config
        let mut delegate = transport.new_fetch_delegate();
        delegate.shallow_setting = Some(shallow);

        // Perform the fetch
        let fetch_outcome = delegate
            .fetch(&remote_ref_specs, &progress)
            .map_err(|e| format!("Fetch failed: {}", e))?;

        // We don't need the fetch outcome details, just check for success
        let _ = fetch_outcome;

        // Try to find the reference directly (local branch)
        let local_ref_name = format!("refs/heads/{}", git_ref.as_str());
        let maybe_ref = repo.try_find_reference(&local_ref_name);
        if let Ok(Some(mut reference)) = maybe_ref {
            // Reference exists, try to follow and peel it
            if reference.peel_to_id_in_place().is_ok() {
                return Ok(());
            }
        }

        // Try with origin/ prefix if direct reference wasn't found
        let origin_ref_name = format!("refs/remotes/origin/{}", git_ref.as_str());
        let maybe_origin_ref = repo.try_find_reference(&origin_ref_name);
        if let Ok(Some(mut reference)) = maybe_origin_ref {
            // Origin reference exists, try to follow and peel it
            if reference.peel_to_id_in_place().is_ok() {
                return Ok(());
            }
        }

        // If we're looking for a tag
        let tag_ref_name = format!("refs/tags/{}", git_ref.as_str());
        let maybe_tag_ref = repo.try_find_reference(&tag_ref_name);
        if let Ok(Some(mut reference)) = maybe_tag_ref {
            // Tag reference exists, try to follow and peel it
            if reference.peel_to_id_in_place().is_ok() {
                return Ok(());
            }
        }

        Err(format!("Branch/tag not found: {}", git_ref.as_str()))
    }

    /// Search code in a GitHub repository or local directory
    ///
    /// This tool clones or updates the repository locally (for GitHub URLs) or uses
    /// the local directory directly (for file paths), then performs a code search
    /// using the specified pattern. It supports both public and private repositories.
    ///
    /// # Authentication
    ///
    /// - For public repositories: No authentication needed
    /// - For private repositories: Requires `GITCODE_MCP_GITHUB_TOKEN` with `repo` scope
    /// - For local directories: No authentication needed
    ///
    /// # Implementation Note
    ///
    /// This tool uses a combination of git operations and the lumin search library:
    /// 1. Repository is cloned or updated locally (for GitHub URLs) or a local directory is used directly
    /// 2. Code search is performed on the files
    /// 3. Returns raw search results without additional formatting
    pub async fn grep_repository(&self, params: GrepParams) -> Result<String, String> {
        //// Repository location is already in the correct type
        //// Parse repository information from URL or local path
        //let repo_info = match self
        //    .repo_manager
        //    .parse_and_prepare_repository(&params.repository_location, params.ref_name.clone())
        //    .await
        //{
        //    Ok(info) => info,
        //    Err(e) => return Err(e),
        //};

        // Execute code search and return raw results
        code_search::perform_code_search(
            &repo_info.repo_dir,
            &params.pattern,
            params.case_sensitive,
            params.use_regex,
            params.file_extensions.clone(),
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
    /// * `repo_dir` - The directory containing the repository
    /// * `pattern` - The search pattern to look for
    /// * `case_sensitive` - Whether the search should be case-sensitive (default: false)
    /// * `_use_regex` - Whether to use regex for the search (not currently implemented)
    /// * `_file_extensions` - Filter by file extensions (not currently implemented)
    pub async fn perform_code_search(
        &self,
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
