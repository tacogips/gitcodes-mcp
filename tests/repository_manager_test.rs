//! Tests for the repository_manager module
//!
//! These tests use the test repository at https://github.com/tacogips/gitcodes-mcp-test-1
//! Note: Live integration tests are commented out to avoid external dependencies.
//! For actual testing, uncomment the integration test and set GITCODE_MCP_GITHUB_TOKEN.

use std::fs::{self, File};
use std::io::Write;

use gitcodes_mcp::gitcodes::repository_manager::RepositoryLocation;
use gitcodes_mcp::gitcodes::{LocalRepository, RepositoryManager};

/// Tests that the HTTPS to SSH URL fallback mechanism works correctly
///
/// This test uses a tracing subscriber to capture log messages and validate
/// that the fallback mechanism is activated when HTTPS URL cloning fails.
/// This test is commented out as it requires an actual repository and may
/// not work in all environments.
#[cfg(feature = "this_test_is_disabled")]
#[tokio::test]
async fn test_https_to_ssh_url_fallback() {
    use std::str::FromStr;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::Registry;
    
    // Create a repository manager
    let manager = RepositoryManager::new(None, None).expect("Failed to create RepositoryManager");
    
    // Use a real repository that exists (you can replace with any valid repo)
    let repo_url = "https://github.com/rust-lang/rust";
    
    // Capture log messages to monitor for fallback behavior
    let log_messages = Arc::new(Mutex::new(Vec::new()));
    let log_messages_clone = log_messages.clone();
    
    // Create a custom tracing layer to capture log messages
    struct LogCaptureLayer {
        messages: Arc<Mutex<Vec<String>>>,
    }
    
    impl<S> tracing_subscriber::Layer<S> for LogCaptureLayer
    where
        S: tracing::Subscriber,
    {
        fn on_event(
            &self,
            event: &tracing::Event<'_>,
            _ctx: tracing_subscriber::layer::Context<'_, S>,
        ) {
            let mut visitor = LogVisitor(self.messages.clone());
            event.record(&mut visitor);
        }
    }
    
    struct LogVisitor(Arc<Mutex<Vec<String>>>);
    
    impl tracing::field::Visit for LogVisitor {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            if field.name() == "message" {
                let message = format!("{:?}", value);
                if message.contains("HTTPS clone failed") || message.contains("Trying SSH URL") {
                    let mut messages = self.0.lock().unwrap();
                    messages.push(message);
                }
            }
        }
    }
    
    // Set up the tracing subscriber with our custom layer
    let _guard = tracing_subscriber::registry()
        .with(LogCaptureLayer {
            messages: log_messages_clone,
        })
        .set_default();
    
    // Parse the URL and attempt to prepare the repository
    let repo_location = RepositoryLocation::from_str(repo_url).expect("Failed to parse URL");
    let result = manager.prepare_repository(&repo_location, None).await;
    
    // Check if fallback messages were captured in logs
    let captured_logs = log_messages.lock().unwrap();
    let fallback_detected = captured_logs.iter().any(|msg| {
        msg.contains("HTTPS clone failed") || msg.contains("Trying SSH URL")
    });
    
    // Also check the error message if it failed
    let fallback_in_error = match &result {
        Err(e) => e.contains("HTTPS") && e.contains("SSH"),
        _ => false,
    };
    
    // The test passes if we detected the fallback in either logs or error message
    assert!(
        fallback_detected || fallback_in_error,
        "HTTPS to SSH URL fallback mechanism was not detected"
    );
    
    println!("✅ Test passed: HTTPS to SSH URL fallback mechanism working correctly");
}

/// Tests LocalRepository validation with a mock git repository
///
/// This test creates a minimal mock git repository structure and tests that
/// the validation logic in LocalRepository correctly identifies it as a valid repository.
#[test]
fn test_local_repository_validation() {
    // Create a temporary directory for our test
    let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
    let repo_dir = temp_dir.path().to_path_buf();

    // Create a minimal .git directory structure to simulate a git repository
    let git_dir = repo_dir.join(".git");
    fs::create_dir_all(&git_dir).expect("Failed to create .git directory");

    // Create a HEAD file (required for validation)
    let mut head_file = File::create(git_dir.join("HEAD")).expect("Failed to create HEAD file");
    writeln!(head_file, "ref: refs/heads/main").expect("Failed to write to HEAD file");

    // Create a dummy README file to simulate repository content
    let mut readme_file =
        File::create(repo_dir.join("README.md")).expect("Failed to create README file");
    writeln!(
        readme_file,
        "# Test Repository\nThis is a mock git repository for testing."
    )
    .expect("Failed to write to README file");

    // Create a LocalRepository instance pointing to our mock repository
    let local_repo = LocalRepository::new(repo_dir);

    // Test validation (should succeed since we created a minimal valid structure)
    local_repo
        .validate()
        .expect("Repository validation failed on mock repository");

    println!(
        "Successfully validated mock repository at: {}",
        local_repo.get_repository_dir().display()
    );
}

