//! Tests for the local_repository module
//!
//! These tests verify the functionality of the LocalRepository struct,
//! particularly the code search functionality.

use std::path::PathBuf;

use gitcodes_mcp::gitcodes::{LocalRepository, repository_manager::RepositoryLocation, CodeSearchParams};

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
async fn test_search_code_basic_pattern() {
    let local_repo = get_test_repository();
    
    // Create repository location
    let repo_location = RepositoryLocation::LocalPath(local_repo.clone());
    
    // Test search for a basic function pattern
    let params = CodeSearchParams {
        repository_location: repo_location,
        ref_name: None,
        pattern: "fn ".to_string(),  // search for function declarations
        case_sensitive: false,
        file_extensions: Some(vec!["rs".to_string()]),
        exclude_dirs: None,
    };
    
    // Execute the search
    let results = local_repo.search_code(params).await.expect("Search failed");
    
    // Should have at least one result (from bin/main.rs)
    assert!(!results.is_empty(), "No search results found");
    
    // Parse the JSON result to verify structure and content
    let result_json: serde_json::Value = serde_json::from_str(&results[0]).expect("Failed to parse JSON result");
    
    // Verify the result contains expected fields
    assert!(result_json["matches"].is_array(), "Result doesn't contain matches array");
    assert!(result_json["pattern"].as_str().unwrap() == "fn ", "Pattern field doesn't match");
    
    // Verify at least one match was found
    let matches = result_json["matches"].as_array().unwrap();
    assert!(!matches.is_empty(), "No matches in results");
    
    // Check that the matches contain expected data structure
    let first_match = &matches[0];
    assert!(first_match["file_path"].is_string(), "Match doesn't contain file_path");
    assert!(first_match["line_content"].is_string(), "Match doesn't contain line_content field");
    
    println!("Successfully found {} matches for 'fn '", matches.len());
}

/// Tests case-sensitive search functionality
#[tokio::test]
async fn test_search_code_case_sensitive() {
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
    };
    
    // Execute the case-sensitive search
    let results_sensitive = local_repo.search_code(params_case_sensitive).await.expect("Search failed");
    let result_json_sensitive: serde_json::Value = serde_json::from_str(&results_sensitive[0]).expect("Failed to parse JSON result");
    let sensitive_count = result_json_sensitive["matches"].as_array().unwrap().len();
    
    // Test with case-insensitive search (should find more)
    let params_case_insensitive = CodeSearchParams {
        repository_location: repo_location,
        ref_name: None,
        pattern: "Error".to_string(), // Same term
        case_sensitive: false,
        file_extensions: Some(vec!["rs".to_string()]),
        exclude_dirs: None,
    };
    
    // Execute the case-insensitive search
    let results_insensitive = local_repo.search_code(params_case_insensitive).await.expect("Search failed");
    let result_json_insensitive: serde_json::Value = serde_json::from_str(&results_insensitive[0]).expect("Failed to parse JSON result");
    let insensitive_count = result_json_insensitive["matches"].as_array().unwrap().len();
    
    // Case-insensitive search should find at least as many matches as case-sensitive
    println!("Case-sensitive search found {} matches, case-insensitive found {} matches",
        sensitive_count, insensitive_count);
    
    // We should find matches in both cases
    assert!(sensitive_count > 0, "Case-sensitive search found no matches");
    assert!(insensitive_count >= sensitive_count, "Case-insensitive search should find at least as many matches as case-sensitive search");
}

/// Tests file extension filtering
#[tokio::test]
async fn test_search_code_file_extension_filter() {
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
    };
    
    // Execute the search with .rs filter
    let results_rs = local_repo.search_code(params_rs).await.expect("Search failed");
    let result_json_rs: serde_json::Value = serde_json::from_str(&results_rs[0]).expect("Failed to parse JSON result");
    let rs_matches = result_json_rs["matches"].as_array().unwrap();
    
    // Verify all matches have .rs extension
    for match_item in rs_matches {
        let file_path = match_item["file_path"].as_str().unwrap();
        assert!(file_path.ends_with(".rs"), "Match found in non-rs file: {}", file_path);
    }
    
    // Test search with a different extension (toml)
    let params_toml = CodeSearchParams {
        repository_location: repo_location.clone(),
        ref_name: None,
        pattern: search_pattern.to_string(),
        case_sensitive: false,
        file_extensions: Some(vec!["toml".to_string()]),
        exclude_dirs: None,
    };
    
    // Execute the search with .toml filter
    let results_toml = local_repo.search_code(params_toml).await.expect("Search failed");
    let result_json_toml: serde_json::Value = serde_json::from_str(&results_toml[0]).expect("Failed to parse JSON result");
    let toml_matches = result_json_toml["matches"].as_array().unwrap();
    
    // Verify all matches have .toml extension
    for match_item in toml_matches {
        let file_path = match_item["file_path"].as_str().unwrap();
        assert!(file_path.ends_with(".toml"), "Match found in non-toml file: {}", file_path);
    }
    
    println!("Found {} matches in .rs files and {} matches in .toml files",
        rs_matches.len(), toml_matches.len());
}

