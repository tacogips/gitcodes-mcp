use gitcodes_mcp::gitcodes::repository_manager::providers::github::parse_github_url;
use gitcodes_mcp::gitcodes::repository_manager::providers::GitRemoteRepository;

#[test]
fn test_parse_github_url() {
    // Test cases for different URL formats
    let test_cases = [
        // HTTPS URLs
        (
            "https://github.com/user/repo",
            "user",
            "repo",
            "git@github.com:user/repo.git",
        ),
        (
            "https://github.com/user/repo.git",
            "user",
            "repo",
            "git@github.com:user/repo.git",
        ),
        (
            "https://github.com/user/repo/",
            "user",
            "repo",
            "git@github.com:user/repo.git",
        ),
        (
            "https://github.com/user/repo.git/",
            "user",
            "repo",
            "git@github.com:user/repo.git",
        ),
        (
            "https://github.com/user/multi-part-repo",
            "user",
            "multi-part-repo",
            "git@github.com:user/multi-part-repo.git",
        ),
        (
            "https://github.com/org-name/repo",
            "org-name",
            "repo",
            "git@github.com:org-name/repo.git",
        ),
        (
            "https://github.com/user/repo-with-dots.js",
            "user",
            "repo-with-dots.js",
            "git@github.com:user/repo-with-dots.js.git",
        ),
        // SSH URLs
        (
            "git@github.com:user/repo.git",
            "user",
            "repo",
            "git@github.com:user/repo.git",
        ),
        (
            "git@github.com:user/repo",
            "user",
            "repo",
            "git@github.com:user/repo.git",
        ),
        (
            "git@github.com:user/repo/",
            "user",
            "repo",
            "git@github.com:user/repo.git",
        ),
        (
            "git@github.com:org-name/repo-name.git",
            "org-name",
            "repo-name",
            "git@github.com:org-name/repo-name.git",
        ),
        // Shorthand URLs
        (
            "github:user/repo",
            "user",
            "repo",
            "git@github.com:user/repo.git",
        ),
        (
            "github:user/repo.git",
            "user",
            "repo",
            "git@github.com:user/repo.git",
        ),
        (
            "github:user/repo/",
            "user",
            "repo",
            "git@github.com:user/repo.git",
        ),
        (
            "github:/user/repo",
            "user",
            "repo",
            "git@github.com:user/repo.git",
        ),
    ];

    for (input_url, expected_user, expected_repo, expected_ssh_url) in test_cases {
        // Parse the URL
        let result = parse_github_url(input_url);
        assert!(result.is_ok(), "Failed to parse URL: {}", input_url);

        // Verify the parsed information
        let github_info = result.unwrap();
        assert_eq!(
            github_info.repo_info.user, expected_user,
            "Wrong user for URL: {}",
            input_url
        );
        assert_eq!(
            github_info.repo_info.repo, expected_repo,
            "Wrong repo for URL: {}",
            input_url
        );

        // Verify to_ssh_url() method returns the expected SSH URL
        assert_eq!(
            github_info.to_ssh_url(),
            expected_ssh_url,
            "Wrong SSH URL for: {}",
            input_url
        );
    }
}

#[test]
fn test_invalid_github_urls() {
    let invalid_urls = [
        "https://example.com/user/repo",     // Not a GitHub URL
        "git@gitlab.com:user/repo.git",      // Not GitHub
        "https://github.com/incomplete",     // No repo part
        "https://github.com/too/many/parts", // Too many parts
        "http:github.com/user/repo",         // Invalid protocol
        "github:incomplete",                 // Incomplete shorthand
    ];

    for invalid_url in invalid_urls {
        let result = parse_github_url(invalid_url);
        assert!(
            result.is_err(),
            "Should fail to parse invalid URL: {}",
            invalid_url
        );
    }
}

#[test]
fn test_git_remote_repository_to_ssh_url() {
    // Test the GitRemoteRepository wrapper for to_ssh_url()
    let test_urls = [
        "https://github.com/user/repo",
        "git@github.com:user/repo.git",
        "github:user/repo",
    ];

    for url in test_urls {
        // Parse via GitRemoteRepository
        let remote_repo = GitRemoteRepository::parse_url(url).expect("Should parse URL");

        // Parse directly via parse_github_url
        let github_info = parse_github_url(url).expect("Should parse URL");

        // Both methods should produce the same SSH URL
        assert_eq!(remote_repo.to_ssh_url(), github_info.to_ssh_url());
        assert_eq!(remote_repo.to_ssh_url(), "git@github.com:user/repo.git");
    }
}

#[test]
fn test_url_normalization() {
    // Test URL normalization during parsing
    // This checks our handling of .git suffixes, trailing slashes, etc.
    let test_cases = [
        // Input URL, Expected normalized URL after parsing
        (
            "https://github.com/user/repo",
            "https://github.com/user/repo.git",
        ),
        (
            "https://github.com/user/repo.git",
            "https://github.com/user/repo.git",
        ),
        (
            "https://github.com/user/repo/",
            "https://github.com/user/repo.git",
        ),
        ("git@github.com:user/repo", "git@github.com:user/repo"), // SSH URLs are preserved as-is
        (
            "git@github.com:user/repo.git",
            "git@github.com:user/repo.git",
        ),
        ("github:user/repo", "https://github.com/user/repo.git"), // github: shorthand becomes HTTPS
    ];

    for (input_url, expected_normalized_url) in test_cases {
        let github_info = parse_github_url(input_url).expect("Should parse URL");
        assert_eq!(
            github_info.clone_url, expected_normalized_url,
            "Normalization failed for {}",
            input_url
        );
    }
}
