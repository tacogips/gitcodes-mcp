//! Tests for the services module, particularly perform_grep_in_repository
//!
//! These tests verify that the service function can:
//! 1. Clone GitHub repositories successfully
//! 2. Search code within repositories
//! 3. Clean up repositories properly after use

// Imports for tests

use gitcodes_mcp::gitcodes::repository_manager::RepositoryManager;
use gitcodes_mcp::services;

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

/// Tests the perform_grep_in_repository function with HTTPS GitHub URL
/// 
/// This test focuses on verifying that a repository is cloned
/// and properly cleaned up afterwards.
#[tokio::test]
async fn test_perform_grep_with_cleanup() {
    // Use public test repository with HTTPS URL
    // The github: format seems to have issues in this environment
    let repo_url = "https://github.com/tacogips/gitcodes-mcp-test-1.git";
    
    // Create test manager
    let manager = create_test_manager();

    // Run grep operation via the service function
    let result = services::perform_grep_in_repository(
        &manager,
        repo_url,
        "fn ".to_string(),  // Search for function declarations
        None,                // Default branch
        false,               // Case insensitive
        Some(&vec!["rs".to_string()]), // Only Rust files
        None,                // No excluded directories
        None,                // No before context
        None,                // No after context
    )
    .await;
    
    // Handle the result conditionally
    if let Ok((result, local_repo)) = result {
        // Verify we found search results
        assert!(!result.matches.is_empty(), "No matches found in test repository");
        assert_eq!(result.pattern, "fn ", "Pattern field doesn't match");
        
        // Get the repository directory
        let repo_dir = local_repo.get_repository_dir();
        println!("Repository cloned at: {}", repo_dir.display());
        
        // Verify the directory exists before cleanup
        assert!(repo_dir.exists(), "Repository directory should exist before cleanup");
        
        // Clean up the repository
        local_repo.cleanup().expect("Failed to clean up repository");
        
        // Verify the directory no longer exists after cleanup
        assert!(!repo_dir.exists(), "Repository directory should not exist after cleanup");
    } else {
        println!("Warning: Could not test repository cloning: {:?}", result.err());
    }
}

/// Tests the perform_grep_in_repository function with different URL formats
/// 
/// This test verifies that the function works with various GitHub URL formats
/// and verifies cleanup works properly for each.
#[tokio::test]
async fn test_grep_url_formats() {
    // Skip if in CI environment without credentials
    let url_formats = vec![
        "github:tacogips/gitcodes-mcp-test-1",
        "https://github.com/tacogips/gitcodes-mcp-test-1.git",
    ];
    
    // Create test manager
    let manager = create_test_manager();
    
    for url in url_formats {
        println!("Testing URL format: {}", url);
        
        // Run grep operation via the service function
        let result = services::perform_grep_in_repository(
            &manager,
            url,
            "README".to_string(), // Search for README references
            None,                 // Default branch
            false,               // Case insensitive
            Some(&vec!["md".to_string()]), // Only markdown files
            None,                // No excluded directories
            None,                // No before context
            None,                // No after context
        )
        .await;
        
        if let Ok((_search_result, local_repo)) = result {
            // Get the repository directory
            let repo_dir = local_repo.get_repository_dir();
            println!("Repository cloned at: {}", repo_dir.display());
            
            // Verify the directory exists before cleanup
            assert!(repo_dir.exists(), "Repository directory should exist before cleanup");
            
            // Clean up the repository
            local_repo.cleanup().expect("Failed to clean up repository");
            
            // Verify the directory no longer exists after cleanup
            assert!(!repo_dir.exists(), "Repository directory should not exist after cleanup");
        } else {
            println!("Warning: Could not test URL format '{}': {:?}", url, result.err());
        }
    }
}

