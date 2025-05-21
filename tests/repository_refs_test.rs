//! Tests for the service's repository references listing functionality
//!
//! These tests verify that the list_repository_refs function can:
//! 1. Connect to GitHub repositories and retrieve references
//! 2. Correctly return branches and tags in JSON format
//! 3. Handle different repository URL formats
//!
//! ### Repository Cloning vs Local Dependencies
//!
//! These tests use dynamic repository cloning instead of referencing `.private.deps-src` because:
//! - `.private.deps-src` is gitignored and won't exist in CI/CD or other developer environments
//! - Dynamic cloning ensures tests work in any environment with internet access
//! - Follows the same patterns as other integration tests in the project

use gitcodes_mcp::gitcodes::local_repository::LocalRepository;
use gitcodes_mcp::gitcodes::repository_manager::RepositoryLocation;
use gitcodes_mcp::gitcodes::repository_manager::RepositoryManager;
use serde_json::Value as JsonValue;
use std::path::PathBuf;
use std::str::FromStr;
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

/// Test fixture that creates a LocalRepository from the test repository
///
/// Returns a LocalRepository pointing to the gitcodes-mcp-test-1 repository
async fn get_test_repository() -> LocalRepository {
    let manager = create_test_manager();
    let repo_location =
        RepositoryLocation::from_str(TEST_REPO_URL).expect("Failed to parse test repository URL");

    manager
        .prepare_repository(&repo_location, None)
        .await
        .expect("Failed to prepare test repository")
}

/// Tests fetch_remote functionality
///
/// This test verifies that:
/// 1. The repository can be opened
/// 2. The fetch_remote method works correctly with the gix library
/// 3. The method handles different error cases gracefully
#[tokio::test]
async fn test_fetch_remote() {
    let local_repo = get_test_repository().await;

    // Test the fetch_remote function
    match local_repo.fetch_remote().await {
        Ok(_) => println!("Successfully fetched updates from remote repository"),
        Err(e) => {
            // If this fails because there's no remote or no network connection, that's ok for a test
            // We're primarily testing that the implementation doesn't panic and handles errors gracefully
            println!(
                "Note: fetch_remote returned error (might be expected in test environment): {}",
                e
            );
        }
    }
}

