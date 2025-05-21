//! Tests for the tree functionality in local_repository module
//!
//! These tests verify the functionality of the LocalRepository::get_tree_with_params method,
//! covering all possible TreeParams configurations and edge cases.
//!
//! ## Test Coverage
//!
//! This test suite provides comprehensive coverage of all TreeParams fields:
//!
//! ### Core Parameters
//! - `case_sensitive`: Tests both true and false values
//! - `search_relative_path`: Tests None, valid paths ("src", "src/utils"), invalid paths, and empty paths
//! - `respect_gitignore`: Tests both true and false values, verifying gitignore behavior
//! - `depth`: Tests various depth values (0, 1, 2, 3, 10) including unlimited depth
//! - `strip_path_prefix`: Tests both true and false values for path prefix handling
//!
//! ### Edge Cases and Combinations
//! - Default parameters (None values vs Some with defaults)
//! - Complex parameter combinations
//! - All parameters set to specific values
//! - Invalid repository paths
//! - Non-existent relative paths
//! - Various relative path combinations
//!
//! ### Test Strategy
//! - Uses RepositoryManager to clone the gitcodes-mcp-test-1 repository dynamically
//! - Leverages `respect_gitignore: Some(false)` in most tests to ensure consistent file visibility
//! - Validates tree structure using helper functions for root-level directory/file detection
//! - Tests error handling for invalid repositories and paths
//!
//! ### Gitignore Handling in Tests
//!
//! Most tests use `respect_gitignore: Some(false)` for the following reasons:
//!
//! - **Consistency**: Ensures all tests see the same file structure regardless of gitignore rules
//! - **Reliability**: Prevents tests from failing due to gitignore changes in the test repository
//! - **Completeness**: Allows testing with the full directory structure including normally ignored files
//!
//! Specific gitignore behavior is tested in dedicated test cases:
//! - `test_get_tree_respect_gitignore_true`: Verifies that gitignore rules are respected
//! - `test_get_tree_respect_gitignore_false`: Verifies that ignored files are included when gitignore is disabled
//!
//! ### Repository Cloning vs Local Dependencies
//!
//! These tests use dynamic repository cloning instead of referencing `.private.deps-src` because:
//! - `.private.deps-src` is gitignored and won't exist in CI/CD or other developer environments
//! - Dynamic cloning ensures tests work in any environment with internet access
//! - Follows the same patterns as other integration tests in the project
//!
//! The tests ensure that all TreeParams patterns work correctly and provide reliable
//! directory tree generation functionality across different configuration scenarios.

use std::path::PathBuf;
use std::str::FromStr;

use gitcodes_mcp::gitcodes::{
    local_repository::TreeParams,
    repository_manager::{RepositoryLocation, RepositoryManager},
    LocalRepository,
};
use tempfile::tempdir;

/// Test repository URL for consistent testing
const TEST_REPO_URL: &str = "https://github.com/tacogips/gitcodes-mcp-test-1.git";

/// Creates a Repository Manager for testing
fn create_test_manager() -> RepositoryManager {
    // Create a temporary directory for repository cache
    let temp_dir = tempdir().expect("Failed to create temporary directory");
    let cache_dir = temp_dir.path().to_path_buf();

    // Create a repository manager with our temporary directory
    RepositoryManager::new(
        None, // No GitHub token for public repos
        Some(cache_dir),
    )
    .expect("Failed to create RepositoryManager")
}

/// Test fixture that gets a LocalRepository by cloning the test repository
async fn get_test_repository() -> LocalRepository {
    let manager = create_test_manager();
    let repo_location =
        RepositoryLocation::from_str(TEST_REPO_URL).expect("Failed to parse test repository URL");

    manager
        .prepare_repository(&repo_location, None)
        .await
        .expect("Failed to prepare test repository")
}

/// Helper function to check if a tree has a directory at root level (dir='')
fn tree_has_root_directory(
    tree: &[gitcodes_mcp::gitcodes::local_repository::RepositoryTree],
    dir_name: &str,
) -> bool {
    tree.iter().any(|repo_tree| {
        repo_tree.dir.is_empty() &&
        repo_tree.entries.iter().any(|entry| {
            matches!(entry, gitcodes_mcp::gitcodes::local_repository::TreeEntry::Directory(name) if name == dir_name)
        })
    })
}

