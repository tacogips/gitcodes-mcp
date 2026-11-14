# Test Notes

## HTTP Tests

The GitHub API tests in `github_api_test.rs` require HTTP connectivity to be properly available in the environment. For HTTP connections, the project uses `reqwest` with `rustls` as the TLS backend, which is a pure-Rust TLS implementation that doesn't depend on any system TLS libraries, making it more portable across different environments. For Git operations, the project uses `gix` with its default transport implementation.

### Running HTTP Tests

To run the HTTP tests successfully:

1. No specific TLS libraries are required for HTTP operations as the project uses `reqwest` with the `rustls-tls` feature
2. For Git operations (cloning, fetching), the standard Git dependencies may be required
3. Ensure the environment has internet connectivity
4. Run the tests normally: `cargo test --test github_api_test`

### Implementation Notes

The `list_repository_refs` function in both the service and GitHub client implementations is designed to handle GitHub API requests with proper error messages. The implementation follows the same patterns as the existing code search functionality in the codebase.