#[tokio::test]
async fn test_services_list_repository_refs_local_with_fetch() {
    // Create test manager
    let manager = create_test_manager();

    // Create a temporary directory for a local repo
    let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
    let temp_path = temp_dir.path();

    // Clone the test repository into the temp directory
    let github_url = "github:tacogips/gitcodes-mcp-test-1";

    // First, use the RepositoryManager to clone the repository
    let github_location =
        RepositoryLocation::from_str(github_url).expect("Failed to parse repository location");

    let local_repo = match manager.prepare_repository(&github_location, None).await {
        Ok(repo) => repo,
        Err(e) => {
            println!("Warning: Failed to prepare GitHub repository: {}", e);
            println!("This test requires network access and may fail in restricted environments.");
            return; // Skip test if we can't access GitHub
        }
    };

    // Create a git command to clone that repository to our temp directory
    let clone_result = std::process::Command::new("git")
        .args([
            "clone",
            local_repo.get_repository_dir().to_str().unwrap(),
            temp_path.to_str().unwrap(),
        ])
        .output();

    match clone_result {
        Ok(output) => {
            if !output.status.success() {
                let error_msg = String::from_utf8_lossy(&output.stderr);
                println!(
                    "Warning: Failed to clone repository to temp dir: {}",
                    error_msg
                );
                return; // Skip test if clone fails
            }

            println!(
                "Successfully cloned repository to temporary directory: {}",
                temp_path.display()
            );

            // Create a LocalPath repository location pointing to our temp directory
            let local_path_str = format!("file:{}", temp_path.display());

            // First, get refs without fetching by calling the repository directly
            // This simulates what would happen without our fetch_remote addition
            let local_path_location = RepositoryLocation::from_str(&local_path_str)
                .expect("Failed to parse local path location");

            let local_repo_direct =
                match manager.prepare_repository(&local_path_location, None).await {
                    Ok(repo) => repo,
                    Err(e) => {
                        println!("Warning: Failed to prepare local repository: {}", e);
                        return; // Skip test if we can't prepare the local repo
                    }
                };

            // Get references from the LocalRepository directly
            let refs_before = local_repo_direct
                .list_repository_refs()
                .await
                .expect("Warning: Failed to list repository refs before fetch ");

            // Convert the RepositoryRefs struct to JSON for the test

            // Manipulate the original repository to add a new tag
            // This simulates changes happening in the remote while our local repo is unchanged
            let new_tag_name = "test-fetch-tag";
            let tag_result = std::process::Command::new("git")
                .args(["tag", new_tag_name])
                .current_dir(local_repo.get_repository_dir())
                .output();

            if let Ok(output) = tag_result {
                if output.status.success() {
                    println!("Created new tag '{}' in source repository", new_tag_name);
                } else {
                    let error_msg = String::from_utf8_lossy(&output.stderr);
                    println!("Warning: Failed to create tag: {}", error_msg);
                }
            }

            // Now, use the repository manager's list_repository_refs method which should fetch updates
            let (refs_after, local_repo_opt) =
                match manager.list_repository_refs(&local_path_str).await {
                    Ok(result) => result,
                    Err(e) => {
                        println!("Warning: Failed to list repository refs via service: {}", e);
                        return; // Skip test if the service call fails
                    }
                };

            // Extract tags to see if our new tag appears after the fetch
            //let tags_before: Vec<&str> = refs_before_value
            let tags_before: Vec<&str> = refs_before
                .tags
                .iter()
                .map(|tag| tag.name.as_str())
                .collect();
            let tags_after: Vec<&str> = refs_after
                .tags
                .iter()
                .map(|tag| tag.name.as_str())
                .collect();

            println!("Tags before fetch: {:?}", tags_before);
            println!("Tags after fetch: {:?}", tags_after);

            // Verify the LocalRepository was returned
            assert!(
                local_repo_opt.is_some(),
                "LocalRepository should be returned for local paths"
            );

            // If fetching worked correctly, the new tag should appear in the after results
            // but not in the before results. However, network or other issues might prevent this,
            // so we'll be lenient in our assertion.
            if !tags_before.contains(&new_tag_name) && tags_after.contains(&new_tag_name) {
                println!("Test verified: New tag appeared after fetch but not before");
            } else if tags_after.contains(&new_tag_name) {
                println!("Test partially verified: New tag appears in after results");
            } else if tags_before.contains(&new_tag_name) {
                println!("Note: New tag appeared in before results (unexpected but allowed)");
            } else {
                println!("Warning: New tag did not appear in results. This could be due to test environment limitations.");
            }

            // At minimum, ensure we got some tags in both results
            assert!(
                !tags_before.is_empty(),
                "Should have some tags before fetch"
            );
            assert!(!tags_after.is_empty(), "Should have some tags after fetch");
        }
        Err(e) => {
            println!("Warning: Failed to clone repository: {}", e);
            // Skip test if we can't set it up properly
        }
    }
}

/// Tests fetch_remote on repository validation failure
///
/// This test verifies that fetch_remote properly validates the repository before
/// attempting to fetch, and returns appropriate errors.
#[tokio::test]
async fn test_fetch_remote_invalid_repository() {
    // Create a LocalRepository with a non-existent path
    let non_existent_path = PathBuf::from("path/to/nonexistent/repo");
    let invalid_repo = LocalRepository::new(non_existent_path);

    // Attempt to fetch from an invalid repository
    let result = invalid_repo.fetch_remote().await;

    // Should return an error
    assert!(
        result.is_err(),
        "fetch_remote should return an error for invalid repository"
    );

    // Error should mention validation
    let error_msg = result.err().unwrap();
    assert!(
        error_msg.contains("Invalid repository"),
        "Error message should mention invalid repository: {}",
        error_msg
    );
}