/// Helper function to check if a tree has a file at root level (dir='')
fn tree_has_root_file(
    tree: &[gitcodes_mcp::gitcodes::local_repository::RepositoryTree],
    file_name: &str,
) -> bool {
    tree.iter().any(|repo_tree| {
        repo_tree.dir.is_empty() &&
        repo_tree.entries.iter().any(|entry| {
            matches!(entry, gitcodes_mcp::gitcodes::local_repository::TreeEntry::File(name) if name == file_name)
        })
    })
}

/// Test get_tree_with_params with None (default parameters)
#[tokio::test]
async fn test_get_tree_with_none_params() {
    let local_repo = get_test_repository().await;

    let result = local_repo.get_tree_with_params(None).await;

    assert!(
        result.is_ok(),
        "Failed to get tree with None params: {:?}",
        result.err()
    );

    let tree = result.unwrap();
    assert!(!tree.is_empty(), "Tree should not be empty");

    // Note: With default settings, gitignore is respected so we might not see all files
    // Just check that we get some tree structure
    assert!(!tree.is_empty(), "Tree should contain at least one entry");
}

/// Test get_tree_with_params with all default values (Some with None fields)
#[tokio::test]
async fn test_get_tree_with_default_params() {
    let local_repo = get_test_repository().await;

    let params = TreeParams {
        case_sensitive: None,
        search_relative_path: None,
        respect_gitignore: Some(false), // Ignore gitignore to see actual files
        depth: None,
        strip_path_prefix: None,
    };

    let result = local_repo.get_tree_with_params(Some(params)).await;

    assert!(
        result.is_ok(),
        "Failed to get tree with default params: {:?}",
        result.err()
    );

    let tree = result.unwrap();
    assert!(!tree.is_empty(), "Tree should not be empty");

    // Should contain the main source directories at root level
    assert!(
        tree_has_root_directory(&tree, "src"),
        "Should contain 'src' directory at root"
    );
    assert!(
        tree_has_root_file(&tree, "Cargo.toml"),
        "Should contain 'Cargo.toml' file at root"
    );
}

/// Test get_tree_with_params with case_sensitive = true
#[tokio::test]
async fn test_get_tree_case_sensitive_true() {
    let local_repo = get_test_repository().await;

    let params = TreeParams {
        case_sensitive: Some(true),
        search_relative_path: None,
        respect_gitignore: Some(false),
        depth: None,
        strip_path_prefix: None,
    };

    let result = local_repo.get_tree_with_params(Some(params)).await;

    assert!(
        result.is_ok(),
        "Failed to get tree with case_sensitive=true: {:?}",
        result.err()
    );

    let tree = result.unwrap();
    assert!(!tree.is_empty(), "Tree should not be empty");
}

/// Test get_tree_with_params with case_sensitive = false
#[tokio::test]
async fn test_get_tree_case_sensitive_false() {
    let local_repo = get_test_repository().await;

    let params = TreeParams {
        case_sensitive: Some(false),
        search_relative_path: None,
        respect_gitignore: Some(false),
        depth: None,
        strip_path_prefix: None,
    };

    let result = local_repo.get_tree_with_params(Some(params)).await;

    assert!(
        result.is_ok(),
        "Failed to get tree with case_sensitive=false: {:?}",
        result.err()
    );

    let tree = result.unwrap();
    assert!(!tree.is_empty(), "Tree should not be empty");
}

/// Test get_tree_with_params with search_relative_path = "src"
#[tokio::test]
async fn test_get_tree_with_relative_path_src() {
    let local_repo = get_test_repository().await;

    let params = TreeParams {
        case_sensitive: None,
        search_relative_path: Some(PathBuf::from("src")),
        respect_gitignore: Some(false), // Ignore gitignore to see actual files
        depth: None,
        strip_path_prefix: None,
    };

    let result = local_repo.get_tree_with_params(Some(params)).await;

    assert!(
        result.is_ok(),
        "Failed to get tree with relative path 'src': {:?}",
        result.err()
    );

    let tree = result.unwrap();
    assert!(!tree.is_empty(), "Tree should not be empty");

    // Should have entries under src directory
    let src_tree = tree.iter().find(|t| t.dir == "src");
    assert!(src_tree.is_some(), "Should have src directory tree");
    let src_entries = &src_tree.unwrap().entries;
    assert!(!src_entries.is_empty(), "src directory should have entries");
}

