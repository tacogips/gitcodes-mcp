//! Global RepositoryManager instance
//!
//! This module provides a global singleton instance of RepositoryManager using once_cell.
//! This ensures that all MCP calls within the same process use the same RepositoryManager
//! instance with the same process_id.

use once_cell::sync::OnceCell;
use std::path::PathBuf;

use super::RepositoryManager;

/// Global RepositoryManager instance
static GLOBAL_REPOSITORY_MANAGER: OnceCell<RepositoryManager> = OnceCell::new();

/// Initialize the global RepositoryManager instance with the given parameters
///
/// This function should be called once during process startup. If called multiple times,
/// only the first call will have an effect, and subsequent calls will be ignored.
///
/// # Parameters
///
/// * `github_token` - Optional GitHub token for authentication
/// * `repository_cache_dir` - Optional path for storing repositories
///
/// # Returns
///
/// The global RepositoryManager instance (either newly created or existing)
pub fn init_repository_manager(
    github_token: Option<String>,
    repository_cache_dir: Option<PathBuf>,
) -> &'static RepositoryManager {
    GLOBAL_REPOSITORY_MANAGER.get_or_init(move || {
        RepositoryManager::new(github_token, repository_cache_dir)
            .expect("Failed to initialize global repository manager")
    })
}

/// Get the global RepositoryManager instance
///
/// If the global instance hasn't been initialized yet, this function will
/// initialize it with default parameters.
///
/// # Returns
///
/// The global RepositoryManager instance
pub fn get_repository_manager() -> &'static RepositoryManager {
    GLOBAL_REPOSITORY_MANAGER
        .get_or_init(|| RepositoryManager::with_default_cache_dir())
}