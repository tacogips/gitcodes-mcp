//! Tests for the local_repository module
//!
//! These tests verify the functionality of the LocalRepository struct,
//! particularly the code search functionality.

use std::path::PathBuf;

use gitcodes_mcp::gitcodes::{
    repository_manager::RepositoryLocation, CodeSearchParams, LocalRepository,
};

/// Test fixture that creates a LocalRepository from the test repository
///
/// Returns a LocalRepository pointing to the gitcodes-mcp-test-1 repository
fn get_test_repository() -> LocalRepository {
    // Path to the test repository
    let repo_path = PathBuf::from(".private.deps-src/gitcodes-mcp-test-1");

    // Create a LocalRepository instance for testing
    LocalRepository::new(repo_path)
}

/// Tests the validation of a valid local repository
#[test]
fn test_local_repository_validation() {
    let local_repo = get_test_repository();

    // Test validation (should succeed)
    match local_repo.validate() {
        Ok(_) => println!("Successfully validated test repository"),
        Err(e) => panic!("Repository validation failed on test repository: {}", e),
    }
}

/// Tests basic code search functionality with a simple pattern
#[tokio::test]
async fn test_grep_basic_pattern() {
    let local_repo = get_test_repository();

    // Create repository location
    let repo_location = RepositoryLocation::LocalPath(local_repo.clone());

    // Test search for a basic function pattern
    let params = CodeSearchParams {
        repository_location: repo_location,
        ref_name: None,
        pattern: "fn ".to_string(), // search for function declarations
        case_sensitive: false,
        file_extensions: Some(vec!["rs".to_string()]),
        exclude_dirs: None,
        before_context: None,
        after_context: None,
    };

    // Execute the search
    let results = local_repo.search_code(params).await.expect("Search failed");

    // Should have at least one match
    assert!(!results.matches.is_empty(), "No search results found");

    // Verify the result contains expected fields
    assert_eq!(results.pattern, "fn ", "Pattern field doesn't match");

    // Check that the matches contain expected data structure
    let first_match = &results.matches[0];
    assert!(
        first_match.file_path.to_string_lossy().len() > 0,
        "Match doesn't contain file_path"
    );
    assert!(
        !first_match.line_content.is_empty(),
        "Match doesn't contain line_content"
    );

    println!(
        "Successfully found {} matches for 'fn '",
        results.matches.len()
    );
}

/// Tests case-sensitive search functionality
#[tokio::test]
async fn test_grep_case_sensitive() {
    let local_repo = get_test_repository();
    let repo_location = RepositoryLocation::LocalPath(local_repo.clone());

    // Test with case-sensitive search
    let params_case_sensitive = CodeSearchParams {
        repository_location: repo_location.clone(),
        ref_name: None,
        pattern: "Error".to_string(), // Capital E
        case_sensitive: true,
        file_extensions: Some(vec!["rs".to_string()]),
        exclude_dirs: None,
        before_context: None,
        after_context: None,
    };

    // Execute the case-sensitive search
    let results_sensitive = local_repo
        .search_code(params_case_sensitive)
        .await
        .expect("Search failed");
    let sensitive_count = results_sensitive.matches.len();

    // Test with case-insensitive search (should find more)
    let params_case_insensitive = CodeSearchParams {
        repository_location: repo_location,
        ref_name: None,
        pattern: "Error".to_string(), // Same term
        case_sensitive: false,
        file_extensions: Some(vec!["rs".to_string()]),
        exclude_dirs: None,
        before_context: None,
        after_context: None,
    };

    // Execute the case-insensitive search
    let results_insensitive = local_repo
        .search_code(params_case_insensitive)
        .await
        .expect("Search failed");
    let insensitive_count = results_insensitive.matches.len();

    // Case-insensitive search should find at least as many matches as case-sensitive
    println!(
        "Case-sensitive search found {} matches, case-insensitive found {} matches",
        sensitive_count, insensitive_count
    );

    // We should find matches in both cases
    assert!(
        sensitive_count > 0,
        "Case-sensitive search found no matches"
    );
    assert!(
        insensitive_count >= sensitive_count,
        "Case-insensitive search should find at least as many matches as case-sensitive search"
    );
}

