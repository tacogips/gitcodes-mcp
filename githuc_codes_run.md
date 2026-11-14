# GitCodes CLI Command Test Results

This document presents the results of running various command patterns with the GitCodes CLI tool.

## Command: Help

### Command: Main Help

```bash
cargo run --bin gitcodes -- --help
```

**Output:**
```
GitCodes CLI for GitHub and repository operations

Usage: gitcodes [OPTIONS] <COMMAND>

Commands:
  repository-search  Search for GitHub repositories
  grep               Search code in a GitHub repository
  list-refs          List branches and tags for a GitHub repository
  help               Print this message or the help of the given subcommand(s)

Options:
  -t, --github-token <GITHUB_TOKEN>
          GitHub API token for authentication (overrides GITCODE_MCP_GITHUB_TOKEN environment variable)
  -c, --cache-dir <REPOSITORY_CACHE_DIR>
          Custom directory for storing repository cache data Defaults to system temp directory if not specified
  -v, --verbose
          Show verbose output including INFO-level logs
  -d, --debug
          Enable debug logging
  -h, --help
          Print help
  -V, --version
          Print version
```

## Command: Repository Search

### Command: Basic Repository Search with All Parameters

```bash
cargo run --bin gitcodes -- repository-search 'rust http client' --sort-by stars --order ascending --per-page 5 --page 1
```

**Output:**
```
1. 3box/ceramic-http-client-rs - 0 stars
   Description: Rust library for talking with ceramic over http api
   URL: https://github.com/3box/ceramic-http-client-rs

2. DavidPZ142/Rust_httpClient - 0 stars
   Description: <no description>
   URL: https://github.com/DavidPZ142/Rust_httpClient

3. OniOni/rust-http - 0 stars
   Description: Toy Rust Http Client
   URL: https://github.com/OniOni/rust-http

4. bayes343/rust-http-client - 0 stars
   Description: Rust based HTTP client
   URL: https://github.com/bayes343/rust-http-client

5. ItsHyde-dev/http-client-rust - 0 stars
   Description: http client for reading requests from a file built using rust
   URL: https://github.com/ItsHyde-dev/http-client-rust
```

### Command: Repository Search with Advanced Query

```bash
cargo run --bin gitcodes -- repository-search 'language:rust stars:>1000' --per-page 5
```

**Output:**
```
1. rust-lang/rust - 103636 stars
   Description: Empowering everyone to build reliable and efficient software.
   URL: https://github.com/rust-lang/rust

2. denoland/deno - 103042 stars
   Description: A modern runtime for JavaScript and TypeScript.
   URL: https://github.com/denoland/deno

3. tauri-apps/tauri - 92663 stars
   Description: Build smaller, faster, and more secure desktop and mobile applications with a web frontend.
   URL: https://github.com/tauri-apps/tauri

4. rustdesk/rustdesk - 88605 stars
   Description: An open-source remote desktop application designed for self-hosting, as an alternative to TeamViewer.
   URL: https://github.com/rustdesk/rustdesk

5. unionlabs/union - 63686 stars
   Description: The trust-minimized, zero-knowledge bridging protocol, designed for censorship resistance, extremely high security, and usage in decentralized finance.
   URL: https://github.com/unionlabs/union
```

## Command: Grep (Code Search)

### Command: Grep with SSH URL Format

```bash
cargo run --bin gitcodes -- grep 'git@github.com:tacogips/gitcodes-mcp-test-1.git' 'fn main'
```

**Output:**
```
/tmp/mcp_gitcodes_tacogips_gitcodes-mcp-test-1_ba7ba25a9cfd2599/src/bin/main.rs:47:async fn main() -> Result<(), Box<dyn Error>> {
```

### Command: Grep with HTTP URL Format (Network Error)

```bash
cargo run --bin gitcodes -- grep 'https://github.com/tacogips/gitcodes-mcp-test-1' 'fn main'
```

**Output:**
```
Error: Failed to search code: Failed to clone repository: An IO error occurred when talking to the server
Suggestion: Failed to clone repository. Try the following:
  - Check your network connection
  - Verify the repository exists and is accessible
  - Ensure you have proper permissions (provide a GitHub token with '-t' if it's a private repository)
  - Check if the ref/branch/tag exists in the repository
```

### Command: Grep with invalid repository path

```bash
cargo run --bin gitcodes -- grep './test-repo' 'fn main'
```

**Output:**
```
Error: Failed to search code: Failed to parse repository location: invalid remote git url: ./test-repo
Suggestion: The repository location format appears to be invalid. Try the following:
  - For GitHub: Use 'https://github.com/user/repo', 'github:user/repo', or 'git@github.com:user/repo.git'
  - For local repositories: Use an absolute path or relative path to an existing local git repository
```

## Command: List Refs

### Command: List Refs for Repository (github: format)

```bash
cargo run --bin gitcodes -- list-refs 'github:tacogips/gitcodes-mcp-test-1'
```

**Output:**
```
Reference                                Type    SHA
---------------------------------------- ------- ----------------------------------------
branch: bugfix/api-client                commit  e32ffddbfd02194dcec46c990bcf30cbe22d7ada
branch: feature/authentication           commit  f3eee488ecc18ef7ca089f5e5788a8f40ef26357
branch: feature/metrics                  commit  fef32b503c2d090392e0cd7a3f6c3e1d05d3276d
branch: main                             commit  831bab9e8b529d3f2f430c4d797f440d7c8e8a27
tag: v0.0.0                              commit  831bab9e8b529d3f2f430c4d797f440d7c8e8a27
```

## Repository Location Format Summary

This section summarizes the behavior of GitCodes CLI with different repository location formats.

| Format | List-Refs | Grep | Notes |
|--------|-----------|------|-------|
| `github:user/repo` | ✅ Works | ❌ Network errors | Good for list-refs |
| `git@github.com:user/repo.git` | ✅ Works | ✅ Works | Most reliable format |
| `https://github.com/user/repo` | ✅ Works | ❌ Network errors | Good for list-refs |
| `user/repo` | ❌ Invalid | ❌ Invalid | Not recognized |
| Local paths | ❌ Issues | ❌ Issues | Path handling needs improvement |

## Improvements Made

1. **Output Format Improvements**: 
   - Changed raw JSON output to formatted, human-readable output
   - Repository search now shows structured list with name, stars, description, and URL
   - List-refs now displays a clean tabular format

2. **Reduced Logging Noise**:
   - Added `-v/--verbose` flag to control log verbosity
   - By default, only WARNING and ERROR logs are shown
   - With `-v/--verbose`, INFO-level logs are also shown
   - With `-d/--debug`, DEBUG-level logs are shown
   - Replaced direct `eprintln!` usage with tracing framework

3. **Enhanced Error Handling**:
   - Added user-friendly suggestions for common error scenarios
   - Provided specific guidance for repository cloning failures
   - Added format examples for valid repository URLs
   - Included suggestions for API rate limits and authentication issues

## Outstanding Issues

1. **Binary Name Confusion**: 
   - Multiple binaries in the project (gitcodes, gitcodes-mcp)
   - Consider standardizing on a single main binary name

2. **Cleanup Warnings**:
   - Warnings when not cleaning up local repositories
   - Consider silencing these for explicitly provided local paths

3. **Repository Location Support**:
   - SSH format (`git@github.com:user/repo.git`) works most consistently
   - Network issues with some formats need investigation
   - Local repository path handling needs improvement