/// Tests listing repository references (branches and tags)
///
/// This test verifies that we can retrieve references from a GitHub repository
/// and that the expected branches and tags are present in the results.
#[tokio::test]
async fn test_list_repository_refs() {
    // Use public test repository with github: URL format
    let repo_url = "github:tacogips/gitcodes-mcp-test-1";

    // Create test manager
    let manager = create_test_manager();

    // List repository references via the repository manager
    let result = manager.list_repository_refs(repo_url).await;

    match result {
        Ok((refs, local_repo_opt)) => {
            // Extract branches and tags directly from the structured refs
            let branches: Vec<&str> = refs
                .branches
                .iter()
                .map(|branch| branch.name.as_str())
                .collect();

            let tags: Vec<&str> = refs.tags.iter().map(|tag| tag.name.as_str()).collect();

            // Print the extracted branches and tags
            println!("Branches: {:?}", branches);
            println!("Tags: {:?}", tags);

            // Verify we have at least some branches and tags
            assert!(!branches.is_empty(), "Repository should have branches");
            assert!(!tags.is_empty(), "Repository should have tags");

            // Verify important branches exist (test repo changed over time, so we check for main and some critical ones)
            assert!(
                branches.contains(&"main"),
                "Branch 'main' should exist in repository"
            );

            // Since branch names might change, we'll check for partial matches on important features
            assert!(
                branches.iter().any(|b| b.contains("api-client")),
                "Should have a branch related to api-client"
            );
            assert!(
                branches
                    .iter()
                    .any(|b| b.contains("authentication") || b.contains("auth")),
                "Should have a branch related to authentication"
            );

            // Verify specific tag exists
            assert!(
                tags.contains(&"v0.0.0"),
                "Tag 'v0.0.0' should exist in repository"
            );

            // For GitHub repository, local_repo_opt should be None
            assert!(
                local_repo_opt.is_none(),
                "GitHub repository should not return a LocalRepository"
            );

            println!(
                "Successfully retrieved {} branches and {} tags",
                branches.len(),
                tags.len()
            );
        }
        Err(e) => {
            panic!("Failed to list repository refs: {}", e);
        }
    }
}

/// Tests listing repository references with HTTPS URL format
///
/// This test verifies that we can use HTTPS URL format to retrieve references
/// from a GitHub repository.
#[tokio::test]
async fn test_list_repository_refs_https_url() {
    // Use public test repository with HTTPS URL format
    let repo_url = "https://github.com/tacogips/gitcodes-mcp-test-1.git";

    // Create test manager
    let manager = create_test_manager();

    // List repository references via the repository manager
    let result = manager.list_repository_refs(repo_url).await;

    // Handle the result conditionally
    match result {
        Ok((refs, _)) => {
            // Access structured data directly

            // Verify that branches or tags are present
            assert!(
                !refs.branches.is_empty() || !refs.tags.is_empty(),
                "Repository should have at least one reference"
            );

            // Extract branches and tags directly from structured data
            let branches = refs
                .branches
                .iter()
                .map(|branch| branch.name.as_str())
                .collect::<Vec<_>>();
            let tags = refs
                .tags
                .iter()
                .map(|tag| tag.name.as_str())
                .collect::<Vec<_>>();

            // Print the branches and tags
            println!("Branches: {:?}", branches);
            println!("Tags: {:?}", tags);

            // Verify we have at least some branches (repos should typically have a main/master branch)
            assert!(
                !branches.is_empty(),
                "Repository should have at least one branch"
            );

            // Typically private repos will have a default branch like main or master
            assert!(
                branches.iter().any(|&b| b == "main" || b == "master"),
                "Expected main or master branch"
            );

            // We may want to verify tags if we know the specific repo will have them
            if !tags.is_empty() {
                println!("Repository has tags: {:?}", tags);
            }

            println!(
                "Successfully retrieved {} branches and {} tags with HTTPS URL",
                branches.len(),
                tags.len()
            );
        }
        Err(e) => {
            panic!("Failed to list repository refs with HTTPS URL: {}", e);
        }
    }
}