/// Tests file extension filtering
#[tokio::test]
async fn test_grep_file_extension_filter() {
    let local_repo = get_test_repository();
    let repo_location = RepositoryLocation::LocalPath(local_repo.clone());

    // Common pattern that would exist in multiple file types
    let search_pattern = "test";

    // Search with .rs extension filter
    let params_rs = CodeSearchParams {
        repository_location: repo_location.clone(),
        ref_name: None,
        pattern: search_pattern.to_string(),
        case_sensitive: false,
        file_extensions: Some(vec!["rs".to_string()]),
        exclude_dirs: None,
        before_context: None,
        after_context: None,
    };

    // Execute the search with .rs filter
    let results_rs = local_repo
        .search_code(params_rs)
        .await
        .expect("Search failed");
    let rs_matches = &results_rs.matches;

    // Verify all matches have .rs extension
    for match_item in rs_matches {
        let file_path = match_item.file_path.to_string_lossy();
        assert!(
            file_path.ends_with(".rs"),
            "Match found in non-rs file: {}",
            file_path
        );
    }

    // Test search with a different extension (toml)
    let params_toml = CodeSearchParams {
        repository_location: repo_location.clone(),
        ref_name: None,
        pattern: search_pattern.to_string(),
        case_sensitive: false,
        file_extensions: Some(vec!["toml".to_string()]),
        exclude_dirs: None,
        before_context: None,
        after_context: None,
    };

    // Execute the search with .toml filter
    let results_toml = local_repo
        .search_code(params_toml)
        .await
        .expect("Search failed");
    let toml_matches = &results_toml.matches;

    // Verify all matches have .toml extension
    for match_item in toml_matches {
        let file_path = match_item.file_path.to_string_lossy();
        assert!(
            file_path.ends_with(".toml"),
            "Match found in non-toml file: {}",
            file_path
        );
    }

    println!(
        "Found {} matches in .rs files and {} matches in .toml files",
        rs_matches.len(),
        toml_matches.len()
    );
}

/// Tests directory exclusion functionality
#[tokio::test]
async fn test_grep_exclude_dirs() {
    let local_repo = get_test_repository();
    let repo_location = RepositoryLocation::LocalPath(local_repo.clone());

    // Search pattern that would exist in multiple directories
    let search_pattern = "fn ";

    // Search without exclusions
    let params_no_exclusion = CodeSearchParams {
        repository_location: repo_location.clone(),
        ref_name: None,
        pattern: search_pattern.to_string(),
        case_sensitive: false,
        file_extensions: Some(vec!["rs".to_string()]),
        exclude_dirs: None,
        before_context: None,
        after_context: None,
    };

    // Execute the search without exclusions
    let results_no_exclusion = local_repo
        .search_code(params_no_exclusion)
        .await
        .expect("Search failed");
    let matches_no_exclusion = &results_no_exclusion.matches;
    let no_exclusion_count = matches_no_exclusion.len();

    // Get the unique set of directories represented in the matches
    let mut dirs_with_matches = std::collections::HashSet::new();
    for match_item in matches_no_exclusion {
        let file_path = match_item.file_path.to_string_lossy();
        if let Some(parent) = std::path::Path::new(file_path.as_ref()).parent() {
            if let Some(dir_name) = parent.file_name() {
                if let Some(dir_str) = dir_name.to_str() {
                    dirs_with_matches.insert(dir_str.to_string());
                }
            }
        }
    }

    println!(
        "Found matches in these directories: {:?}",
        dirs_with_matches
    );

    // Find a directory that has matches to exclude
    let dir_to_exclude = match dirs_with_matches.iter().next() {
        Some(dir) => dir.to_string(),
        None => {
            println!("No directories with matches found, cannot perform exclusion test");
            return;
        }
    };

    println!("Will exclude directory: {}", dir_to_exclude);

    // Count matches in the directory we'll exclude
    let dir_match_count = matches_no_exclusion
        .iter()
        .filter(|m| {
            let file_path = m.file_path.to_string_lossy();
            let path = std::path::Path::new(file_path.as_ref());
            if let Some(parent) = path.parent() {
                if let Some(dir_name) = parent.file_name() {
                    return dir_name.to_str() == Some(dir_to_exclude.as_str());
                }
            }
            false
        })
        .count();

    // Ensure there are matches in the directory to make the test meaningful
    if dir_match_count == 0 {
        println!(
            "No matches found in chosen directory, skipping directory exclusion portion of test"
        );
        return;
    }

    // Now search with the chosen directory excluded
    let params_with_exclusion = CodeSearchParams {
        repository_location: repo_location,
        ref_name: None,
        pattern: search_pattern.to_string(),
        case_sensitive: false,
        file_extensions: Some(vec!["rs".to_string()]),
        exclude_dirs: Some(vec![dir_to_exclude.clone()]),
        before_context: None,
        after_context: None,
    };

    // Execute the search with exclusions
    let results_with_exclusion = local_repo
        .search_code(params_with_exclusion)
        .await
        .expect("Search failed");
    let matches_with_exclusion = &results_with_exclusion.matches;

    // Verify no matches from the excluded directory
    let excluded_matches = matches_with_exclusion
        .iter()
        .filter(|m| {
            let file_path = m.file_path.to_string_lossy();
            let path = std::path::Path::new(file_path.as_ref());
            if let Some(parent) = path.parent() {
                if let Some(dir_name) = parent.file_name() {
                    return dir_name.to_str() == Some(dir_to_exclude.as_str());
                }
            }
            false
        })
        .count();

    assert_eq!(
        excluded_matches, 0,
        "Found {} matches in excluded directory: {}",
        excluded_matches, dir_to_exclude
    );

    // Should have fewer matches with exclusion
    assert!(
        matches_with_exclusion.len() < no_exclusion_count,
        "Excluding directory didn't reduce match count"
    );

    println!(
        "Found {} matches without exclusion and {} matches with '{}' directory excluded",
        no_exclusion_count,
        matches_with_exclusion.len(),
        dir_to_exclude
    );
}

