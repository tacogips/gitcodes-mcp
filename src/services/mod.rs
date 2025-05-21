use crate::gitcodes::local_repository::{CodeSearchParams, ViewFileParams};
use crate::gitcodes::repository_manager;
use crate::gitcodes::CodeSearchResult;
use repository_manager::RepositoryLocation;
use std::path::PathBuf;
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
    before_context: Option<usize>,
    after_context: Option<usize>,
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
        .prepare_repository(&repository_location, ref_name.map(String::from))
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
        before_context,
        after_context,
    };

    // Execute the grep operation
    let search_result = local_repo.search_code(params).await?;

    // Return both the search results and the local repository instance
    Ok((search_result, local_repo))
}

/// Shows the contents of a file within a repository, first preparing the repository if needed
///
/// This pure function handles the entire file viewing process:
/// 1. Parses a repository location string into a RepositoryLocation
/// 2. Prepares (clones if needed) the repository using the provided manager
/// 3. Creates view parameters with the provided options
/// 4. Retrieves the file contents from the local repository
///
/// The function is designed to have no side effects and does not access global state,
/// which makes it ideal for unit testing. All dependencies are explicitly passed as parameters.
///
/// # Parameters
///
/// * `repository_manager` - The repository manager for cloning/preparing repositories
/// * `repository_location_str` - The repository location string to parse (e.g., "github:user/repo" or "/path/to/local/repo")
/// * `file_path` - The path of the file within the repository to view
/// * `ref_name` - Optional reference name (branch/tag) to checkout
/// * `max_size` - Optional maximum file size to read (in bytes)
/// * `line_from` - Optional start line number (1-indexed)
/// * `line_to` - Optional end line number (1-indexed, inclusive)
///
/// # Returns
///
/// * `Result<(lumin::view::FileContents, repository_manager::LocalRepository), String>` - A tuple containing the file contents and the local repository instance
///
/// # Errors
///
/// This function returns an error if:
/// - The repository location string cannot be parsed
/// - The repository cannot be prepared (cloned or validated)
/// - The file does not exist in the repository
/// - The file is too large
/// - The file viewing operation fails
pub async fn show_file_contents(
    repository_manager: &repository_manager::RepositoryManager,
    repository_location_str: &str,
    file_path: String,
    ref_name: Option<&str>,
    max_size: Option<usize>,
    line_from: Option<usize>,
    line_to: Option<usize>,
    without_line_numbers: Option<bool>,
) -> Result<
    (
        lumin::view::FileContents,
        crate::gitcodes::local_repository::LocalRepository,
        bool, // Added parameter to return the effective use_line_numbers value
    ),
    String,
> {
    // Parse the repository location string
    let repository_location = RepositoryLocation::from_str(repository_location_str)
        .map_err(|e| format!("Failed to parse repository location: {}", e))?;

    // Prepare the repository (clone if necessary)
    let local_repo = repository_manager
        .prepare_repository(&repository_location, ref_name.map(String::from))
        .await?;

    // Create view parameters
    let params = ViewFileParams {
        file_path: PathBuf::from(file_path),
        max_size,
        line_from,
        line_to,
    };

    // View the file contents
    let file_contents = local_repo.view_file_contents(params).await?;
    
    // Determine the effective value for without_line_numbers (default to false if not specified)
    let effective_without_line_numbers = without_line_numbers.unwrap_or(false);

    // Return the file contents, local repository instance, and the format choice
    Ok((file_contents, local_repo, effective_without_line_numbers))
}


