//! Tests for GitHub API functions in the repository_manager module
//!
//! These tests focus on GitHub API integration, particularly the list_repository_refs function.
//! The tests use the test repository at https://github.com/tacogips/gitcodes-mcp-test-1

use std::env;

use gitcodes_mcp::gitcodes::repository_manager::RepositoryManager;

/// Creates a Repository Manager for testing
fn create_test_manager() -> RepositoryManager {
    // Check for GitHub token in environment
    let github_token = env::var("GITCODES_MCP_GITHUB_TOKEN").ok();

    // Create a temporary directory for repository cache
    let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
    let cache_dir = temp_dir.path().to_path_buf();

    // Create a repository manager with our temporary directory
    RepositoryManager::new(
        github_token, // Use token if available
        Some(cache_dir),
    )
    .expect("Failed to create RepositoryManager")
}

/// Tests that list_repository_refs can successfully retrieve branch and tag information from GitHub
///
/// This test verifies that the service function can:
/// 1. Parse a repository location
/// 2. Connect to the GitHub API
/// 3. Retrieve and parse repository references (branches and tags)
/// 4. Return the results in a valid JSON format
#[tokio::test]
async fn test_list_repository_refs_github() {
    // Use public test repository with various URL formats
    let repo_urls = vec![
        "github:tacogips/gitcodes-mcp-test-1",
        "https://github.com/tacogips/gitcodes-mcp-test-1.git",
    ];

    // Create test manager
    let manager = create_test_manager();

    for repo_url in repo_urls {
        println!("Testing repository refs for URL: {}", repo_url);

        // Call the repository manager method to list repository refs
        let result = manager.list_repository_refs(repo_url).await;

        // Verify the result
        match result {
            Ok((refs, local_repo)) => {
                // Verify the refs structure is valid
                // Check that we have some branches or tags
                assert!(
                    !refs.branches.is_empty() || !refs.tags.is_empty(),
                    "Repository should have at least one reference"
                );

                // Verify each branch reference has required properties
                for branch in &refs.branches {
                    assert!(!branch.name.is_empty(), "Branch should have a name");
                    assert!(!branch.full_ref.is_empty(), "Branch should have a full ref");
                    assert!(
                        !branch.commit_id.is_empty(),
                        "Branch should have a commit ID"
                    );
                }

                // Verify each tag reference has required properties (if any tags exist)
                for tag in &refs.tags {
                    assert!(!tag.name.is_empty(), "Tag should have a name");
                    assert!(!tag.full_ref.is_empty(), "Tag should have a full ref");
                    assert!(!tag.commit_id.is_empty(), "Tag should have a commit ID");
                }

                // Clean up the local repository if one was created
                if let Some(repo) = local_repo {
                    let repo_dir = repo.get_repository_dir();
                    println!("Cleaning up repository at: {}", repo_dir.display());

                    if repo_dir.exists() {
                        // Verify the directory exists before cleanup
                        assert!(
                            repo_dir.exists(),
                            "Repository directory should exist before cleanup"
                        );

                        // Clean up the repository
                        repo.cleanup().expect("Failed to clean up repository");

                        // Verify the directory no longer exists after cleanup
                        assert!(
                            !repo_dir.exists(),
                            "Repository directory should not exist after cleanup"
                        );
                    }
                }

                println!(
                    "Successfully retrieved and verified repository refs for {}",
                    repo_url
                );
            }
            Err(e) => {
                panic!("Failed to list repository refs for {}: {}", repo_url, e);
            }
        }
    }
}

/// Tests the GithubClient directly by calling list_repository_refs
///
/// This test verifies that the GitHub client can successfully connect
/// to the GitHub API and retrieve repository references.
#[tokio::test]
async fn test_github_client_list_refs() {
    // Create test manager
    let manager = create_test_manager();

    // Get test repository info
    let repo_url = "github:tacogips/gitcodes-mcp-test-1";

    // Instead of using the private method get_github_client, use the public list_repository_refs method
    let result = manager.list_repository_refs(repo_url).await;

    // Verify the result
    match result {
        Ok((refs, _)) => {
            // Verify that we have some branches or tags
            assert!(
                !refs.branches.is_empty() || !refs.tags.is_empty(),
                "Repository should have at least one reference"
            );

            // For more detailed verification, we could check branch and tag details as in the previous test

            println!(
                "GitHub client successfully retrieved repository refs: {} branches, {} tags",
                refs.branches.len(),
                refs.tags.len()
            );
        }
        Err(e) => {
            panic!("GitHub client failed to list repository refs: {}", e);
        }
    }
}

/// Tests error handling in list_repository_refs
///
/// This test verifies that the service function properly handles errors
/// when given invalid repository information.
#[tokio::test]
async fn test_list_repository_refs_error_handling() {
    // Create test manager
    let manager = create_test_manager();

    // Invalid repository URL (non-existent repository)
    let invalid_repo_url = "github:tacogips/non-existent-repository-12345";

    // Call the repository manager method with the invalid URL
    let result = manager.list_repository_refs(invalid_repo_url).await;

    // Verify the function returns an error
    assert!(
        result.is_err(),
        "Function should return an error for non-existent repository"
    );

    // Check the error message
    if let Err(error_message) = result {
        println!("Error message: {}", error_message);
        assert!(
            error_message.contains("404") || error_message.contains("not found"),
            "Error message should indicate repository not found"
        );
    }

    // Test with invalid format
    let malformed_url = "invalid-format";
    let result = manager.list_repository_refs(malformed_url).await;

    // Verify the function returns an error
    assert!(
        result.is_err(),
        "Function should return an error for invalid URL format"
    );

    // Check the error message
    if let Err(error_message) = result {
        println!("Error message: {}", error_message);
        assert!(
            error_message.contains("parse") || error_message.contains("format"),
            "Error message should indicate parsing error"
        );
    }
}
