//! Tests for the grep functionality with context lines
//!
//! These tests ensure the before_context and after_context parameters
//! correctly handle capturing lines of context around matches.

use std::path::PathBuf;
use tempfile::tempdir;

use gitcodes_mcp::gitcodes::repository_manager::RepositoryLocation;
use gitcodes_mcp::gitcodes::{CodeSearchParams, LocalRepository};

/// Helper function to create a test file structure for context testing
async fn create_test_repository() -> (PathBuf, tempfile::TempDir) {
    // Create a temporary directory for the test repository
    let temp_dir = tempdir().expect("Failed to create temporary directory");
    let repo_path = temp_dir.path().to_path_buf();

    // Create a simple file with numbered lines for testing context
    let test_file_path = repo_path.join("context_test.txt");
    let test_content = "\
Line 1: Introduction
\
Line 2: This is some text
\
Line 3: More sample content
\
Line 4: This line has the pattern we will search for
\
Line 5: Additional content after the match
\
Line 6: More text following the match
\
Line 7: Final content line
";

    std::fs::write(&test_file_path, test_content).expect("Failed to write test file");

    // Create a simple git repository to make validation happy
    std::process::Command::new("git")
        .current_dir(&repo_path)
        .args(["init"])
        .output()
        .expect("Failed to initialize git repository");

    std::process::Command::new("git")
        .current_dir(&repo_path)
        .args(["add", "."])
        .output()
        .expect("Failed to add files to git repository");

    std::process::Command::new("git")
        .current_dir(&repo_path)
        .args(["config", "user.name", "Test User"])
        .output()
        .expect("Failed to configure git user name");

    std::process::Command::new("git")
        .current_dir(&repo_path)
        .args(["config", "user.email", "test@example.com"])
        .output()
        .expect("Failed to configure git user email");

    std::process::Command::new("git")
        .current_dir(&repo_path)
        .args(["commit", "-m", "Initial commit"])
        .output()
        .expect("Failed to commit files to git repository");

    // Return the path to the temporary directory containing the repository
    // and the TempDir object to keep it alive
    (repo_path, temp_dir)
}

/// Test grep with no context lines
#[tokio::test]
async fn test_grep_no_context() {
    let (local_repo_path, _temp_dir) = create_test_repository().await;
    let local_repo = LocalRepository::new(local_repo_path.clone());
    let repo_location = RepositoryLocation::LocalPath(local_repo.clone());

    // Write the test file manually for complete control
    let test_file = local_repo_path.join("no_context.txt");
    std::fs::write(&test_file, "line1\nline2\npattern\nline4\nline5\n")
        .expect("Failed to write test file");

    // Add this file to git so the repo validation passes
    std::process::Command::new("git")
        .current_dir(&local_repo_path)
        .args(["add", "no_context.txt"])
        .output()
        .expect("Failed to add no_context.txt to git");

    std::process::Command::new("git")
        .current_dir(&local_repo_path)
        .args(["commit", "-m", "Add no_context.txt"])
        .output()
        .expect("Failed to commit no_context.txt");

    // Test with no context lines
    let params = CodeSearchParams {
        repository_location: repo_location,
        ref_name: None,
        pattern: "pattern".to_string(),
        case_sensitive: false,
        file_extensions: None,
        include_globs: None,
        exclude_dirs: None,
        before_context: None,
        after_context: None,
        skip: None,
        take: None,
        match_content_omit_num: Some(150),
    };

    // Execute the search
    let result = local_repo.search_code(params).await.expect("Search failed");

    // Verify search parameters are recorded correctly
    assert_eq!(result.pattern, "pattern");
    assert_eq!(result.before_context, None);
    assert_eq!(result.after_context, None);

    // Debug output
    let json = serde_json::to_string_pretty(&result).unwrap();
    println!("\nEntire search result: {}", json);

    // Count the number of matches referring to the no_context.txt file
    let no_context_matches: Vec<_> = result
        .matches
        .iter()
        .filter(|m| m.file_path.ends_with("no_context.txt"))
        .collect();

    // We should have 1 match from no_context.txt, with no context lines
    assert_eq!(
        no_context_matches.len(),
        1,
        "Expected 1 match from no_context.txt"
    );

    // Verify that only the matching line is included (not context lines)
    assert!(
        no_context_matches[0].line_content.contains("pattern"),
        "Match should contain the pattern"
    );
    assert!(
        !no_context_matches[0].is_context,
        "Match should not be marked as context"
    );

    // Make sure this match has no context flags
    let context_matches = result
        .matches
        .iter()
        .filter(|m| m.file_path.ends_with("no_context.txt") && m.is_context)
        .count();
    assert_eq!(
        context_matches, 0,
        "Should not have any context lines marked"
    );
}

