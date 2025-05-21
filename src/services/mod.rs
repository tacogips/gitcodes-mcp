use crate::gitcodes::local_repository::{CodeSearchParams, ViewFileParams};
use crate::gitcodes::repository_manager;
use crate::gitcodes::CodeSearchResult;
use repository_manager::RepositoryLocation;
use std::path::PathBuf;
use std::str::FromStr;

/// Parameters for performing a grep operation in a repository
#[derive(Debug, Clone)]
pub struct GrepParams {
    pub repository_location_str: String,
    pub pattern: String,
    pub ref_name: Option<String>,
    pub case_sensitive: bool,
    pub file_extensions: Option<Vec<String>>,
    pub include_globs: Option<Vec<String>>,
    pub exclude_dirs: Option<Vec<String>>,
    pub before_context: Option<usize>,
    pub after_context: Option<usize>,
    pub skip: Option<usize>,
    pub take: Option<usize>,
    pub match_content_omit_num: Option<usize>,
}

/// Parameters for showing file contents in a repository
#[derive(Debug, Clone)]
pub struct ShowFileParams {
    pub repository_location_str: String,
    pub file_path: String,
    pub ref_name: Option<String>,
    pub max_size: Option<usize>,
    pub line_from: Option<usize>,
    pub line_to: Option<usize>,
    pub without_line_numbers: Option<bool>,
}

/// Parameters for getting repository tree structure
#[derive(Debug, Clone)]
pub struct TreeServiceParams {
    pub repository_location_str: String,
    pub ref_name: Option<String>,
    pub case_sensitive: Option<bool>,
    pub respect_gitignore: Option<bool>,
    pub depth: Option<usize>,
    pub strip_path_prefix: Option<bool>,
    pub search_relative_path: Option<PathBuf>,
}

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
/// * `file_extensions` - Optional list of file extensions to filter by (e.g., ["rs", "md"]) (deprecated, use include_globs instead)
/// * `include_globs` - Optional list of glob patterns to include files (e.g., ["**/*.rs", "**/*.md"]) (not exposed through this API yet)
/// * `exclude_dirs` - Optional list of directories to exclude (e.g., ["target", "node_modules"])
/// * `before_context` - Optional number of lines to include before each match
/// * `after_context` - Optional number of lines to include after each match
/// * `skip` - Optional number of results to skip (for pagination)
/// * `take` - Optional maximum number of results to return (for pagination, defaults to 50 if not specified)
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
    params: GrepParams,
) -> Result<
    (
        CodeSearchResult,
        crate::gitcodes::local_repository::LocalRepository,
    ),
    String,
> {
    // Parse the repository location string
    let repository_location = RepositoryLocation::from_str(&params.repository_location_str)
        .map_err(|e| format!("Failed to parse repository location: {}", e))?;

    // Prepare the repository (clone if necessary)
    let local_repo = repository_manager
        .prepare_repository(&repository_location, params.ref_name.clone())
        .await?;

    // Use the pattern as provided - the caller is responsible for any regex escaping

    // Create search parameters directly as CodeSearchParams
    let search_params = CodeSearchParams {
        repository_location: repository_location.clone(),
        ref_name: params.ref_name,
        pattern: params.pattern.clone(), // Already processed for regex if needed
        case_sensitive: params.case_sensitive,
        file_extensions: params.file_extensions.clone(),
        include_globs: params.include_globs.clone(),
        exclude_dirs: params.exclude_dirs.clone(),
        before_context: params.before_context,
        after_context: params.after_context,
        skip: params.skip, // Allow pagination through service API
        take: params.take.or(Some(50)), // Default to 50 if not specified
        match_content_omit_num: params.match_content_omit_num.or(Some(150)), // Default to 150 if not specified
    };

    // Execute the grep operation
    let search_result = local_repo.search_code(search_params).await?;

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
    params: ShowFileParams,
) -> Result<
    (
        lumin::view::FileContents,
        crate::gitcodes::local_repository::LocalRepository,
        bool, // Added parameter to return the effective use_line_numbers value
    ),
    String,
> {
    // Parse the repository location string
    let repository_location = RepositoryLocation::from_str(&params.repository_location_str)
        .map_err(|e| format!("Failed to parse repository location: {}", e))?;

    // Prepare the repository (clone if necessary)
    let local_repo = repository_manager
        .prepare_repository(&repository_location, params.ref_name)
        .await?;

    // Set up view parameters
    let view_params = ViewFileParams {
        file_path: PathBuf::from(params.file_path),
        max_size: params.max_size,
        line_from: params.line_from,
        line_to: params.line_to,
    };

    // View the file contents
    let file_contents = local_repo.view_file_contents(view_params).await?;

    // Determine the effective value for without_line_numbers (default to false if not specified)
    let effective_without_line_numbers = params.without_line_numbers.unwrap_or(false);

    // Return the file contents, local repository instance, and the format choice
    Ok((file_contents, local_repo, effective_without_line_numbers))
}

/// Shows the directory tree structure of a repository, first preparing the repository if needed
///
/// This pure function handles the entire tree viewing process:
/// 1. Parses a repository location string into a RepositoryLocation
/// 2. Prepares (clones if needed) the repository using the provided manager
/// 3. Creates tree parameters with the provided options
/// 4. Retrieves the directory tree from the local repository
///
/// The function is designed to have no side effects and does not access global state,
/// which makes it ideal for unit testing. All dependencies are explicitly passed as parameters.
///
/// # Parameters
///
/// * `repository_manager` - The repository manager for cloning/preparing repositories
/// * `repository_location_str` - The repository location string to parse (e.g., "github:user/repo" or "/path/to/local/repo")
/// * `ref_name` - Optional reference name (branch/tag) to checkout
/// * `case_sensitive` - Optional whether file path matching should be case sensitive (default: false)
/// * `respect_gitignore` - Optional whether to respect .gitignore files (default: true)
/// * `depth` - Optional maximum depth of directory traversal (default: unlimited)
/// * `strip_path_prefix` - Optional whether to strip the repository path prefix from results (default: true)
///
/// # Returns
///
/// * `Result<(Vec<crate::gitcodes::local_repository::RepositoryTree>, repository_manager::LocalRepository), String>` - A tuple containing the directory tree and the local repository instance
///
/// # Errors
///
/// This function returns an error if:
/// - The repository location string cannot be parsed
/// - The repository cannot be prepared (cloned or validated)
/// - The tree generation operation fails
pub async fn get_repository_tree(
    repository_manager: &repository_manager::RepositoryManager,
    params: TreeServiceParams,
) -> Result<
    (
        Vec<crate::gitcodes::local_repository::RepositoryTree>,
        crate::gitcodes::local_repository::LocalRepository,
    ),
    String,
> {
    // Parse the repository location string
    let repository_location = RepositoryLocation::from_str(&params.repository_location_str)
        .map_err(|e| format!("Failed to parse repository location: {}", e))?;

    // Prepare the repository (clone if necessary)
    let local_repo = repository_manager
        .prepare_repository(&repository_location, params.ref_name)
        .await?;

    // Create tree parameters
    let tree_params = crate::gitcodes::local_repository::TreeParams {
        case_sensitive: params.case_sensitive,
        respect_gitignore: params.respect_gitignore,
        depth: params.depth,
        strip_path_prefix: params.strip_path_prefix,
        search_relative_path: params.search_relative_path,
    };

    // Get the directory tree
    let tree = local_repo.get_tree_with_params(Some(tree_params)).await?;

    Ok((tree, local_repo))
}