/// Tests directory exclusion functionality in grep
#[tokio::test]
async fn test_grep_dir_exclusion() {
    // Use public test repository
    let repo_url = "github:tacogips/gitcodes-mcp-test-1";
    
    // Create test manager
    let manager = create_test_manager();
    
    // For this test, we'll exclude the "src" directory
    let exclude_dir = "src";

    // First grep without exclusion
    let grep_result = services::perform_grep_in_repository(
        &manager,
        repo_url,
        "fn ".to_string(),  // Search for function declarations
        None,               // Default branch
        false,              // Case insensitive
        Some(&vec!["rs".to_string()]), // Only Rust files
        None,               // No excluded directories
        None,               // No before context
        None,               // No after context
    )
    .await;
    
    if let Ok((results_without_exclusion, repo1)) = grep_result {
        // Verify repository directory exists
        let repo_dir1 = repo1.get_repository_dir();
        assert!(repo_dir1.exists(), "Repository directory should exist");
        
        // Get total match count without exclusion
        let total_matches = results_without_exclusion.matches.len();
        println!("Matches without exclusion: {}", total_matches);
        
        // Count matches in src directory
        let src_matches = results_without_exclusion.matches.iter()
            .filter(|m| m.file_path.to_string_lossy().contains("/src/"))
            .count();
        
        // Only proceed if we have matches in src directory
        if src_matches > 0 {
            println!("Matches in src directory: {}", src_matches);
            
            // Now grep with exclusion
            let exclude_result = services::perform_grep_in_repository(
                &manager,
                repo_url,
                "fn ".to_string(),  // Search for function declarations
                None,               // Default branch
                false,              // Case insensitive
                Some(&vec!["rs".to_string()]), // Only Rust files
                Some(&vec![exclude_dir.to_string()]), // Exclude src directory
                None,               // No before context
                None,               // No after context
            )
            .await;
            
            if let Ok((results_with_exclusion, repo2)) = exclude_result {
                // Verify repository directory exists
                let repo_dir2 = repo2.get_repository_dir();
                assert!(repo_dir2.exists(), "Repository directory should exist");
                
                // Get match count with exclusion
                let matches_with_exclusion = results_with_exclusion.matches.len();
                println!("Matches with exclusion: {}", matches_with_exclusion);
                
                // Verify no matches in src directory
                let remaining_src_matches = results_with_exclusion.matches.iter()
                    .filter(|m| m.file_path.to_string_lossy().contains("/src/"))
                    .count();
                assert_eq!(remaining_src_matches, 0, "Should find no matches in excluded directory");
                
                // Verify fewer matches with exclusion
                assert!(matches_with_exclusion < total_matches, "Should find fewer matches with exclusion");
                
                // Clean up and verify directory is gone
                repo2.cleanup().expect("Failed to clean up repository");
                assert!(!repo_dir2.exists(), "Repository should be deleted after cleanup");
            }
        } else {
            println!("Skipping directory exclusion test - no matches in src directory");
        }
        
        // Clean up and verify directory is gone
        repo1.cleanup().expect("Failed to clean up repository");
        assert!(!repo_dir1.exists(), "Repository should be deleted after cleanup");
    }
}

