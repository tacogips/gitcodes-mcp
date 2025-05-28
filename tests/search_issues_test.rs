//! Tests for search_issues method in the repository_manager module
//!
//! These tests focus on the issue search functionality and verify:
//! 1. Parameter conversion from generic strings to provider-specific options
//! 2. Proper API integration with the GitHub issues search endpoint
//! 3. Error handling for invalid input

use std::env;
use std::str::FromStr;

use gitcodes_mcp::gitcodes::repository_manager::providers::GitProvider;
use gitcodes_mcp::gitcodes::repository_manager::{IssueSortOption, OrderOption, RepositoryManager};

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

/// Tests parameter conversion from strings to provider-specific options
///
/// This test verifies that:
/// 1. The generic string parameters are correctly converted to provider-specific enum values
/// 2. The search works with various parameter combinations
#[tokio::test]
async fn test_search_issues_parameter_conversion() {
    let manager = create_test_manager();

    // Test cases for different parameter combinations
    // Each tuple contains:
    // (provider, sort_option, order_option)
    let test_cases = vec![
        // Default case - no sort, no order
        ("github", None::<IssueSortOption>, None::<OrderOption>),
        // Sort by created
        ("github", Some(IssueSortOption::Created), None),
        // Sort by updated
        ("github", Some(IssueSortOption::Updated), None),
        // Sort by comments
        ("github", Some(IssueSortOption::Comments), None),
        // Test best match
        ("github", Some(IssueSortOption::BestMatch), None),
        // Test order alone
        ("github", None, Some(OrderOption::Ascending)),
        ("github", None, Some(OrderOption::Descending)),
        // Full combination
        (
            "github",
            Some(IssueSortOption::Updated),
            Some(OrderOption::Descending),
        ),
    ];

    // Use a query that will reliably return results from a popular repository
    let query = "repo:rust-lang/rust state:open label:bug";

    for (provider_str, sort_option, order_option) in test_cases {
        println!(
            "Testing with provider: {}, sort: {:?}, order: {:?}",
            provider_str, sort_option, order_option
        );

        // Convert provider string to GitProvider enum
        let provider = GitProvider::from_str(provider_str).expect("Failed to parse provider");

        // Execute the search
        let result = manager
            .search_issues(
                provider,
                gitcodes_mcp::gitcodes::repository_manager::IssueSearchParams {
                    query: query.to_string(),
                    sort_by: sort_option.clone(),
                    order: order_option.clone(),
                    per_page: Some(5),
                    page: Some(1),
                    repository: None,
                    labels: None,
                    state: None,
                    creator: None,
                    mentioned: None,
                    assignee: None,
                    milestone: None,
                    issue_type: None,
                }
            )
            .await;

        // Verify the result
        match result {
            Ok(search_results) => {
                // Verify the result structure directly from the structured data
                println!(
                    "Found {} issues with total count: {}",
                    search_results.items.len(),
                    search_results.total_count
                );

                // We can access the structured data directly
                if !search_results.items.is_empty() {
                    let first_issue = &search_results.items[0];
                    println!(
                        "First issue: #{} - {}",
                        first_issue.number, first_issue.title
                    );
                    assert!(
                        !first_issue.title.is_empty(),
                        "Issue title should not be empty"
                    );
                    assert!(first_issue.number > 0, "Issue number should be positive");
                    assert!(
                        !first_issue.state.is_empty(),
                        "Issue state should not be empty"
                    );
                }

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

/// Tests basic issue searching without parameters
///
/// This test verifies that the search still succeeds without any sorting parameters
/// since the function now enforces valid enum values at compile time
#[tokio::test]
async fn test_search_issues_basic() {
    let manager = create_test_manager();

    // Use a query that will reliably return results
    let query = "repo:rust-lang/rust state:open";

    // Execute the search with minimal parameters
    println!("Testing with minimal parameters");

    let result = manager
        .search_issues(
            GitProvider::Github,
            gitcodes_mcp::gitcodes::repository_manager::IssueSearchParams {
                query: query.to_string(),
                sort_by: None,
                order: None,
                per_page: Some(3),
                page: Some(1),
                repository: None,
                labels: None,
                state: None,
                creator: None,
                mentioned: None,
                assignee: None,
                milestone: None,
                issue_type: None,
            }
        )
        .await;

    // Check if the search was successful
    match result {
        Ok(search_results) => {
            // Verify the result structure directly
            println!(
                "Found {} issues with total count: {}",
                search_results.items.len(),
                search_results.total_count
            );

            // Validate structure of returned issues
            for (i, issue) in search_results.items.iter().enumerate() {
                println!("Issue {}: #{} - {}", i + 1, issue.number, issue.title);

                // Basic validation
                assert!(issue.number > 0, "Issue number should be positive");
                assert!(!issue.title.is_empty(), "Issue title should not be empty");
                assert!(!issue.state.is_empty(), "Issue state should not be empty");
                assert!(
                    !issue.user.login.is_empty(),
                    "Issue author should not be empty"
                );
                assert!(
                    !issue.html_url.is_empty(),
                    "Issue HTML URL should not be empty"
                );
                assert!(
                    !issue.repository.name.is_empty(),
                    "Repository name should not be empty"
                );

                // Validate that we're getting the right repository
                assert!(
                    issue.repository.name.contains("rust"),
                    "Should be from rust repository"
                );
            }

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

/// Tests issue search with various query syntaxes
///
/// This test verifies that different GitHub issue search query syntaxes work correctly
#[tokio::test]
async fn test_search_issues_query_syntax() {
    let manager = create_test_manager();

    // Test different query syntaxes
    let test_queries = vec![
        // Basic state filtering
        ("state:open", "Open issues"),
        ("state:closed", "Closed issues"),
        // Label filtering
        ("label:bug", "Issues with bug label"),
        ("label:enhancement", "Issues with enhancement label"),
        // Repository-specific searches
        (
            "repo:rust-lang/rust state:open",
            "Open issues in rust-lang/rust",
        ),
        // Author/assignee filtering (may not return results but should not error)
        ("author:octocat", "Issues by octocat"),
        // Combined filters
        (
            "repo:rust-lang/rust state:open label:bug",
            "Open bugs in rust-lang/rust",
        ),
    ];

    for (query, description) in test_queries {
        println!("Testing query: {} ({})", query, description);

        let result = manager
            .search_issues(
                GitProvider::Github,
                gitcodes_mcp::gitcodes::repository_manager::IssueSearchParams {
                    query: query.to_string(),
                    sort_by: Some(IssueSortOption::Updated),
                    order: Some(OrderOption::Descending),
                    per_page: Some(5),
                    page: Some(1),
                    repository: None,
                    labels: None,
                    state: None,
                    creator: None,
                    mentioned: None,
                    assignee: None,
                    milestone: None,
                    issue_type: None,
                }
            )
            .await;

        match result {
            Ok(search_results) => {
                println!(
                    "Query '{}' returned {} issues (total: {})",
                    query,
                    search_results.items.len(),
                    search_results.total_count
                );

                // Validate that all returned issues match the expected criteria
                if query.contains("state:open") {
                    for issue in &search_results.items {
                        assert_eq!(issue.state, "open", "All issues should be open");
                    }
                }

                if query.contains("state:closed") {
                    for issue in &search_results.items {
                        assert_eq!(issue.state, "closed", "All issues should be closed");
                    }
                }

                if query.contains("repo:rust-lang/rust") {
                    for issue in &search_results.items {
                        assert!(
                            issue.repository.name.contains("rust"),
                            "All issues should be from rust repository"
                        );
                    }
                }
            }
            Err(e) => {
                // Skip rate limit errors
                if e.contains("rate limit") {
                    println!("Skipping query '{}' due to rate limit: {}", query, e);
                    continue;
                }

                // Some queries might legitimately return no results, which is okay
                if e.contains("no results") || e.contains("not found") {
                    println!(
                        "Query '{}' returned no results (expected for some queries)",
                        query
                    );
                    continue;
                }

                // Other errors should fail the test
                panic!("Query '{}' failed: {}", query, e);
            }
        }
    }
}

/// Tests pagination functionality for issue search
///
/// This test verifies that pagination parameters work correctly
#[tokio::test]
async fn test_search_issues_pagination() {
    let manager = create_test_manager();

    // Use a query that will return many results
    let query = "repo:rust-lang/rust state:open";

    // Test different page sizes and pages
    let test_cases = vec![
        (Some(5), Some(1)),  // 5 per page, page 1
        (Some(10), Some(1)), // 10 per page, page 1
        (Some(5), Some(2)),  // 5 per page, page 2
    ];

    for (per_page, page) in test_cases {
        println!(
            "Testing pagination: per_page={:?}, page={:?}",
            per_page, page
        );

        let result = manager
            .search_issues(
                GitProvider::Github,
                gitcodes_mcp::gitcodes::repository_manager::IssueSearchParams {
                    query: query.to_string(),
                    sort_by: Some(IssueSortOption::Updated),
                    order: Some(OrderOption::Descending),
                    per_page,
                    page,
                    repository: None,
                    labels: None,
                    state: None,
                    creator: None,
                    mentioned: None,
                    assignee: None,
                    milestone: None,
                    issue_type: None,
                }
            )
            .await;

        match result {
            Ok(search_results) => {
                println!(
                    "Page {} with {} per page returned {} issues",
                    page.unwrap_or(1),
                    per_page.unwrap_or(30),
                    search_results.items.len()
                );

                // Verify that we don't get more results than requested
                if let Some(expected_per_page) = per_page {
                    assert!(
                        search_results.items.len() <= expected_per_page as usize,
                        "Should not return more items than per_page limit"
                    );
                }

                // Verify all issues have required fields
                for issue in &search_results.items {
                    assert!(issue.number > 0, "Issue number should be positive");
                    assert!(!issue.title.is_empty(), "Issue title should not be empty");
                }
            }
            Err(e) => {
                // Skip rate limit errors
                if e.contains("rate limit") {
                    println!("Skipping pagination test due to rate limit: {}", e);
                    continue;
                }

                panic!("Pagination test failed: {}", e);
            }
        }
    }
}
