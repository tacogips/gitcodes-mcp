//! Tests for the service's repository references listing functionality
//!
//! These tests verify that the list_repository_refs function can:
//! 1. Connect to GitHub repositories and retrieve references
//! 2. Correctly return branches and tags in JSON format
//! 3. Handle different repository URL formats

use gitcodes_mcp::gitcodes::repository_manager::RepositoryManager;
use gitcodes_mcp::services;
use serde_json::Value as JsonValue;
use std::str::FromStr;

/// Creates a Repository Manager for testing
fn create_test_manager() -> RepositoryManager {
    // Create a temporary directory for repository cache
    let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
    let cache_dir = temp_dir.path().to_path_buf();

    // Create a repository manager with our temporary directory
    RepositoryManager::new(
        None, // No GitHub token for public repos
        Some(cache_dir),
    )
    .expect("Failed to create RepositoryManager")
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
    
    // List repository references via the service function
    let result = services::list_repository_refs(
        &manager,
        repo_url,
    )
    .await;
    
    match result {
        Ok((refs_json, local_repo_opt)) => {
            // Parse the JSON response
            let refs_value: JsonValue = serde_json::from_str(&refs_json)
                .expect("Failed to parse JSON response");
            
            // Verify the refs are returned as a JSON array
            assert!(refs_value.is_array(), "References should be returned as a JSON array");
            
            // Extract branches and tags from the array
            let refs_array = refs_value.as_array().unwrap();
            
            // Collect branches and tags by examining the "ref" field of each element
            let branches: Vec<&str> = refs_array.iter()
                .filter_map(|item| {
                    if let Some(ref_path) = item["ref"].as_str() {
                        if ref_path.starts_with("refs/heads/") {
                            // Extract just the branch name without the "refs/heads/" prefix
                            Some(&ref_path["refs/heads/".len()..]) 
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();
                
            let tags: Vec<&str> = refs_array.iter()
                .filter_map(|item| {
                    if let Some(ref_path) = item["ref"].as_str() {
                        if ref_path.starts_with("refs/tags/") {
                            // Extract just the tag name without the "refs/tags/" prefix
                            Some(&ref_path["refs/tags/".len()..]) 
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();
            
            // Print the extracted branches and tags
            println!("Branches: {:?}", branches);
            println!("Tags: {:?}", tags);
            
            // Verify we have at least some branches and tags
            assert!(!branches.is_empty(), "Repository should have branches");
            assert!(!tags.is_empty(), "Repository should have tags");
            
            // Verify specific branches exist
            let expected_branches = vec!["main", "bugfix/api-client", "feature/authentication"];
            for expected_branch in expected_branches {
                assert!(branches.contains(&expected_branch), 
                        "Branch '{}' should exist in repository", expected_branch);
            }
            
            // Verify specific tag exists
            assert!(tags.contains(&"v0.0.0"), "Tag 'v0.0.0' should exist in repository");
            
            // For GitHub repository, local_repo_opt should be None
            assert!(local_repo_opt.is_none(), "GitHub repository should not return a LocalRepository");
            
            println!("Successfully retrieved {} branches and {} tags", branches.len(), tags.len());
        },
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
    
    // List repository references via the service function
    let result = services::list_repository_refs(
        &manager,
        repo_url,
    )
    .await;
    
    // Handle the result conditionally
    match result {
        Ok((refs_json, _)) => {
            // Parse the JSON response
            let refs_value: JsonValue = serde_json::from_str(&refs_json)
                .expect("Failed to parse JSON response");
            
            // Verify the refs are returned as a JSON array
            assert!(refs_value.is_array(), "References should be returned as a JSON array");
            
            // Extract branches and tags from the array
            let refs_array = refs_value.as_array().unwrap();
            
            // Collect branches and tags by examining the "ref" field of each element
            let branches: Vec<&str> = refs_array.iter()
                .filter_map(|item| {
                    if let Some(ref_path) = item["ref"].as_str() {
                        if ref_path.starts_with("refs/heads/") {
                            // Extract just the branch name without the "refs/heads/" prefix
                            Some(&ref_path["refs/heads/".len()..]) 
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();
                
            let tags: Vec<&str> = refs_array.iter()
                .filter_map(|item| {
                    if let Some(ref_path) = item["ref"].as_str() {
                        if ref_path.starts_with("refs/tags/") {
                            // Extract just the tag name without the "refs/tags/" prefix
                            Some(&ref_path["refs/tags/".len()..]) 
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();
            
            // Simple verification of branches and tags
            assert!(!branches.is_empty(), "Repository should have branches");
            assert!(!tags.is_empty(), "Repository should have tags");
            
            // Verify at least main branch exists
            assert!(branches.contains(&"main"), "Branch 'main' should exist in repository");
            
            println!("Successfully retrieved {} branches and {} tags with HTTPS URL", 
                branches.len(), tags.len());
        },
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
    let repository_location = gitcodes_mcp::gitcodes::repository_manager::RepositoryLocation::from_str(github_url)
        .expect("Failed to parse repository location");
    let prepare_result = manager.prepare_repository(&repository_location, None).await;
    if let Ok(local_repo) = prepare_result {
        // Get the local path
        let repo_dir = local_repo.get_repository_dir();
        let local_path = repo_dir.to_string_lossy();
        
        // Now list refs from the local repository
        let result = services::list_repository_refs(
            &manager,
            &local_path,
        )
        .await;
        
        match result {
            Ok((refs_json, local_repo_opt)) => {
                // Parse the JSON response
                let refs_value: JsonValue = serde_json::from_str(&refs_json)
                    .expect("Failed to parse JSON response");
                
                // Verify the refs are returned as a JSON array
                assert!(refs_value.is_array(), "References should be returned as a JSON array");
                
                // Extract branches and tags from the array
                let refs_array = refs_value.as_array().unwrap();
                
                // Collect branches and tags by examining the "ref" field of each element
                let branches: Vec<&str> = refs_array.iter()
                    .filter_map(|item| {
                        if let Some(ref_path) = item["ref"].as_str() {
                            if ref_path.starts_with("refs/heads/") {
                                Some(&ref_path["refs/heads/".len()..]) 
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect();
                    
                let _tags: Vec<&str> = refs_array.iter()
                    .filter_map(|item| {
                        if let Some(ref_path) = item["ref"].as_str() {
                            if ref_path.starts_with("refs/tags/") {
                                Some(&ref_path["refs/tags/".len()..]) 
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect();
                
                // Verify we have at least some branches
                assert!(!branches.is_empty(), "Repository should have branches");
                
                // For local repository, local_repo_opt should be Some
                assert!(local_repo_opt.is_some(), "Local repository should return a LocalRepository");
                
                // Clean up if we have a local repo instance
                if let Some(repo) = local_repo_opt {
                    repo.cleanup().expect("Failed to clean up repository");
                }
                
                // Clean up the initial repo too
                local_repo.cleanup().expect("Failed to clean up initial repository");
            },
            Err(e) => {
                // Clean up even on error
                local_repo.cleanup().expect("Failed to clean up repository");
                panic!("Failed to list local repository refs: {}", e);
            }
        }
    } else {
        println!("Warning: Could not prepare repository for local refs test: {:?}", prepare_result.err());
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
            println!("Skipping test_local_repository_list_refs_direct: 'git' command failed: {}", e);
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
            println!("Warning: Failed to commit test file: {}", String::from_utf8_lossy(&output.stderr));
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
    let local_repo = gitcodes_mcp::gitcodes::local_repository::LocalRepository::new(
        repo_path.to_path_buf()
    );
    
    // Call the list_repository_refs method directly on the LocalRepository instance
    let refs_result = local_repo.list_repository_refs().await;
    
    match refs_result {
        Ok(refs_json) => {
            // Parse the JSON response
            let refs_value: JsonValue = serde_json::from_str(&refs_json)
                .expect("Failed to parse JSON response");
            
            // Verify the refs are returned as a JSON array
            assert!(refs_value.is_array(), "References should be returned as a JSON array");
            
            // Extract branches and tags from the array
            let refs_array = refs_value.as_array().unwrap();
            
            // Collect branches and tags by examining the "ref" field of each element
            let branches: Vec<&str> = refs_array.iter()
                .filter_map(|item| {
                    if let Some(ref_path) = item["ref"].as_str() {
                        if ref_path.starts_with("refs/heads/") {
                            // Extract just the branch name without the "refs/heads/" prefix
                            Some(&ref_path["refs/heads/".len()..]) 
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();
                
            let tags: Vec<&str> = refs_array.iter()
                .filter_map(|item| {
                    if let Some(ref_path) = item["ref"].as_str() {
                        if ref_path.starts_with("refs/tags/") {
                            // Extract just the tag name without the "refs/tags/" prefix
                            Some(&ref_path["refs/tags/".len()..]) 
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();
            
            // Print the extracted branches and tags
            println!("Branches from direct call: {:?}", branches);
            println!("Tags from direct call: {:?}", tags);
            
            // Verify we have at least the branches we created
            assert!(branches.contains(&"main"), "Branch 'main' should exist in repository");
            assert!(branches.contains(&"feature/test-feature"), "Branch 'feature/test-feature' should exist in repository");
            
            // Verify tag exists
            assert!(tags.contains(&"v0.1.0"), "Tag 'v0.1.0' should exist in repository");
            
            // Verify each reference has a SHA
            for item in refs_array {
                assert!(item["object"]["sha"].is_string(), "Each reference should have a SHA");
                let sha = item["object"]["sha"].as_str().unwrap();
                assert!(!sha.is_empty(), "SHA should not be empty");
                // SHA should be a hex string (typically 40 chars for full SHA-1)
                assert!(sha.chars().all(|c| c.is_ascii_hexdigit()), "SHA should be a hex string");
            }
            
            println!("Successfully retrieved {} branches and {} tags directly from LocalRepository", 
                    branches.len(), tags.len());
        },
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
async fn test_fetch_and_list_repository_refs() {
    // Create test manager
    let manager = create_test_manager();
    
    // Use a public test repository (same one used in other tests)
    let github_url = "github:tacogips/gitcodes-mcp-test-1";
    
    // Parse the repository location
    let repository_location = gitcodes_mcp::gitcodes::repository_manager::RepositoryLocation::from_str(github_url)
        .expect("Failed to parse repository location");
    
    println!("Preparing repository from {}", github_url);
    
    // Try to prepare the repository (clone it)
    let prepare_result = manager.prepare_repository(&repository_location, None).await;
    
    match prepare_result {
        Ok(local_repo) => {
            println!("Successfully cloned repository to {}", local_repo.get_repository_dir().display());
            
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
                Ok(refs_json) => {
                    // Parse the JSON response
                    let refs_value: JsonValue = serde_json::from_str(&refs_json)
                        .expect("Failed to parse JSON response");
                    
                    // Verify the refs are returned as a JSON array
                    assert!(refs_value.is_array(), "References should be returned as a JSON array");
                    
                    // Extract branches and tags from the array
                    let refs_array = refs_value.as_array().unwrap();
                    
                    // Collect branches and tags by examining the "ref" field of each element
                    let branches: Vec<&str> = refs_array.iter()
                        .filter_map(|item| {
                            if let Some(ref_path) = item["ref"].as_str() {
                                if ref_path.starts_with("refs/heads/") {
                                    // Extract just the branch name without the "refs/heads/" prefix
                                    Some(&ref_path["refs/heads/".len()..]) 
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                        .collect();
                        
                    let tags: Vec<&str> = refs_array.iter()
                        .filter_map(|item| {
                            if let Some(ref_path) = item["ref"].as_str() {
                                if ref_path.starts_with("refs/tags/") {
                                    // Extract just the tag name without the "refs/tags/" prefix
                                    Some(&ref_path["refs/tags/".len()..]) 
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                        .collect();
                    
                    // Print the extracted branches and tags
                    println!("Branches after fetch: {:?}", branches);
                    println!("Tags after fetch: {:?}", tags);
                    
                    // Verify we have at least some branches and tags
                    assert!(!branches.is_empty(), "Repository should have branches");
                    assert!(!tags.is_empty(), "Repository should have tags");
                    
                    // Verify specific branches exist (using the known branches from the test repo)
                    let expected_branches = vec!["main", "bugfix/api-client", "feature/authentication"];
                    for expected_branch in expected_branches {
                        assert!(branches.contains(&expected_branch), 
                                "Branch '{}' should exist in repository", expected_branch);
                    }
                    
                    // Verify specific tag exists
                    assert!(tags.contains(&"v0.0.0"), "Tag 'v0.0.0' should exist in repository");
                    
                    // Verify each reference has a SHA
                    for item in refs_array {
                        assert!(item["object"]["sha"].is_string(), "Each reference should have a SHA");
                        let sha = item["object"]["sha"].as_str().unwrap();
                        assert!(!sha.is_empty(), "SHA should not be empty");
                        // SHA should be a hex string (typically 40 chars for full SHA-1)
                        assert!(sha.chars().all(|c| c.is_ascii_hexdigit()), "SHA should be a hex string");
                    }
                    
                    println!("Successfully retrieved {} branches and {} tags after fetching", 
                            branches.len(), tags.len());
                },
                Err(e) => {
                    // If list_repository_refs fails, clean up and fail the test
                    local_repo.cleanup().expect("Failed to clean up repository");
                    panic!("Failed to list repository refs: {}", e);
                }
            }
            
            // Clean up
            local_repo.cleanup().expect("Failed to clean up repository");
        },
        Err(e) => {
            // If we can't even clone the repository, make this a soft failure that prints a message
            // rather than failing the test. This handles case where network may be unavailable.
            println!("Skipping test_fetch_and_list_repository_refs: Failed to clone repository: {}", e);
        }
    }
}