/// Tests that the RepositoryManager can handle local repositories
///
/// This test verifies that the `prepare_repository` function correctly handles
/// a local repository path and returns a valid LocalRepository object.
#[tokio::test]
async fn test_prepare_repository_local() {
    // Create a temporary directory for our test
    let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
    let repo_dir = temp_dir.path().to_path_buf();

    // Create a minimal .git directory structure to simulate a git repository
    let git_dir = repo_dir.join(".git");
    fs::create_dir_all(&git_dir).expect("Failed to create .git directory");

    // Create a HEAD file (required for validation)
    let mut head_file = File::create(git_dir.join("HEAD")).expect("Failed to create HEAD file");
    writeln!(head_file, "ref: refs/heads/main").expect("Failed to write to HEAD file");

    // Create a repository manager
    let manager = RepositoryManager::new(
        None, None, // Use default cache dir
    )
    .expect("Failed to create RepositoryManager");

    // Create a LocalRepository for our mock repository
    let local_repo = LocalRepository::new(repo_dir.clone());

    // Create a RepositoryLocation::LocalPath variant
    let repo_location = RepositoryLocation::LocalPath(local_repo);

    // Test prepare_repository with our local repository
    let prepared_repo = manager
        .prepare_repository(&repo_location, None)
        .await
        .expect("Failed to prepare local repository");

    // Verify the prepared repository path matches our original path
    assert_eq!(
        prepared_repo.get_repository_dir().canonicalize().unwrap(),
        repo_dir.canonicalize().unwrap(),
        "Repository directory doesn't match expected path"
    );

    println!(
        "Successfully prepared local repository at: {}",
        prepared_repo.get_repository_dir().display()
    );
}

/// Tests that the prepare_repository method handles URLs appropriately
///
/// This test verifies that the repository manager can properly handle invalid URLs
/// and provides appropriate error messages with URL format conversion suggestions.
#[tokio::test]
async fn test_repository_url_handling() {
    // Create a repository manager
    let manager = RepositoryManager::new(None, None).expect("Failed to create RepositoryManager");
    
    // Test case 1: Invalid URL format
    let invalid_url = "not-a-valid-url";
    let _repo_location = match invalid_url.parse::<RepositoryLocation>() {
        Ok(_) => panic!("Expected invalid URL to fail parsing"),
        Err(e) => {
            // Verify the error message has useful information
            assert!(e.contains("invalid"), "Error message should indicate invalid URL format");
            println!("✅ Test passed: Invalid URL properly rejected: {}", e);
        }
    };
    
    // Test case 1.1: Relative path (should be rejected)
    let relative_path = "./local-repo";
    let rel_path_result = relative_path.parse::<RepositoryLocation>();
    assert!(rel_path_result.is_err(), "Expected relative path to be rejected");
    if let Err(e) = rel_path_result {
        // The rejection happens in the GitRemoteRepository::parse_url method because the path doesn't exist,
        // so it falls back to trying to parse as a URL, which fails with 'invalid remote git url'
        assert!(e.contains("invalid remote git url"), "Error message should indicate invalid URL format");
        println!("✅ Test passed: Relative path properly rejected: {}", e);
    }
    
    // Test case 1.2: Create an absolute path and test that
    // Create a temporary directory for our test
    let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
    let repo_dir = temp_dir.path().to_path_buf();

    // Create a minimal .git directory structure to simulate a git repository
    let git_dir = repo_dir.join(".git");
    fs::create_dir_all(&git_dir).expect("Failed to create .git directory");

    // Create a HEAD file (required for validation)
    let mut head_file = File::create(git_dir.join("HEAD")).expect("Failed to create HEAD file");
    writeln!(head_file, "ref: refs/heads/main").expect("Failed to write to HEAD file");

    // This should be an absolute path (like /tmp/something) on most platforms
    let abs_path = repo_dir.to_string_lossy().to_string();
    println!("Testing with absolute path: {}", abs_path);
    
    // Absolute path to an existing git repo should work
    let abs_path_result = abs_path.parse::<RepositoryLocation>();
    assert!(abs_path_result.is_ok(), "Expected absolute path to be accepted");
    println!("✅ Test passed: Absolute path properly accepted: {}", abs_path);
    
    // Test case 2: Non-existent repository with HTTPS URL
    let nonexistent_repo = "https://github.com/nonexistent-user/nonexistent-repo-12345";
    let repo_location = nonexistent_repo.parse::<RepositoryLocation>().expect("Failed to parse URL");
    
    // Attempt to prepare the repository (should fail)
    let result = manager.prepare_repository(&repo_location, None).await;
    
    // Check that it failed as expected
    match result {
        Ok(_) => panic!("Expected non-existent repository to fail"),
        Err(e) => {
            // Print the error message for diagnostic purposes
            println!("Error message: {}", e);
            
            // Just verify the error indicates a network issue
            assert!(e.contains("clone") && (e.contains("IO error") || e.contains("failed") || 
                                             e.contains("server") || e.contains("network")),
                "Error message should indicate a network or clone issue");
            println!("✅ Test passed: Non-existent repository error handling works");
        }
    }
    
    // Test case 3: Malformed HTTPS URL (valid format but wrong protocol)
    let malformed_url = "http://github.com/user/repo"; // Using http:// instead of https://
    match malformed_url.parse::<RepositoryLocation>() {
        Ok(repo_location) => {
            // It might parse correctly even though it's slightly malformed
            let result = manager.prepare_repository(&repo_location, None).await;
            match result {
                Ok(_) => panic!("Expected malformed URL to fail"),
                Err(e) => {
                    // Check that the error message provides helpful information
                    assert!(
                        e.contains("invalid") || e.contains("failed") || e.contains("error"),
                        "Error message should indicate an issue with the URL"
                    );
                    println!("✅ Test passed: Malformed URL properly handled");
                }
            }
        },
        Err(e) => {
            // This is also a valid outcome
            println!("✅ Test passed: Malformed URL rejected during parsing: {}", e);
        }
    };
}

