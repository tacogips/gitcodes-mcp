//! Tests for the repository_manager module
//!
//! These tests use the test repository at https://github.com/tacogips/gitcodes-mcp-test-1
//! Note: Live integration tests are commented out to avoid external dependencies.
//! For actual testing, uncomment the integration test and set GITCODE_MCP_GITHUB_TOKEN.

use std::fs::{self, File};
use std::io::Write;

use gitcodes_mcp::gitcodes::{RepositoryManager, LocalRepository};
use gitcodes_mcp::gitcodes::repository_manager::RepositoryLocation;

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
    let mut head_file = File::create(git_dir.join("HEAD"))
        .expect("Failed to create HEAD file");
    writeln!(head_file, "ref: refs/heads/main")
        .expect("Failed to write to HEAD file");
    
    // Create a dummy README file to simulate repository content
    let mut readme_file = File::create(repo_dir.join("README.md"))
        .expect("Failed to create README file");
    writeln!(readme_file, "# Test Repository\nThis is a mock git repository for testing.")
        .expect("Failed to write to README file");
    
    // Create a LocalRepository instance pointing to our mock repository
    let local_repo = LocalRepository::new(repo_dir);
    
    // Test validation (should succeed since we created a minimal valid structure)
    local_repo.validate().expect("Repository validation failed on mock repository");
    
    println!("Successfully validated mock repository at: {}", local_repo.get_repository_dir().display());
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
    let mut head_file = File::create(git_dir.join("HEAD"))
        .expect("Failed to create HEAD file");
    writeln!(head_file, "ref: refs/heads/main")
        .expect("Failed to write to HEAD file");
    
    // Create a repository manager
    let manager = RepositoryManager::new(
        None,
        None, // Use default cache dir
    ).expect("Failed to create RepositoryManager");
    
    // Create a LocalRepository for our mock repository
    let local_repo = LocalRepository::new(repo_dir.clone());
    
    // Create a RepositoryLocation::LocalPath variant
    let repo_location = RepositoryLocation::LocalPath(local_repo);
    
    // Test prepare_repository with our local repository
    let prepared_repo = manager.prepare_repository(repo_location, None)
        .await
        .expect("Failed to prepare local repository");
    
    // Verify the prepared repository path matches our original path
    assert_eq!(
        prepared_repo.get_repository_dir().canonicalize().unwrap(),
        repo_dir.canonicalize().unwrap(),
        "Repository directory doesn't match expected path"
    );
    
    println!("Successfully prepared local repository at: {}", prepared_repo.get_repository_dir().display());
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