/// Test grep with before context lines
#[tokio::test]
async fn test_grep_before_context() {
    let (local_repo_path, _temp_dir) = create_test_repository().await;
    let local_repo = LocalRepository::new(local_repo_path.clone());
    let repo_location = RepositoryLocation::LocalPath(local_repo.clone());

    // Write the test file manually for complete control
    let test_file = local_repo_path.join("minimal.txt");
    std::fs::write(&test_file, "line1\nline2\npattern\nline4\n")
        .expect("Failed to write test file");

    // Test with 1 line of before context
    let params = CodeSearchParams {
        repository_location: repo_location,
        ref_name: None,
        pattern: "pattern".to_string(),
        case_sensitive: false,
        file_extensions: None,
        include_globs: None,
        exclude_dirs: None,
        before_context: Some(1),
        after_context: None,
        skip: None,
        take: None,
        match_content_omit_num: Some(150),
    };

    // Add this file to git so the repo validation passes
    std::process::Command::new("git")
        .current_dir(&local_repo_path)
        .args(["add", "minimal.txt"])
        .output()
        .expect("Failed to add minimal.txt to git");

    std::process::Command::new("git")
        .current_dir(&local_repo_path)
        .args(["commit", "-m", "Add minimal.txt"])
        .output()
        .expect("Failed to commit minimal.txt");

    // Execute the search
    let result = local_repo.search_code(params).await.expect("Search failed");

    // We expect one match
    assert!(
        result.matches.len() >= 1,
        "Expected at least one match, got {}",
        result.matches.len()
    );

    // Verify search parameters are recorded correctly
    assert_eq!(result.pattern, "pattern");
    assert_eq!(result.before_context, Some(1));
    assert_eq!(result.after_context, None);

    // Verify the search results
    // Debug output to understand what's happening
    println!("Number of matches: {}", result.matches.len());
    for (i, m) in result.matches.iter().enumerate() {
        println!(
            "Match #{}: '{}' at line {}",
            i + 1,
            m.line_content.trim(),
            m.line_number
        );
    }

    // Print entire serialized result for inspection
    let json = serde_json::to_string_pretty(&result).unwrap();
    println!("\nEntire search result: {}", json);

    // Count the number of matches referring to the minimal file
    let minimal_matches: Vec<_> = result
        .matches
        .iter()
        .filter(|m| m.file_path.ends_with("minimal.txt"))
        .collect();

    // We should have 2 matches from minimal.txt: one context line and one actual match
    assert_eq!(
        minimal_matches.len(),
        2,
        "Expected 2 matches from minimal.txt"
    );

    // The first should be the context line, the second should be the actual match
    assert!(
        minimal_matches[0].line_content.contains("line2"),
        "First match should be context line"
    );
    assert!(
        minimal_matches[0].is_context,
        "First match should be marked as context"
    );
    assert!(
        minimal_matches[1].line_content.contains("pattern"),
        "Second match should contain the pattern"
    );
    assert!(
        !minimal_matches[1].is_context,
        "Second match should not be marked as context"
    );

    // We've already verified the minimal.txt file matches above
    // No need for additional assertions on context_test.txt
}