/// Tests regex pattern search
#[tokio::test]
async fn test_grep_regex_pattern() {
    let local_repo = get_test_repository();
    let repo_location = RepositoryLocation::LocalPath(local_repo.clone());

    // Complex regex pattern to find trait implementations
    let regex_pattern = r"impl\s+\w+";

    let params = CodeSearchParams {
        repository_location: repo_location,
        ref_name: None,
        pattern: regex_pattern.to_string(),
        case_sensitive: false,
        file_extensions: Some(vec!["rs".to_string()]),
        exclude_dirs: None,
        before_context: None,
        after_context: None,
    };

    // Execute the search with regex pattern
    let results = local_repo.search_code(params).await.expect("Search failed");
    let matches = &results.matches;

    // Should find some impl blocks
    assert!(!matches.is_empty(), "No matches found for regex pattern");

    println!(
        "Found {} matches for regex pattern '{}'",
        matches.len(),
        regex_pattern
    );
}

/// Tests file viewing functionality
#[tokio::test]
async fn test_view_file_contents() {
    use gitcodes_mcp::gitcodes::local_repository::ViewFileParams;

    let local_repo = get_test_repository();
    
    // 1. Test viewing a text file (Cargo.toml should always exist)
    let text_file_params = ViewFileParams {
        file_path: PathBuf::from("Cargo.toml"),
        max_size: None,
        line_from: None,
        line_to: None,
    };
    
    let text_result = local_repo.view_file_contents(text_file_params).await;
    assert!(text_result.is_ok(), "Failed to view text file: {:?}", text_result.err());
    
    // Verify that we got text content back
    match text_result.unwrap() {
        lumin::view::FileContents::Text { content, metadata } => {
            // Verify that the content is non-empty and contains typical Cargo.toml content
            assert!(!content.is_empty(), "Text file content is empty");
            assert!(content.contains("[package]"), "Cargo.toml doesn't contain [package] section");
            
            // Verify metadata
            assert!(metadata.line_count > 0, "Text file has no lines");
            assert!(metadata.char_count > 0, "Text file has no characters");
            
            println!("Successfully viewed text file with {} lines and {} characters", 
                     metadata.line_count, metadata.char_count);
        },
        _ => panic!("Expected Text content for Cargo.toml, got a different type"),
    }
    
    // 2. Test viewing a non-existent file
    let invalid_file_params = ViewFileParams {
        file_path: PathBuf::from("file-that-does-not-exist.txt"),
        max_size: None,
        line_from: None,
        line_to: None,
    };
    
    let error_result = local_repo.view_file_contents(invalid_file_params).await;
    assert!(error_result.is_err(), "Expected error for non-existent file, but got success");
    let error_msg = error_result.err().unwrap();
    assert!(error_msg.contains("not found"), "Unexpected error message: {}", error_msg);
    
    // 3. Test with a small file size limit
    // This test creates a ViewFileParams with a very small max_size (10 bytes)
    // which should cause an error when viewing a larger file like Cargo.toml
    let small_limit_params = ViewFileParams {
        file_path: PathBuf::from("Cargo.toml"),
        max_size: Some(10), // Limit to just 10 bytes
        line_from: None,
        line_to: None,
    };
    
    let size_limit_result = local_repo.view_file_contents(small_limit_params).await;
    assert!(size_limit_result.is_err(), "Expected error for size limit, but got success");
    let size_error_msg = size_limit_result.err().unwrap();
    assert!(size_error_msg.contains("too large"), "Unexpected error message: {}", size_error_msg);
    
    println!("All view_file_contents tests passed successfully");
}