/// Tests directory exclusion functionality
#[tokio::test]
async fn test_search_code_exclude_dirs() {
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
    };
    
    // Execute the search without exclusions
    let results_no_exclusion = local_repo.search_code(params_no_exclusion).await.expect("Search failed");
    let result_json_no_exclusion: serde_json::Value = serde_json::from_str(&results_no_exclusion[0])
        .expect("Failed to parse JSON result");
    let matches_no_exclusion = result_json_no_exclusion["matches"].as_array().unwrap();
    let no_exclusion_count = matches_no_exclusion.len();
    
    // Get the unique set of directories represented in the matches
    let mut dirs_with_matches = std::collections::HashSet::new();
    for match_item in matches_no_exclusion {
        let file_path = match_item["file_path"].as_str().unwrap();
        if let Some(parent) = std::path::Path::new(file_path).parent() {
            if let Some(dir_name) = parent.file_name() {
                if let Some(dir_str) = dir_name.to_str() {
                    dirs_with_matches.insert(dir_str.to_string());
                }
            }
        }
    }
    
    println!("Found matches in these directories: {:?}", dirs_with_matches);
    
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
    let dir_match_count = matches_no_exclusion.iter().filter(|m| {
        let file_path = m["file_path"].as_str().unwrap();
        let path = std::path::Path::new(file_path);
        if let Some(parent) = path.parent() {
            if let Some(dir_name) = parent.file_name() {
                return dir_name.to_str() == Some(dir_to_exclude.as_str());
            }
        }
        false
    }).count();
    
    // Ensure there are matches in the directory to make the test meaningful
    if dir_match_count == 0 {
        println!("No matches found in chosen directory, skipping directory exclusion portion of test");
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
    };
    
    // Execute the search with exclusions
    let results_with_exclusion = local_repo.search_code(params_with_exclusion).await.expect("Search failed");
    let result_json_with_exclusion: serde_json::Value = serde_json::from_str(&results_with_exclusion[0])
        .expect("Failed to parse JSON result");
    let matches_with_exclusion = result_json_with_exclusion["matches"].as_array().unwrap();
    
    // Verify no matches from the excluded directory
    let excluded_matches = matches_with_exclusion.iter().filter(|m| {
        let file_path = m["file_path"].as_str().unwrap();
        let path = std::path::Path::new(file_path);
        if let Some(parent) = path.parent() {
            if let Some(dir_name) = parent.file_name() {
                return dir_name.to_str() == Some(dir_to_exclude.as_str());
            }
        }
        false
    }).count();
    
    assert_eq!(excluded_matches, 0, "Found {} matches in excluded directory: {}", 
        excluded_matches, dir_to_exclude);
    
    // Should have fewer matches with exclusion
    assert!(matches_with_exclusion.len() < no_exclusion_count, 
        "Excluding directory didn't reduce match count");
    
    println!("Found {} matches without exclusion and {} matches with '{}' directory excluded",
        no_exclusion_count, matches_with_exclusion.len(), dir_to_exclude);
}

/// Tests regex pattern search
#[tokio::test]
async fn test_search_code_regex_pattern() {
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
    };
    
    // Execute the search with regex pattern
    let results = local_repo.search_code(params).await.expect("Search failed");
    let result_json: serde_json::Value = serde_json::from_str(&results[0]).expect("Failed to parse JSON result");
    let matches = result_json["matches"].as_array().unwrap();
    
    // Should find some impl blocks
    assert!(!matches.is_empty(), "No matches found for regex pattern");
    
    println!("Found {} matches for regex pattern '{}'", matches.len(), regex_pattern);
}