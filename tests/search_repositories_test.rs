//! Tests for search_repositories method in the repository_manager module
//!
//! These tests focus on the repository search functionality and verify:
//! 1. Parameter conversion from generic strings to provider-specific options
//! 2. Proper API integration with the GitHub search endpoint
//! 3. Error handling for invalid input

use std::env;
use std::str::FromStr;

use gitcodes_mcp::gitcodes::repository_manager::providers::GitProvider;
use gitcodes_mcp::gitcodes::repository_manager::{OrderOption, RepositoryManager, SortOption};

/// Creates a Repository Manager for testing
fn create_test_manager() -> RepositoryManager {
    // Check for GitHub token in environment
    let github_token = env::var("GITCODE_MCP_GITHUB_TOKEN").ok();

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

/// Tests parameter conversion from strings to provider-specific options
///
/// This test verifies that:
/// 1. The generic string parameters are correctly converted to provider-specific enum values
/// 2. The search works with various parameter combinations
#[tokio::test]
async fn test_search_repositories_parameter_conversion() {
    let manager = create_test_manager();

    // Test cases for different parameter combinations
    // Each tuple contains:
    // (provider, sort_option, order_option)
    let test_cases = vec![
        // Default case - no sort, no order
        ("github", None::<SortOption>, None::<OrderOption>),
        // Sort by stars
        ("github", Some(SortOption::Stars), None),
        // Sort by forks
        ("github", Some(SortOption::Forks), None),
        // Sort by updated
        ("github", Some(SortOption::Updated), None),
        // Test relevance
        ("github", Some(SortOption::Relevance), None),
        // Test order alone
        ("github", None, Some(OrderOption::Ascending)),
        ("github", None, Some(OrderOption::Descending)),
        // Full combination
        (
            "github",
            Some(SortOption::Forks),
            Some(OrderOption::Descending),
        ),
    ];

    // Use a query that will reliably return results
    let query = "github language:rust stars:>100";

    for (provider_str, sort_option, order_option) in test_cases {
        println!(
            "Testing with provider: {}, sort: {:?}, order: {:?}",
            provider_str, sort_option, order_option
        );

        // Convert provider string to GitProvider enum
        let provider = GitProvider::from_str(provider_str).expect("Failed to parse provider");

        // Execute the search
        let result = manager
            .search_repositories(
                provider,
                query.to_string(),
                sort_option.clone(),  // Clone to avoid move
                order_option.clone(), // Clone to avoid move
                Some(5),              // Limit results to 5 per page
                Some(1),              // First page
            )
            .await;

        // Verify the result
        match result {
            Ok(search_results) => {
                // Verify the result structure directly from the structured data
                assert!(
                    !search_results.items.is_empty(),
                    "Search results should have items"
                );

                // We can access the structured data directly

                // GitHub API doesn't return the search parameters in the response,
                // so we can't directly verify the converted parameters were used.
                // But we can verify the request succeeded with different parameter combinations.

                println!(
                    "Search succeeded with parameters: {:?}, {:?}",
                    sort_option, order_option
                );
            }
            Err(e) => {
                // Skip rate limit errors, which are expected when running tests frequently
                if e.contains("rate limit") {
                    println!("Skipping due to rate limit: {}", e);
                    continue;
                }

                // Other errors should fail the test
                panic!("Search failed: {}", e);
            }
        }
    }
}

/// Tests basic repository searching without parameters
///
/// This test verifies that the search still succeeds without any sorting parameters
/// since the function now enforces valid enum values at compile time
#[tokio::test]
async fn test_search_repositories_basic() {
    let manager = create_test_manager();

    // Use a query that will reliably return results
    let query = "github language:rust stars:>100";

    // Execute the search with minimal parameters
    println!("Testing with minimal parameters");

    let result = manager
        .search_repositories(
            GitProvider::Github,
            query.to_string(),
            None,    // No sort option
            None,    // No order option
            Some(5), // Limit results to 5 per page
            Some(1), // First page
        )
        .await;

    // Check if the search was successful
    match result {
        Ok(json_result) => {
            // Verify the result structure directly
            assert!(
                !json_result.items.is_empty(),
                "Search results should have items"
            );

            println!("Search succeeded with minimal parameters");
        }
        Err(e) => {
            // Skip rate limit errors, which are expected when running tests frequently
            if e.contains("rate limit") {
                println!("Skipping due to rate limit: {}", e);
                return;
            }

            // Other errors should fail the test
            panic!("Basic search failed: {}", e);
        }
    }
}

/* Mock test example (requires mockall to be set up)

/// Tests the search_repositories method with a mock GitHub client
///
/// This test uses a mock to:
/// 1. Verify that the correct URL is constructed based on the provided parameters
/// 2. Verify correct parameter conversion without making actual API calls
#[tokio::test]
async fn test_search_repositories_with_mock() {
    use gitcodes_mcp::gitcodes::repository_manager::providers::github::GithubClient;
    use mockall::predicate::*;

    // Create a mock GitHub client
    let mut mock_client = GithubClient::new_mock();

    // Set up expectations for various parameter combinations
    mock_client
        .expect_execute_search_repository_request()
        .withf(|params| {
            // Verify parameters for default case
            params.query == "test repo" &&
            params.sort_by.is_none() &&
            params.order.is_none() &&
            params.per_page == Some(30) &&
            params.page == Some(1)
        })
        .returning(|_| Ok("{\"items\": []}".to_string()))
        .times(1);

    mock_client
        .expect_execute_search_repository_request()
        .withf(|params| {
            // Verify parameters for sort by stars
            params.query == "test repo" &&
            params.sort_by.as_ref().map(|s| s.to_str()) == Some("stars") &&
            params.order.is_none() &&
            params.per_page == Some(30) &&
            params.page == Some(1)
        })
        .returning(|_| Ok("{\"items\": []}".to_string()))
        .times(1);

    mock_client
        .expect_execute_search_repository_request()
        .withf(|params| {
            // Verify parameters for sort by forks with descending order
            params.query == "test repo" &&
            params.sort_by.as_ref().map(|s| s.to_str()) == Some("forks") &&
            params.order.as_ref().map(|o| o.to_str()) == Some("desc") &&
            params.per_page == Some(30) &&
            params.page == Some(1)
        })
        .returning(|_| Ok("{\"items\": []}".to_string()))
        .times(1);

    // Test various parameter combinations
    let manager = create_test_manager();

    let _ = manager
        .search_repositories(
            GitProvider::Github,
            "test repo".to_string(),
            None,
            None,
            Some(30),
            Some(1),
        )
        .await;

    let _ = manager
        .search_repositories(
            GitProvider::Github,
            "test repo".to_string(),
            Some("stars".to_string()),
            None,
            Some(30),
            Some(1),
        )
        .await;

    let _ = manager
        .search_repositories(
            GitProvider::Github,
            "test repo".to_string(),
            Some("forks".to_string()),
            Some("descending".to_string()),
            Some(30),
            Some(1),
        )
        .await;

    // If we get here without mockall panic, the test passes
}

*/
