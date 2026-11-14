//! Security-focused tests for the LocalRepository module
//!
//! This file contains comprehensive tests for security features in LocalRepository,
//! particularly focusing on preventing directory traversal attacks and other
//! file system security vulnerabilities.

use std::path::PathBuf;

use gitcodes_mcp::gitcodes::local_repository::prevent_directory_traversal;
use gitcodes_mcp::gitcodes::local_repository::*;
use gitcodes_mcp::gitcodes::repository_manager::RepositoryLocation;

/// Creates a test repository instance
fn create_test_repo() -> LocalRepository {
    // Use a temporary path for testing
    let temp_dir = std::env::temp_dir().join("mcp_test_repo");
    LocalRepository::new(temp_dir)
}

#[test]
fn test_prevent_directory_traversal() {
    // Test paths that should pass validation
    let valid_paths = vec!["file.txt", "/file.txt", "dir/file.txt", "/dir/file.txt"];

    for path_str in valid_paths {
        let path = PathBuf::from(path_str);
        assert!(
            prevent_directory_traversal(&path).is_ok(),
            "Path '{}' should be considered safe but was rejected",
            path_str
        );
    }

    // Test paths that should fail validation due to directory traversal attempts
    let invalid_paths = vec![
        "../file.txt",
        "../../file.txt",
        "dir/../file.txt",
        "/dir/../file.txt",
        "dir/subdir/../../file.txt",
        "./dir/../../../file.txt",
        "%2E%2E/file.txt",              // URL-encoded "../file.txt"
        "dir/%2E%2E/secrets.txt",       // URL-encoded "dir/../secrets.txt"
        "dir/%2e%2e/%2E%2E/system.txt", // Mixed-case URL-encoded traversal
    ];

    for path_str in invalid_paths {
        let path = PathBuf::from(path_str);
        assert!(
            prevent_directory_traversal(&path).is_err(),
            "Path '{}' contains directory traversal but was not rejected",
            path_str
        );
    }
}

#[test]
fn test_normalize_repository_path_security() {
    let repo = create_test_repo();

    // Test valid paths normalization
    // Paths without leading slash should remain unchanged
    let path1 = PathBuf::from("file.txt");
    let normalized1 = repo.normalize_repository_path(&path1).unwrap();
    assert_eq!(
        normalized1, path1,
        "Path without leading slash should remain unchanged"
    );

    // Paths with leading slash should have it removed
    let path2 = PathBuf::from("/file.txt");
    let normalized2 = repo.normalize_repository_path(&path2).unwrap();
    assert_eq!(
        normalized2,
        PathBuf::from("file.txt"),
        "Path with leading slash should have it removed"
    );

    // Test invalid paths
    let invalid_paths = vec!["../file.txt", "dir/../file.txt", "/dir/../file.txt"];

    for path_str in invalid_paths {
        let path = PathBuf::from(path_str);
        assert!(
            repo.normalize_repository_path(&path).is_err(),
            "Path '{}' should fail normalization due to directory traversal",
            path_str
        );
    }
}

#[test]
fn test_view_file_params_security() {
    let repo = create_test_repo();

    // Create test ViewFileParams with valid and invalid paths
    let valid_params = ViewFileParams {
        file_path: PathBuf::from("README.md"),
        max_size: None,
        line_from: None,
        line_to: None,
    };

    // These params should be rejected due to directory traversal
    let invalid_params = ViewFileParams {
        file_path: PathBuf::from("../secret.txt"),
        max_size: None,
        line_from: None,
        line_to: None,
    };

    // Calling normalize_repository_path directly should work for valid paths
    assert!(
        repo.normalize_repository_path(&valid_params.file_path)
            .is_ok(),
        "Valid ViewFileParams should have their path successfully normalized"
    );

    // Calling normalize_repository_path directly should fail for invalid paths
    assert!(
        repo.normalize_repository_path(&invalid_params.file_path)
            .is_err(),
        "Invalid ViewFileParams should fail path normalization"
    );

    // Ensure prevent_directory_traversal catches the invalid path
    assert!(
        prevent_directory_traversal(&invalid_params.file_path).is_err(),
        "Invalid path with directory traversal should be caught by prevent_directory_traversal"
    );
}

#[test]
fn test_comprehensive_path_security() {
    // Test various path security exploits that should all be caught
    let exploit_attempts = vec![
        // Basic traversal patterns
        "../sensitive-file.txt",
        "../../etc/passwd",
        "config/../../etc/shadow",
        "/var/../../etc/hosts",
        // Encoded traversal attempts
        // Note: These might need separate handling if the code performs URL decoding
        "..%2Fsensitive-file.txt",
        "%2E%2E/config.json",
        // Mixed slash variants
        "dir\\..\\..\\windows\\system32",
        // Long chains of traversal
        "assets/images/avatars/../../../../../etc/passwd",
        // Attempt with trailing dots
        "secret/file.txt..",
        // Combination of ./.. patterns
        "./config/../../../etc/passwd",
    ];

    for path_str in exploit_attempts {
        let path = PathBuf::from(path_str);
        assert!(
            prevent_directory_traversal(&path).is_err(),
            "Security exploit path '{}' was not properly rejected",
            path_str
        );
    }
}

#[test]
fn test_directory_traversal_in_exclude_dirs() {
    // Instead of running the full search_code method (which requires a valid repository),
    // we'll test the path validation directly for exclude_dirs

    // Valid directory paths should pass
    let valid_dirs = vec!["valid_dir", "nested/valid_dir", "/valid_dir"];

    for dir in valid_dirs {
        assert!(
            prevent_directory_traversal(&PathBuf::from(dir)).is_ok(),
            "Valid directory path '{}' should be accepted",
            dir
        );
    }

    // Invalid directory paths should fail
    let invalid_dirs = vec![
        "../invalid_dir",
        "nested/../invalid_dir",
        "invalid_dir/../../../etc",
        "%2E%2E/invalid_dir",
    ];

    for dir in invalid_dirs {
        assert!(
            prevent_directory_traversal(&PathBuf::from(dir)).is_err(),
            "Invalid directory path '{}' should be rejected",
            dir
        );
    }
}

#[tokio::test]
async fn test_search_code_integration() {
    // Create a temporary directory that actually exists
    let temp_dir = tempfile::tempdir().unwrap();
    let repo = LocalRepository::new(temp_dir.path().to_path_buf());

    // Create a simple .git directory to pass repository validation
    let git_dir = temp_dir.path().join(".git");
    std::fs::create_dir_all(&git_dir).unwrap();

    // Test that search_code properly checks exclude_dirs for directory traversal
    let params = CodeSearchParams {
        repository_location: RepositoryLocation::LocalPath(repo.clone()),
        ref_name: None,
        pattern: "test".to_string(),
        case_sensitive: false,
        file_extensions: None,
        include_globs: None,
        exclude_dirs: Some(vec![
            "valid_dir".to_string(),
            "../invalid_dir".to_string(), // This should trigger the directory traversal check
        ]),
        before_context: None,
        after_context: None,
        skip: None,
        take: None,
        match_content_omit_num: Some(150),
    };

    let result = repo.search_code(params).await;

    assert!(
        result.is_err(),
        "search_code should reject parameters with directory traversal in exclude_dirs"
    );
    let error_message = result.err().unwrap();
    assert!(
        error_message.contains("Invalid exclude_dir path"),
        "Error message should indicate the problem is with exclude_dir path: {}",
        error_message
    );
}