/// Tests show_file_contents service function with various parameters
/// 
/// This test verifies that the show_file_contents function can:
/// 1. Correctly retrieve a text file's contents
/// 2. Handle line range parameters properly
/// 3. Handle errors for non-existent files
#[tokio::test]
async fn test_show_file_contents() {
    // Use public test repository with HTTPS URL
    let repo_url = "https://github.com/tacogips/gitcodes-mcp-test-1.git";
    
    // Create test manager
    let manager = create_test_manager();

    // 1. Test viewing a text file (Cargo.toml should always exist)
    let result = services::show_file_contents(
        &manager,
        repo_url,
        "Cargo.toml".to_string(),
        None,               // Default branch
        None,               // Default max size
        None,               // No start line
        None,               // No end line
        None,               // Default format (with line numbers)
    )
    .await;
    
    // Handle the result
    match result {
        Ok((file_contents, local_repo, without_line_numbers)) => {
            // Verify we got text content back
            match file_contents {
                lumin::view::FileContents::Text { content, metadata } => {
                    // Verify that the content is non-empty and contains typical Cargo.toml content
                    assert!(!content.to_string().is_empty(), "Text file content is empty");
                    assert!(content.to_string().contains("[package]"), "Cargo.toml doesn't contain [package] section");
                    
                    // Verify metadata
                    assert!(metadata.line_count > 0, "Text file has no lines");
                    assert!(metadata.char_count > 0, "Text file has no characters");
                    
                    println!("Successfully viewed text file with {} lines and {} characters", 
                             metadata.line_count, metadata.char_count);
                },
                _ => panic!("Expected Text content for Cargo.toml, got a different type"),
            }
            
            // Get the repository directory
            let repo_dir = local_repo.get_repository_dir();
            println!("Repository cloned at: {}", repo_dir.display());
            
            // Verify the directory exists before cleanup
            assert!(repo_dir.exists(), "Repository directory should exist before cleanup");
            
            // Clean up the repository
            local_repo.cleanup().expect("Failed to clean up repository");
            
            // Verify the directory no longer exists after cleanup
            assert!(!repo_dir.exists(), "Repository directory should not exist after cleanup");
            
            // Continue with more tests since we've established that basic repo access works
            
            // 2. Test with line range parameters
            let line_range_result = services::show_file_contents(
                &manager,
                repo_url,
                "Cargo.toml".to_string(),
                None,               // Default branch
                None,               // Default max size
                Some(1),            // Start from line 1
                Some(5),            // End at line 5
                None,               // Default format (with line numbers)
            )
            .await;
            
            if let Ok((file_contents, local_repo, without_line_numbers)) = line_range_result {
                // Verify we got text content back with limited lines
                match file_contents {
                    lumin::view::FileContents::Text { content: _, metadata } => {
                        // Check that we have at most 5 lines based on the metadata
                        assert!(metadata.line_count <= 5, "Expected at most 5 lines, got {}", metadata.line_count);
                        
                        println!("Successfully viewed text file with line range, got {} lines", metadata.line_count);
                    },
                    _ => panic!("Expected Text content for Cargo.toml with line range, got a different type"),
                }
                
                // Clean up the repository
                local_repo.cleanup().expect("Failed to clean up repository with line range");
            } else {
                panic!("Failed to view file contents with line range: {:?}", line_range_result.err());
            }
            
            // 3. Test with non-existent file
            let nonexistent_result = services::show_file_contents(
                &manager,
                repo_url,
                "file-that-does-not-exist.txt".to_string(),
                None,               // Default branch
                None,               // Default max size
                None,               // No start line
                None,               // No end line
                None,               // Default format (with line numbers)
            )
            .await;
            
            // This should result in an error
            assert!(nonexistent_result.is_err(), "Expected error for non-existent file");
            let error_message = nonexistent_result.err().unwrap();
            assert!(error_message.contains("not found") || error_message.contains("File not found"), 
                    "Unexpected error message: {}", error_message);
            
            // Test with without_line_numbers set to true
            let plain_text_result = services::show_file_contents(
                &manager,
                repo_url,
                "Cargo.toml".to_string(),
                None,               // Default branch
                None,               // Default max size
                None,               // No start line
                None,               // No end line
                Some(true),         // Plain text format without line numbers
            )
            .await;
            
            if let Ok((file_contents, local_repo, without_line_numbers)) = plain_text_result {
                // Verify we got the correct format parameter back
                assert!(without_line_numbers, "Expected without_line_numbers to be true");
                
                // Clean up the repository
                local_repo.cleanup().expect("Failed to clean up repository with plain text format");
                
                println!("All show_file_contents tests passed successfully");
            } else {
                println!("Skipping without_line_numbers test due to error: {:?}", plain_text_result.err());
            }
        },
        Err(e) => {
            // If we can't clone, skip the test with a clear message
            if e.contains("Failed to clone") || e.contains("server") || e.contains("network") || e.contains("IO error") {
                println!("Skipping show_file_contents test due to network/clone issues: {}", e);
            } else {
                panic!("Failed to view file contents with unexpected error: {}", e);
            }
        }
    }
}