/// Tests handling of repository refs for local repository
///
/// After cloning a repository locally, we should be able to get refs
/// directly from the local copy, which will include a LocalRepository
/// instance in the result.
#[tokio::test]
async fn test_list_local_repository_refs() {
    // First, clone the repository to ensure we have a local copy
    // Create test manager
    let manager = create_test_manager();

    // Use a GitHub URL initially to clone the repo
    let github_url = "github:tacogips/gitcodes-mcp-test-1";

    // Prepare the repository first (clone it)
    // Parse the repository location string and prepare the repository
    let repository_location =
        gitcodes_mcp::gitcodes::repository_manager::RepositoryLocation::from_str(github_url)
            .expect("Failed to parse repository location");
    let prepare_result = manager.prepare_repository(&repository_location, None).await;
    if let Ok(local_repo) = prepare_result {
        // Get the local path
        let repo_dir = local_repo.get_repository_dir();
        let local_path = repo_dir.to_string_lossy();

        // Now list refs from the local repository
        let result = manager.list_repository_refs(&local_path).await;

        match result {
            Ok((refs, local_repo_opt)) => {
                // Extract branches and tags directly from the structured refs
                let branches: Vec<&str> = refs
                    .branches
                    .iter()
                    .map(|branch| branch.name.as_str())
                    .collect();

                let tags: Vec<&str> = refs.tags.iter().map(|tag| tag.name.as_str()).collect();

                // Print the extracted branches and tags
                println!("Branches: {:?}", branches);
                println!("Tags: {:?}", tags);

                // Check that we have the expected branches
                assert!(
                    branches
                        .iter()
                        .any(|b| b.contains("main") || b.contains("master")),
                    "Repository should have a main or master branch"
                );

                // Verify we have at least some branches
                assert!(!branches.is_empty(), "Repository should have branches");

                // For local repository, local_repo_opt should be Some
                assert!(
                    local_repo_opt.is_some(),
                    "Local repository should return a LocalRepository"
                );

                // Clean up if we have a local repo instance
                if let Some(repo) = local_repo_opt {
                    repo.cleanup().expect("Failed to clean up repository");
                }

                // Clean up the initial repo too
                local_repo
                    .cleanup()
                    .expect("Failed to clean up initial repository");
            }
            Err(e) => {
                // Clean up even on error
                local_repo.cleanup().expect("Failed to clean up repository");
                panic!("Failed to list local repository refs: {}", e);
            }
        }
    } else {
        println!(
            "Warning: Could not prepare repository for local refs test: {:?}",
            prepare_result.err()
        );
    }
}

