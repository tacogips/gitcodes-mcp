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
    GLOBAL_REPOSITORY_MANAGER.get_or_init(RepositoryManager::with_default_cache_dir)
}