// UNCOMMENT THIS TEST TO RUN INTEGRATION TEST AGAINST REAL REPOSITORY
// Make sure to set the GITCODE_MCP_GITHUB_TOKEN environment variable
//
// /// Tests that the RepositoryManager can successfully clone a new repository
// ///
// /// This test verifies that the `prepare_repository` function can clone the test repository
// /// and return a valid LocalRepository object. It uses the test repository
// /// at https://github.com/tacogips/gitcodes-mcp-test-1 which is dedicated for testing.
// #[tokio::test]
// async fn test_prepare_repository_clone() {
//     // Skip the test if running in CI without token
//     let github_token = match env::var("GITCODE_MCP_GITHUB_TOKEN") {
//         Ok(token) => Some(token),
//         Err(_) => {
//             println!("No GitHub token found. This test may fail due to GitHub API rate limits.");
//             None
//         }
//     };
//
//     // Create a temporary directory for our test
//     let temp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
//     let cache_dir = PathBuf::from(temp_dir.path());
//
//     // Create a repository manager with our temporary directory
//     let manager = RepositoryManager::new(
//         github_token,
//         Some(cache_dir.clone()),
//     ).expect("Failed to create RepositoryManager");
//
//     // Test repository URL
//     let repo_url = "https://github.com/tacogips/gitcodes-mcp-test-1";
//
//     // Parse the URL into a RepositoryLocation
//     let repo_location = repo_url.parse::<RepositoryLocation>()
//         .expect("Failed to parse repository URL");
//
//     // Attempt to prepare (clone) the repository
//     let local_repo = manager.prepare_repository(repo_location, None)
//         .await
//         .expect("Failed to prepare repository");
//
//     // Verify the repository was cloned successfully
//     let repo_dir = local_repo.get_repository_dir();
//
//     // Check that the directory exists
//     assert!(repo_dir.exists(), "Repository directory does not exist");
//
//     // Check that it contains a .git directory (indicating a successful clone)
//     let git_dir = repo_dir.join(".git");
//     assert!(git_dir.exists(), "Repository does not contain a .git directory");
//
//     // Validate the repository (this calls the LocalRepository's validate method)
//     local_repo.validate().expect("Failed to validate cloned repository");
//
//     println!("Successfully cloned and validated repository at: {}", repo_dir.display());
// }