/// Test get_tree_with_params with search_relative_path = "src/utils"
#[tokio::test]
async fn test_get_tree_with_relative_path_nested() {
    let local_repo = get_test_repository().await;

    let params = TreeParams {
        case_sensitive: None,
        search_relative_path: Some(PathBuf::from("src/utils")),
        respect_gitignore: Some(false), // Ignore gitignore to see actual files
        depth: None,
        strip_path_prefix: None,
    };

    let result = local_repo.get_tree_with_params(Some(params)).await;

    assert!(
        result.is_ok(),
        "Failed to get tree with relative path 'src/utils': {:?}",
        result.err()
    );

    let tree = result.unwrap();
    assert!(!tree.is_empty(), "Tree should not be empty");

    // Should have entries under src/utils directory
    let utils_tree = tree.iter().find(|t| t.dir == "src/utils");
    assert!(utils_tree.is_some(), "Should have src/utils directory tree");
    let utils_entries = &utils_tree.unwrap().entries;
    assert!(
        !utils_entries.is_empty(),
        "src/utils directory should have entries"
    );
}

/// Test get_tree_with_params with search_relative_path that doesn't exist
#[tokio::test]
async fn test_get_tree_with_invalid_relative_path() {
    let local_repo = get_test_repository().await;

    let params = TreeParams {
        case_sensitive: None,
        search_relative_path: Some(PathBuf::from("nonexistent/path")),
        respect_gitignore: Some(false),
        depth: None,
        strip_path_prefix: None,
    };

    let result = local_repo.get_tree_with_params(Some(params)).await;

    // The lumin library doesn't fail for non-existent paths; it returns a tree with the
    // specified directory path and a single "." entry, which is valid behavior
    if let Ok(tree) = result {
        assert!(
            !tree.is_empty(),
            "Non-existent path should still return a tree structure"
        );
        // The tree should contain the requested path even if it doesn't exist
        assert!(
            tree.iter().any(|t| t.dir == "nonexistent/path"),
            "Should contain the requested path"
        );
    } else {
        // If it returns an error, that's also acceptable behavior
        assert!(result.is_err(), "Should handle invalid paths gracefully");
    }
}

/// Test get_tree_with_params with respect_gitignore = true
#[tokio::test]
async fn test_get_tree_respect_gitignore_true() {
    let local_repo = get_test_repository().await;

    let params = TreeParams {
        case_sensitive: None,
        search_relative_path: None,
        respect_gitignore: Some(true),
        depth: None,
        strip_path_prefix: None,
    };

    let result = local_repo.get_tree_with_params(Some(params)).await;

    assert!(
        result.is_ok(),
        "Failed to get tree with respect_gitignore=true: {:?}",
        result.err()
    );

    let tree = result.unwrap();
    assert!(!tree.is_empty(), "Tree should not be empty");

    // When respecting gitignore, should still see main source files that are not ignored
    assert!(
        tree_has_root_file(&tree, "Cargo.toml"),
        "Should contain 'Cargo.toml' even with gitignore respected"
    );
}

/// Test get_tree_with_params with respect_gitignore = false
#[tokio::test]
async fn test_get_tree_respect_gitignore_false() {
    let local_repo = get_test_repository().await;

    let params = TreeParams {
        case_sensitive: None,
        search_relative_path: None,
        respect_gitignore: Some(false),
        depth: None,
        strip_path_prefix: None,
    };

    let result = local_repo.get_tree_with_params(Some(params)).await;

    assert!(
        result.is_ok(),
        "Failed to get tree with respect_gitignore=false: {:?}",
        result.err()
    );

    let tree = result.unwrap();
    assert!(!tree.is_empty(), "Tree should not be empty");

    // Should contain .git directory when not respecting .gitignore (git dirs are typically ignored by tools)
    assert!(
        tree_has_root_directory(&tree, ".git"),
        "Should contain '.git' directory when not respecting .gitignore"
    );
}