/// Tests relative path handling in view_file_contents
/// 
/// This test verifies that the function correctly handles relative paths with proper safeguards:
/// - Paths relative to the repository root (with or without leading slash)
/// - Prevention of directory traversal attacks using ".."
#[tokio::test]
async fn test_view_file_contents_path_handling() {
    use gitcodes_mcp::gitcodes::local_repository::ViewFileParams;

    let local_repo = get_test_repository();
    
    // 1. Test path without leading slash
    let no_leading_slash_params = ViewFileParams {
        file_path: PathBuf::from("Cargo.toml"),
        max_size: None,
        line_from: None,
        line_to: None,
    };
    
    let no_slash_result = local_repo.view_file_contents(no_leading_slash_params).await;
    assert!(no_slash_result.is_ok(), "Failed to view file with path without leading slash: {:?}", no_slash_result.err());
    
    // 2. Test path with leading slash
    let leading_slash_params = ViewFileParams {
        file_path: PathBuf::from("/Cargo.toml"), // Note the leading slash
        max_size: None,
        line_from: None,
        line_to: None,
    };
    
    let with_slash_result = local_repo.view_file_contents(leading_slash_params).await;
    assert!(with_slash_result.is_ok(), "Failed to view file with path with leading slash: {:?}", with_slash_result.err());
    
    // 3. Test subdirectory path (depends on the test repository structure)
    // For this test, find a directory that should exist in the test repository
    // If src directory exists, use it; otherwise, this test might need adjustment
    let src_path_params = ViewFileParams {
        file_path: PathBuf::from("src/lib.rs"), // Common path in Rust projects
        max_size: None,
        line_from: None,
        line_to: None,
    };
    
    // This might fail if the test repository doesn't have this structure,
    // which is fine - we're just testing that paths work in principle
    let src_result = local_repo.view_file_contents(src_path_params).await;
    println!("Note: src/lib.rs access result: {:?}", src_result.is_ok());
    
    // 4. Test directory traversal attempt
    // Try to access a file outside the repository using ".." notation
    let traversal_params = ViewFileParams {
        file_path: PathBuf::from("../../../etc/passwd"), // Common traversal attempt
        max_size: None,
        line_from: None,
        line_to: None,
    };
    
    let traversal_result = local_repo.view_file_contents(traversal_params).await;
    assert!(traversal_result.is_err(), "Expected error for directory traversal attempt, but got success");
    
    // 5. Test another directory traversal variant
    let traversal_params2 = ViewFileParams {
        file_path: PathBuf::from("src/../../../../etc/passwd"),
        max_size: None,
        line_from: None,
        line_to: None,
    };
    
    let traversal_result2 = local_repo.view_file_contents(traversal_params2).await;
    assert!(traversal_result2.is_err(), "Expected error for directory traversal attempt, but got success");
    
    // 6. Test with normalized but suspicious path (contains ".." but stays within repo)
    let normalized_params = ViewFileParams {
        file_path: PathBuf::from("src/../Cargo.toml"),
        max_size: None,
        line_from: None,
        line_to: None,
    };
    
    let normalized_result = local_repo.view_file_contents(normalized_params).await;
    assert!(normalized_result.is_err(), "Expected error for path with '..', even if it normalizes within the repo boundary");
    
    println!("All path handling tests for view_file_contents passed successfully");
}