/// Test grep with after context lines
#[tokio::test]
async fn test_grep_after_context() {
    let (local_repo_path, _temp_dir) = create_test_repository().await;
    let local_repo = LocalRepository::new(local_repo_path.clone());
    let repo_location = RepositoryLocation::LocalPath(local_repo.clone());

    // Write the test file manually for complete control
    let test_file = local_repo_path.join("after.txt");
    std::fs::write(&test_file, "line1\npattern\nline3\nline4\n")
        .expect("Failed to write test file");

    // Add this file to git so the repo validation passes
    std::process::Command::new("git")
        .current_dir(&local_repo_path)
        .args(["add", "after.txt"])
        .output()
        .expect("Failed to add after.txt to git");

    std::process::Command::new("git")
        .current_dir(&local_repo_path)
        .args(["commit", "-m", "Add after.txt"])
        .output()
        .expect("Failed to commit after.txt");

    // Test with 1 line of after context
    let params = CodeSearchParams {
        repository_location: repo_location,
        ref_name: None,
        pattern: "pattern".to_string(),
        case_sensitive: false,
        file_extensions: None,
        include_globs: None,
        exclude_dirs: None,
        before_context: None,
        after_context: Some(1),
        skip: None,
        take: None,
        match_content_omit_num: Some(150),
    };

    // Execute the search
    let result = local_repo.search_code(params).await.expect("Search failed");

    // Verify search parameters are recorded correctly
    assert_eq!(result.pattern, "pattern");
    assert_eq!(result.before_context, None);
    assert_eq!(result.after_context, Some(1));

    // Verify the search results
    // Debug output to understand what's happening
    println!("Number of matches: {}", result.matches.len());
    for (i, m) in result.matches.iter().enumerate() {
        println!(
            "Match #{}: '{}' at line {}",
            i + 1,
            m.line_content.trim(),
            m.line_number
        );
    }

    let json = serde_json::to_string_pretty(&result).unwrap();
    println!("\nEntire search result: {}", json);

    // Count the number of matches referring to the after.txt file
    let after_matches: Vec<_> = result
        .matches
        .iter()
        .filter(|m| m.file_path.ends_with("after.txt"))
        .collect();

    // We should have 2 matches from after.txt: one actual match and one context line
    assert_eq!(after_matches.len(), 2, "Expected 2 matches from after.txt");

    // The first should be the match, the second should be the context line
    assert!(
        after_matches[0].line_content.contains("pattern"),
        "First match should contain the pattern"
    );
    assert!(
        !after_matches[0].is_context,
        "First match should not be marked as context"
    );
    assert!(
        after_matches[1].line_content.contains("line3"),
        "Second match should be context line"
    );
    assert!(
        after_matches[1].is_context,
        "Second match should be marked as context"
    );
}

/// Test grep with both before and after context lines
#[tokio::test]
async fn test_grep_before_after_context() {
    let (local_repo_path, _temp_dir) = create_test_repository().await;
    let local_repo = LocalRepository::new(local_repo_path.clone());
    let repo_location = RepositoryLocation::LocalPath(local_repo.clone());

    // Write the test file manually for complete control
    let test_file = local_repo_path.join("both.txt");
    std::fs::write(&test_file, "line1\nline2\npattern\nline4\nline5\n")
        .expect("Failed to write test file");

    // Add this file to git so the repo validation passes
    std::process::Command::new("git")
        .current_dir(&local_repo_path)
        .args(["add", "both.txt"])
        .output()
        .expect("Failed to add both.txt to git");

    std::process::Command::new("git")
        .current_dir(&local_repo_path)
        .args(["commit", "-m", "Add both.txt"])
        .output()
        .expect("Failed to commit both.txt");

    // Test with both before and after context
    let params = CodeSearchParams {
        repository_location: repo_location,
        ref_name: None,
        pattern: "pattern".to_string(),
        case_sensitive: false,
        file_extensions: None,
        include_globs: None,
        exclude_dirs: None,
        before_context: Some(1),
        after_context: Some(1),
        skip: None,
        take: None,
        match_content_omit_num: Some(150),
    };

    // Execute the search
    let result = local_repo.search_code(params).await.expect("Search failed");

    // Verify search parameters are recorded correctly
    assert_eq!(result.pattern, "pattern");
    assert_eq!(result.before_context, Some(1));
    assert_eq!(result.after_context, Some(1));

    // Verify the search results
    // Debug output to understand what's happening
    println!("Number of matches: {}", result.matches.len());
    for (i, m) in result.matches.iter().enumerate() {
        println!(
            "Match #{}: '{}' at line {}",
            i + 1,
            m.line_content.trim(),
            m.line_number
        );
    }

    let json = serde_json::to_string_pretty(&result).unwrap();
    println!("\nEntire search result: {}", json);

    // Count the number of matches referring to the both.txt file
    let both_matches: Vec<_> = result
        .matches
        .iter()
        .filter(|m| m.file_path.ends_with("both.txt"))
        .collect();

    // We should have 3 matches from both.txt: one before context, one match, and one after context
    assert_eq!(both_matches.len(), 3, "Expected 3 matches from both.txt");

    // Check that we have the right context lines and match
    let before_lines = both_matches
        .iter()
        .filter(|m| m.is_context && m.line_content.contains("line2"))
        .count();
    let match_lines = both_matches
        .iter()
        .filter(|m| !m.is_context && m.line_content.contains("pattern"))
        .count();
    let after_lines = both_matches
        .iter()
        .filter(|m| m.is_context && m.line_content.contains("line4"))
        .count();

    assert_eq!(before_lines, 1, "Should have 1 before context line");
    assert_eq!(match_lines, 1, "Should have 1 match line");
    assert_eq!(after_lines, 1, "Should have 1 after context line");
}