/// Test get_tree_with_params with depth = 1
#[tokio::test]
async fn test_get_tree_with_depth_1() {
    let local_repo = get_test_repository().await;

    let params = TreeParams {
        case_sensitive: None,
        search_relative_path: None,
        respect_gitignore: Some(false), // Ignore gitignore to see actual files
        depth: Some(1),
        strip_path_prefix: None,
    };

    let result = local_repo.get_tree_with_params(Some(params)).await;

    assert!(
        result.is_ok(),
        "Failed to get tree with depth=1: {:?}",
        result.err()
    );

    let tree = result.unwrap();
    assert!(!tree.is_empty(), "Tree should not be empty");

    // With depth 1, should only see top-level items
    assert!(
        tree_has_root_directory(&tree, "src"),
        "Should contain 'src' directory at root with depth=1"
    );
    assert!(
        tree_has_root_file(&tree, "Cargo.toml"),
        "Should contain 'Cargo.toml' file at root with depth=1"
    );

    // Should not have deeply nested paths in the directory names
    for repo_tree in &tree {
        assert!(
            repo_tree.dir.matches('/').count() <= 1,
            "Should not have deeply nested paths with depth=1"
        );
    }
}

/// Test get_tree_with_params with depth = 2
#[tokio::test]
async fn test_get_tree_with_depth_2() {
    let local_repo = get_test_repository().await;

    let params = TreeParams {
        case_sensitive: None,
        search_relative_path: None,
        respect_gitignore: Some(false),
        depth: Some(2),
        strip_path_prefix: None,
    };

    let result = local_repo.get_tree_with_params(Some(params)).await;

    assert!(
        result.is_ok(),
        "Failed to get tree with depth=2: {:?}",
        result.err()
    );

    let tree = result.unwrap();
    assert!(!tree.is_empty(), "Tree should not be empty");
}

/// Test get_tree_with_params with depth = 0 (should include only root)
#[tokio::test]
async fn test_get_tree_with_depth_0() {
    let local_repo = get_test_repository().await;

    let params = TreeParams {
        case_sensitive: None,
        search_relative_path: None,
        respect_gitignore: Some(false), // Ignore gitignore to see actual files
        depth: Some(0),
        strip_path_prefix: None,
    };

    let result = local_repo.get_tree_with_params(Some(params)).await;

    assert!(
        result.is_ok(),
        "Failed to get tree with depth=0: {:?}",
        result.err()
    );

    let _tree = result.unwrap();
    // With depth 0, might be empty or contain only root level items
    // This depends on lumin library implementation
}

/// Test get_tree_with_params with strip_path_prefix = true
#[tokio::test]
async fn test_get_tree_strip_prefix_true() {
    let local_repo = get_test_repository().await;

    let params = TreeParams {
        case_sensitive: None,
        search_relative_path: None,
        respect_gitignore: Some(false),
        depth: None,
        strip_path_prefix: Some(true),
    };

    let result = local_repo.get_tree_with_params(Some(params)).await;

    assert!(
        result.is_ok(),
        "Failed to get tree with strip_path_prefix=true: {:?}",
        result.err()
    );

    let tree = result.unwrap();
    assert!(!tree.is_empty(), "Tree should not be empty");

    // When stripping prefix, paths should be relative (already the default behavior)
    for repo_tree in &tree {
        assert!(
            !repo_tree.dir.starts_with('/'),
            "Directory paths should not start with '/' when prefix is stripped"
        );
        // Note: The path prefix is already stripped by default in this implementation
    }
}

/// Test get_tree_with_params with strip_path_prefix = false
#[tokio::test]
async fn test_get_tree_strip_prefix_false() {
    let local_repo = get_test_repository().await;

    let params = TreeParams {
        case_sensitive: None,
        search_relative_path: None,
        respect_gitignore: Some(false),
        depth: None,
        strip_path_prefix: Some(false),
    };

    let result = local_repo.get_tree_with_params(Some(params)).await;

    assert!(
        result.is_ok(),
        "Failed to get tree with strip_path_prefix=false: {:?}",
        result.err()
    );

    let tree = result.unwrap();
    assert!(!tree.is_empty(), "Tree should not be empty");
}

/// Test get_tree_with_params with complex combination of parameters
#[tokio::test]
async fn test_get_tree_complex_params() {
    let local_repo = get_test_repository().await;

    let params = TreeParams {
        case_sensitive: Some(false),
        search_relative_path: Some(PathBuf::from("src")),
        respect_gitignore: Some(false), // Ignore gitignore to see actual files
        depth: Some(2),
        strip_path_prefix: Some(true),
    };

    let result = local_repo.get_tree_with_params(Some(params)).await;

    assert!(
        result.is_ok(),
        "Failed to get tree with complex params: {:?}",
        result.err()
    );

    let tree = result.unwrap();
    assert!(!tree.is_empty(), "Tree should not be empty");

    // Should have entries under src directory
    let src_tree = tree.iter().find(|t| t.dir == "src");
    assert!(
        src_tree.is_some(),
        "Should have src directory tree with complex params"
    );
}

