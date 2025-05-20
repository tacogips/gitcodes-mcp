use crate::gitcodes::local_repository::CodeSearchParams;
use crate::gitcodes::repository_manager;
use crate::gitcodes::CodeSearchResult;
use crate::gitcodes::local_repository::LocalRepository;
use repository_manager::RepositoryLocation;
use repository_manager::providers::GitRemoteRepository;
use std::str::FromStr;


/// Performs a grep-like code search within a repository, first preparing the repository if needed
///
/// This pure function handles the entire grep process:
/// 1. Parses a repository location string into a RepositoryLocation
/// 2. Prepares (clones if needed) the repository using the provided manager
/// 3. Creates search parameters with the provided options
/// 4. Executes the code search against the local repository
///
/// The function is designed to have no side effects and does not access global state,
/// which makes it ideal for unit testing. All dependencies are explicitly passed as parameters.
///
/// # Parameters
///
/// * `repository_manager` - The repository manager for cloning/preparing repositories
/// * `repository_location_str` - The repository location string to parse (e.g., "github:user/repo" or "/path/to/local/repo")
/// * `pattern` - The search pattern (already processed for regex escaping if needed)
/// * `ref_name` - Optional reference name (branch/tag) to checkout
/// * `case_sensitive` - Whether to perform a case-sensitive search
/// * `file_extensions` - Optional list of file extensions to filter by (e.g., ["rs", "md"])
/// * `exclude_dirs` - Optional list of directories to exclude (e.g., ["target", "node_modules"])
///
/// # Returns
///
/// * `Result<(CodeSearchResult, repository_manager::LocalRepository), String>` - A tuple containing the search results and the local repository instance
///
/// # Errors
///
/// This function returns an error if:
/// - The repository location string cannot be parsed
/// - The repository cannot be prepared (cloned or validated)
/// - The code search operation fails
pub async fn perform_grep_in_repository(
    repository_manager: &repository_manager::RepositoryManager,
    repository_location_str: &str,
    pattern: String,
    ref_name: Option<&str>,
    case_sensitive: bool,
    file_extensions: Option<&Vec<String>>,
    exclude_dirs: Option<&Vec<String>>,
) -> Result<
    (
        CodeSearchResult,
        crate::gitcodes::local_repository::LocalRepository,
    ),
    String,
> {
    // Parse the repository location string
    let repository_location = RepositoryLocation::from_str(repository_location_str)
        .map_err(|e| format!("Failed to parse repository location: {}", e))?;

    // Prepare the repository (clone if necessary)
    let local_repo = repository_manager
        .prepare_repository(repository_location.clone(), ref_name.map(String::from))
        .await?;

    // Use the pattern as provided - the caller is responsible for any regex escaping

    // Create search parameters directly as CodeSearchParams
    let params = CodeSearchParams {
        repository_location: repository_location.clone(),
        ref_name: ref_name.map(String::from),
        pattern, // Already processed for regex if needed
        case_sensitive,
        file_extensions: file_extensions.cloned(),
        exclude_dirs: exclude_dirs.cloned(),
    };

    // Execute the grep operation
    let search_result = local_repo.search_code(params).await?;

    // Return both the search results and the local repository instance
    Ok((search_result, local_repo))
}

/// Lists all references (branches and tags) for a given repository using the GitHub API
///
/// This pure function handles the entire refs listing process:
/// 1. Parses a repository location string into a RepositoryLocation
/// 2. For GitHub repositories, uses the GitHub API to fetch refs
/// 3. For local repositories, uses git commands to list refs
///
/// # Parameters
///
/// * `repository_manager` - The repository manager for accessing repositories
/// * `repository_location_str` - The repository location string to parse (e.g., "github:user/repo" or "/path/to/local/repo")
///
/// # Returns
///
/// * `Result<(String, Option<LocalRepository>), String>` - A tuple containing the JSON results string and optionally a local repository reference
///
/// # Errors
///
/// This function returns an error if:
/// - The repository location string cannot be parsed
/// - The repository cannot be accessed or prepared
/// - The API request fails (for GitHub repositories)
/// - The git command fails (for local repositories)
pub async fn list_repository_refs(
    repository_manager: &repository_manager::RepositoryManager, 
    repository_location_str: &str
) -> Result<(String, Option<LocalRepository>), String> {
    // Parse the repository location string
    let repository_location = RepositoryLocation::from_str(repository_location_str)
        .map_err(|e| format!("Failed to parse repository location: {}", e))?;
    
    // Different handling based on repository type
    match &repository_location {
        RepositoryLocation::RemoteRepository(remote_repo) => {
            // Currently only GitHub repositories are supported
            match remote_repo {
                GitRemoteRepository::Github(github_repo_info) => {
                    // For GitHub repositories, use the GitHub API
                    let github_client = repository_manager.get_github_client();
                    let refs_json = github_client.list_repository_refs(&github_repo_info.repo_info).await?;
                    
                    // Return the JSON result without a local repository reference
                    Ok((refs_json, None))
                }
            }
        },
        RepositoryLocation::LocalPath(_local_path) => {
            // For local repositories, prepare the repository and use git commands
            let local_repo = repository_manager
                .prepare_repository(repository_location.clone(), None)
                .await?;
                
            // Use the local repository to list refs
            let refs_json = local_repo.list_repository_refs(repository_location.clone()).await;
            
            // Return both the JSON results and the local repository reference
            Ok((refs_json, Some(local_repo)))
        }
    }
}
