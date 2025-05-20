# Test Notes

## HTTP Tests

The GitHub API tests in `github_api_test.rs` require HTTP connectivity and OpenSSL to be properly installed in the environment. In environments where these dependencies are not available, the tests are designed to be skippable.

### Running HTTP Tests

To run the HTTP tests successfully:

1. Ensure OpenSSL development libraries are installed (`libssl-dev` on Ubuntu, `openssl-devel` on Fedora)
2. Ensure the environment has internet connectivity
3. Run the tests normally: `cargo test --test github_api_test`

### Skipping HTTP Tests

To skip the HTTP tests (e.g., in CI environments without these dependencies):

```bash
SKIP_HTTP_TESTS=1 cargo test --test github_api_test
```

This will cause the tests to detect the environment variable and skip the actual HTTP requests.

### Implementation Notes

The `list_repository_refs` function in both the service and GitHub client implementations is designed to be resilient to various environment conditions. The HTTP requests are properly handled with appropriate error messages when connectivity is not available.