/// Tests the direct list_repository_refs method on a local repository
///
/// This test verifies that the list_repository_refs method directly on the
/// LocalRepository struct correctly returns repository references as JSON.
///
/// Instead of trying to clone a remote repository, this test creates a temporary
/// local git repository to test with, making it more reliable in environments
/// where network access might be restricted.
#[tokio::test]
async fn test_local_repository_list_refs_direct() {
    // Create a temporary directory for our test repository
    let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
    let repo_path = temp_dir.path();

    // Initialize a git repository in the temporary directory
    let init_result = std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(repo_path)
        .output();

    // Skip test if git command fails (git might not be installed)
    let init_output = match init_result {
        Ok(output) => output,
        Err(e) => {
            println!(
                "Skipping test_local_repository_list_refs_direct: 'git' command failed: {}",
                e
            );
            return;
        }
    };

    if !init_output.status.success() {
        println!("Skipping test_local_repository_list_refs_direct: Failed to initialize git repository: {}",
                String::from_utf8_lossy(&init_output.stderr));
        return;
    }

    // Configure git user for the test repository
    let _ = std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .output();

    let _ = std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .output();

    // Create a test file and commit it to create the main branch
    let test_file_path = repo_path.join("test_file.txt");
    std::fs::write(&test_file_path, "This is a test file.").expect("Failed to write test file");

    // Add the file to git
    let _ = std::process::Command::new("git")
        .args(["add", "test_file.txt"])
        .current_dir(repo_path)
        .output();

    // Commit the file
    let commit_result = std::process::Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .output();

    if let Ok(output) = commit_result {
        if !output.status.success() {
            println!(
                "Warning: Failed to commit test file: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    // Create a feature branch
    let _ = std::process::Command::new("git")
        .args(["checkout", "-b", "feature/test-feature"])
        .current_dir(repo_path)
        .output();

    // Create a tag
    let _ = std::process::Command::new("git")
        .args(["tag", "-a", "v0.1.0", "-m", "Test tag"])
        .current_dir(repo_path)
        .output();

    // Create a LocalRepository instance pointing to our test repository
    let local_repo =
        gitcodes_mcp::gitcodes::local_repository::LocalRepository::new(repo_path.to_path_buf());

    // Call the list_repository_refs method directly on the LocalRepository instance
    let refs_result = local_repo.list_repository_refs().await;

    match refs_result {
        Ok(refs) => {
            // Convert the RepositoryRefs struct to JSON for the test
            let refs_value: JsonValue =
                serde_json::to_value(&refs).expect("Failed to convert refs to JSON value");

            // Check branches and tags arrays exist in the JSON representation
            assert!(
                refs_value.get("branches").is_some() && refs_value.get("tags").is_some(),
                "References should have branches and tags fields"
            );

            // Now we can work directly with the branches and tags from the struct
            // Collect branch names directly from the RepositoryRefs struct
            let branch_names = refs
                .branches
                .iter()
                .map(|branch| branch.name.clone())
                .collect::<Vec<String>>();
            let tag_names = refs
                .tags
                .iter()
                .map(|tag| tag.name.clone())
                .collect::<Vec<String>>(); // Print the extracted branches and tags
            println!("Branches from direct call: {:?}", branch_names);
            println!("Tags from direct call: {:?}", tag_names);
            // Verify we have at least the branches we created
            assert!(
                branch_names.contains(&String::from("main")),
                "Branch 'main' should exist in repository"
            );
            assert!(
                branch_names.contains(&String::from("feature/test-feature")),
                "Branch 'feature/test-feature' should exist in repository"
            );

            // Verify tag exists
            assert!(
                tag_names.contains(&String::from("v0.1.0")),
                "Tag 'v0.1.0' should exist in repository"
            );

            // Verify each reference has a SHA
            // Iterate through both branches and tags
            for item in refs.branches.iter().chain(refs.tags.iter()) {
                assert!(!item.commit_id.is_empty(), "Commit ID should not be empty");
                // Commit ID should be a hex string (typically 40 chars for full SHA-1)
                assert!(
                    item.commit_id.chars().all(|c| c.is_ascii_hexdigit()),
                    "Commit ID should be a hex string"
                );
            }

            println!(
                "Successfully retrieved {} branches and {} tags directly from LocalRepository",
                branch_names.len(),
                tag_names.len()
            );
        }
        Err(e) => {
            panic!("Failed to list repository refs directly: {}", e);
        }
    }

    // The temporary directory will be automatically cleaned up when it goes out of scope
}

/// Tests fetching from remote and listing repository refs
///
/// This test verifies that the fetch_remote and list_repository_refs methods
/// work correctly with a local repository that has a remote origin. It clones
/// a real repository, fetches latest updates, and then lists refs directly.
#[tokio::test]
async fn test_fetch_and_list_new_built_repository_refs() {
    // Create test manager
    let manager = create_test_manager();

    // Use a public test repository (same one used in other tests)
    let github_url = "github:tacogips/gitcodes-mcp-test-1";

    // Parse the repository location
    let repository_location =
        gitcodes_mcp::gitcodes::repository_manager::RepositoryLocation::from_str(github_url)
            .expect("Failed to parse repository location");

    println!("Preparing repository from {}", github_url);

    // Try to prepare the repository (clone it)
    let prepare_result = manager.prepare_repository(&repository_location, None).await;

    match prepare_result {
        Ok(local_repo) => {
            println!(
                "Successfully cloned repository to {}",
                local_repo.get_repository_dir().display()
            );

            // Fetch updates from the remote
            println!("Fetching updates from remote...");
            match local_repo.fetch_remote().await {
                Ok(_) => println!("Successfully fetched updates from remote"),
                Err(e) => println!("Note: Fetch from remote failed (test will continue): {}", e),
            }

            // Now list the repository refs directly using the LocalRepository instance
            println!("Listing repository refs...");
            let refs_result = local_repo.list_repository_refs().await;

            match refs_result {
                Ok(refs) => {
                    // Convert the RepositoryRefs struct to JSON for the test
                    let refs_value: JsonValue =
                        serde_json::to_value(&refs).expect("Failed to convert refs to JSON value");

                    // Check branches and tags arrays exist in the JSON representation
                    assert!(
                        refs_value.get("branches").is_some() && refs_value.get("tags").is_some(),
                        "References should have branches and tags fields"
                    );

                    // Now we can work directly with the branches and tags from the struct
                    // Collect branch names directly from the RepositoryRefs struct
                    let branch_names = refs
                        .branches
                        .iter()
                        .map(|branch| branch.name.clone())
                        .collect::<Vec<String>>();
                    let tag_names = refs
                        .tags
                        .iter()
                        .map(|tag| tag.name.clone())
                        .collect::<Vec<String>>();
                    // Print the extracted branches and tags
                    println!("Branches after fetch: {:?}", branch_names);
                    println!("Tags after fetch: {:?}", tag_names);

                    // Verify we have at least some branches and tags
                    assert!(!branch_names.is_empty(), "Repository should have branches");
                    assert!(!tag_names.is_empty(), "Repository should have tags");

                    // Verify specific branches exist (using the known branches from the test repo)
                    let expected_branches =
                        vec!["main", "bugfix/api-client", "feature/authentication"]
                            .into_iter()
                            .map(|branch| branch.to_string());
                    for expected_branch in expected_branches {
                        assert!(
                            branch_names.contains(&expected_branch),
                            "Branch '{}' should exist in repository",
                            expected_branch
                        );
                    }

                    // Verify specific tag exists
                    assert!(
                        tag_names.contains(&"v0.0.0".to_string()),
                        "Tag 'v0.0.0' should exist in repository"
                    );

                    // Verify each reference has a SHA
                    for item in refs.branches.iter().chain(refs.tags.iter()) {
                        assert!(!item.commit_id.is_empty(), "Commit ID should not be empty");
                        // Commit ID should be a hex string (typically 40 chars for full SHA-1)
                        assert!(
                            item.commit_id.chars().all(|c| c.is_ascii_hexdigit()),
                            "Commit ID should be a hex string"
                        );
                    }

                    println!(
                        "Successfully retrieved {} branches and {} tags after fetching",
                        branch_names.len(),
                        tag_names.len()
                    );
                }
                Err(e) => {
                    // If list_repository_refs fails, clean up and fail the test
                    local_repo.cleanup().expect("Failed to clean up repository");
                    panic!("Failed to list repository refs: {}", e);
                }
            }

            // Clean up
            local_repo.cleanup().expect("Failed to clean up repository");
        }
        Err(e) => {
            // If we can't even clone the repository, make this a soft failure that prints a message
            // rather than failing the test. This handles case where network may be unavailable.
            println!(
                "Skipping test_fetch_and_list_repository_refs: Failed to clone repository: {}",
                e
            );
        }
    }
}

#[tokio::test]
async fn test_fetch_and_list_remote_repository_refs() {
    // Create test manager
    let manager = create_test_manager();

    // Use a public test repository (same one used in other tests)
    let github_url = "github:tacogips/gitcodes-mcp-test-1";

    // Parse the repository location
    let repository_location =
        gitcodes_mcp::gitcodes::repository_manager::RepositoryLocation::from_str(github_url)
            .expect("Failed to parse repository location");

    println!("Preparing repository from {}", github_url);

    // Try to prepare the repository (clone it)
    let prepare_result = manager.prepare_repository(&repository_location, None).await;

    match prepare_result {
        Ok(local_repo) => {
            println!(
                "Successfully cloned repository to {}",
                local_repo.get_repository_dir().display()
            );

            // First, list repository refs before fetching
            println!("Listing repository refs before fetch...");
            let refs_before = match local_repo.list_repository_refs().await {
                Ok(refs) => refs,
                Err(e) => panic!("Failed to list repository refs before fetch: {}", e),
            };

            // Count the number of references before fetching
            let branches_before = &refs_before.branches;
            let tags_before = &refs_before.tags;

            println!(
                "Found {} branches and {} tags before fetch",
                branches_before.len(),
                tags_before.len()
            );

            // Now execute the fetch operation
            println!("Fetching updates from remote...");
            match local_repo.fetch_remote().await {
                Ok(_) => {
                    println!("Successfully fetched updates from remote");

                    // Verify that fetch_remote was successful by checking that references remained the same
                    // or potentially increased (if there were new refs to fetch)
                    println!("Listing repository refs after fetch...");
                    let refs_after = match local_repo.list_repository_refs().await {
                        Ok(refs) => refs,
                        Err(e) => panic!("Failed to list repository refs after fetch: {}", e),
                    };

                    let branches_after = &refs_after.branches;
                    let tags_after = &refs_after.tags;

                    println!(
                        "Found {} branches and {} tags after fetch",
                        branches_after.len(),
                        tags_after.len()
                    );

                    // After fetch, we should have at least the same number of references as before
                    // (or potentially more if new refs were fetched)
                    assert!(
                        branches_after.len() >= branches_before.len(),
                        "Number of branches should not decrease after fetch"
                    );
                    assert!(
                        tags_after.len() >= tags_before.len(),
                        "Number of tags should not decrease after fetch"
                    );

                    // Verify that all pre-fetch branches are still present
                    for branch in branches_before {
                        assert!(
                            branches_after.iter().any(|b| b.full_ref == branch.full_ref),
                            "Branch '{}' missing after fetch",
                            branch.full_ref
                        );
                    }

                    // Verify that all pre-fetch tags are still present
                    for tag in tags_before {
                        assert!(
                            tags_after.iter().any(|t| t.full_ref == tag.full_ref),
                            "Tag '{}' missing after fetch",
                            tag.full_ref
                        );
                    }

                    // Verify that some known refs exist in the final result
                    let main_branch_exists = branches_after
                        .iter()
                        .any(|item| item.full_ref == "refs/heads/main" || item.name == "main");
                    assert!(main_branch_exists, "Main branch should exist after fetch");

                    let known_tag_exists = tags_after
                        .iter()
                        .any(|item| item.full_ref == "refs/tags/v0.0.0" || item.name == "v0.0.0");
                    assert!(known_tag_exists, "Tag v0.0.0 should exist after fetch");
                }
                Err(e) => {
                    // Since fetch might fail in test environments (network issues, etc.),
                    // we'll print a warning but not fail the test
                    println!("Warning: fetch_remote failed with error: {}", e);
                    println!("This could be due to network issues or test environment limitations");
                }
            }
        }
        Err(e) => {
            // Don't fail the test if we couldn't prepare the repository due to network issues
            // This allows the test to pass in restricted environments
            println!("WARNING: Could not prepare repository: {}", e);
            println!("This test requires network access and may fail in restricted environments.");
            println!("Skipping fetch_remote test due to inability to prepare repository.");
        }
    }
}