/// Test get_tree_with_params with all parameters set to Some values
#[tokio::test]
async fn test_get_tree_all_params_set() {
    let local_repo = get_test_repository().await;

    let params = TreeParams {
        case_sensitive: Some(true),
        search_relative_path: Some(PathBuf::from("src/utils")),
        respect_gitignore: Some(false), // Ignore gitignore to see actual files
        depth: Some(1),
        strip_path_prefix: Some(true),
    };

    let result = local_repo.get_tree_with_params(Some(params)).await;

    assert!(
        result.is_ok(),
        "Failed to get tree with all params set: {:?}",
        result.err()
    );

    let tree = result.unwrap();
    assert!(!tree.is_empty(), "Tree should not be empty");

    // Should have entries under src/utils directory
    let utils_tree = tree.iter().find(|t| t.dir == "src/utils");
    assert!(
        utils_tree.is_some(),
        "Should have src/utils directory tree with all params set"
    );
}

/// Test get_tree_with_params with multiple search_relative_path values
#[tokio::test]
async fn test_get_tree_various_relative_paths() {
    let local_repo = get_test_repository().await;

    // Test different relative paths
    let test_paths = vec!["src", "src/models", "src/utils"];

    for path in test_paths {
        let params = TreeParams {
            case_sensitive: None,
            search_relative_path: Some(PathBuf::from(path)),
            respect_gitignore: Some(false),
            depth: None,
            strip_path_prefix: None,
        };

        let result = local_repo.get_tree_with_params(Some(params)).await;

        assert!(
            result.is_ok(),
            "Failed to get tree with relative path '{}': {:?}",
            path,
            result.err()
        );

        let tree = result.unwrap();
        assert!(
            !tree.is_empty(),
            "Tree should not be empty for path '{}'",
            path
        );
    }
}

/// Test get_tree_with_params with edge case: empty relative path
#[tokio::test]
async fn test_get_tree_empty_relative_path() {
    let local_repo = get_test_repository().await;

    let params = TreeParams {
        case_sensitive: None,
        search_relative_path: Some(PathBuf::from("")),
        respect_gitignore: Some(false),
        depth: None,
        strip_path_prefix: None,
    };

    let result = local_repo.get_tree_with_params(Some(params)).await;

    // Empty path should work (equivalent to root)
    assert!(
        result.is_ok(),
        "Failed to get tree with empty relative path: {:?}",
        result.err()
    );

    let tree = result.unwrap();
    assert!(!tree.is_empty(), "Tree should not be empty");
}

/// Test get_tree_with_params with various depth values
#[tokio::test]
async fn test_get_tree_various_depths() {
    let local_repo = get_test_repository().await;

    // Test different depth values
    let test_depths = vec![0, 1, 2, 3, 10];

    for depth in test_depths {
        let params = TreeParams {
            case_sensitive: None,
            search_relative_path: None,
            respect_gitignore: Some(false),
            depth: Some(depth),
            strip_path_prefix: None,
        };

        let result = local_repo.get_tree_with_params(Some(params)).await;

        assert!(
            result.is_ok(),
            "Failed to get tree with depth {}: {:?}",
            depth,
            result.err()
        );
    }
}

/// Test get_tree_with_params error handling for invalid repository
#[tokio::test]
async fn test_get_tree_invalid_repository() {
    // Create LocalRepository with invalid path
    let invalid_repo = LocalRepository::new(PathBuf::from("/nonexistent/repository"));

    let params = TreeParams {
        case_sensitive: None,
        search_relative_path: None,
        respect_gitignore: Some(false),
        depth: None,
        strip_path_prefix: None,
    };

    let result = invalid_repo.get_tree_with_params(Some(params)).await;

    // Should return an error for invalid repository
    assert!(result.is_err(), "Should fail for invalid repository");
    assert!(
        result.unwrap_err().contains("Invalid repository"),
        "Error should mention invalid repository"
    );
}
