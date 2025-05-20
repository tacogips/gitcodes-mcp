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
  -d, --debug
          Enable debug logging
  -h, --help
          Print help
  -V, --version
          Print version
```

### Command: Repository Search Help

```bash
cargo run --bin gitcodes -- repository-search --help
```

**Output:**
```
Search for GitHub repositories

Usage: gitcodes repository-search [OPTIONS] <QUERY>

Arguments:
  <QUERY>  Search query - keywords to search for repositories. Can include advanced search qualifiers like 'language:rust' or 'stars:>1000'

Options:
      --sort-by <SORT_BY>
          How to sort results (default is 'relevance') [default: relevance] [possible values: relevance, stars, forks, updated]
  -t, --github-token <GITHUB_TOKEN>
          GitHub API token for authentication (overrides GITCODE_MCP_GITHUB_TOKEN environment variable)
  -c, --cache-dir <REPOSITORY_CACHE_DIR>
          Custom directory for storing repository cache data Defaults to system temp directory if not specified
      --order <ORDER>
          Sort order (default is 'descending') [default: descending] [possible values: ascending, descending]
  -d, --debug
          Enable debug logging
      --per-page <PER_PAGE>
          Results per page (default is 30, max 100) [default: 30]
      --page <PAGE>
          Result page number (default is 1) [default: 1]
  -h, --help
          Print help
  -V, --version
          Print version
```

### Command: Grep Help

```bash
cargo run --bin gitcodes -- grep --help
```

**Output:**
```
Search code in a GitHub repository

Usage: gitcodes grep [OPTIONS] <REPOSITORY_LOCATION> <PATTERN>

Arguments:
  <REPOSITORY_LOCATION>  Repository URL or local file path - supports GitHub formats: 'https://github.com/user/repo', 'git@github.com:user/repo.git', 'github:user/repo', or local paths
  <PATTERN>              Search pattern - the text pattern to search for in the code. Supports regular expressions by default

Options:
  -r, --ref-name <REF_NAME>
          Branch, Commit or tag (default is 'main' or 'master')
  -t, --github-token <GITHUB_TOKEN>
          GitHub API token for authentication (overrides GITCODE_MCP_GITHUB_TOKEN environment variable)
  -c, --cache-dir <REPOSITORY_CACHE_DIR>
          Custom directory for storing repository cache data Defaults to system temp directory if not specified
      --case-sensitive <CASE_SENSITIVE>
          Whether to be case-sensitive [default: false] [possible values: true, false]
  -d, --debug
          Enable debug logging
  -e, --ext <FILE_EXTENSIONS>
          File extensions to search
      --exclude <EXCLUDE_DIRS>
          Directories to exclude from search
  -h, --help
          Print help
  -V, --version
          Print version
```

### Command: List Refs Help

```bash
cargo run --bin gitcodes -- list-refs --help
```

**Output:**
```
List branches and tags for a GitHub repository

Usage: gitcodes list-refs [OPTIONS] <REPOSITORY_LOCATION>

Arguments:
  <REPOSITORY_LOCATION>  Repository URL or local file path - supports GitHub formats: 'https://github.com/user/repo', 'git@github.com:user/repo.git', 'github:user/repo', or local paths

Options:
  -t, --github-token <GITHUB_TOKEN>
          GitHub API token for authentication (overrides GITCODE_MCP_GITHUB_TOKEN environment variable)
  -c, --cache-dir <REPOSITORY_CACHE_DIR>
          Custom directory for storing repository cache data Defaults to system temp directory if not specified
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
(Output was too long to include in full, but contains search results for GitHub repositories matching the query 'rust http client', sorted by stars in ascending order, with 5 results per page, starting from page 1)

**Issues:**
- No issues noted

### Command: Repository Search with Advanced Query

```bash
cargo run --bin gitcodes -- repository-search 'language:rust stars:>1000'
```

**Output:**
(Output was too long to include in full, but contains search results for GitHub repositories with language Rust and more than 1000 stars, using default sorting and pagination)

**Issues:**
- No issues noted

## Command: Grep (Code Search)

### Command: Grep from Remote Repository

```bash
cargo run --bin gitcodes -- grep 'https://github.com/tacogips/gitcodes-mcp-test-1' 'fn main'
```

**Output:**
```
2025-05-20T15:34:15.156078Z  INFO ThreadId(01) src/bin/gitcodes_cli.rs:188: GitCodes CLI initialized
2025-05-20T15:34:15.156121Z  INFO ThreadId(01) src/bin/gitcodes_cli.rs:246: Searching for code pattern in repository: https://github.com/tacogips/gitcodes-mcp-test-1
2025-05-20T15:34:15.156128Z  INFO ThreadId(01) src/bin/gitcodes_cli.rs:250: Pattern: fn main
2025-05-20T15:34:15.156207Z  INFO ThreadId(01) src/gitcodes/repository_manager/mod.rs:381: Cloning repository from https://github.com/tacogips/gitcodes-mcp-test-1 to /tmp/mcp_gitcodes_tacogips_gitcodes-mcp-test-1_a24676bed193f048
2025-05-20T15:34:15.163821Z ERROR ThreadId(01) src/bin/gitcodes_cli.rs:291: Failed to search code: Failed to clone repository: An IO error occurred when talking to the server
Error: Failed to search code: Failed to clone repository: An IO error occurred when talking to the server
```

**Issues:**
- Failed to clone repository - likely due to network or permission issues

### Command: Grep from Local Repository

```bash
cargo run --bin gitcodes -- grep './test-repo' 'fn main'
```

**Output:**
```
2025-05-20T15:34:28.300088Z  INFO ThreadId(01) src/bin/gitcodes_cli.rs:188: GitCodes CLI initialized
2025-05-20T15:34:28.300126Z  INFO ThreadId(01) src/bin/gitcodes_cli.rs:246: Searching for code pattern in repository: ./test-repo
2025-05-20T15:34:28.300132Z  INFO ThreadId(01) src/bin/gitcodes_cli.rs:250: Pattern: fn main
./test-repo/src/bin/main.rs:47:async fn main() -> Result<(), Box<dyn Error>> {

2025-05-20T15:34:28.307140Z  WARN ThreadId(01) src/bin/gitcodes_cli.rs:142: Failed to clean up repository: Refusing to delete directory './test-repo' that doesn't match temporary repository pattern
```

**Issues:**
- Warning about failing to clean up the local repository, but this is expected behavior as it shouldn't delete local repositories not created as temporary ones

### Command: Grep with File Extensions and Exclusions

```bash
cargo run --bin gitcodes -- -d grep './test-repo' 'let' -e rs --exclude target
```

**Output:**
(Output was too long to include in full, but shows numerous matches for the pattern 'let' in .rs files, excluding the target directory, with additional debug information due to the -d flag)

**Issues:**
- No issues noted, though the debug output is very verbose

## Command: List Refs

### Command: List Refs for Repository

```bash
cargo run --bin gitcodes -- list-refs 'https://github.com/tacogips/gitcodes-mcp-test-1'
```

**Output:**
```
2025-05-20T15:34:15.414267Z  INFO ThreadId(01) src/bin/gitcodes_cli.rs:188: GitCodes CLI initialized
2025-05-20T15:34:15.414305Z  INFO ThreadId(01) src/bin/gitcodes_cli.rs:299: Listing references for repository: https://github.com/tacogips/gitcodes-mcp-test-1
[{"ref":"refs/heads/bugfix/api-client","node_id":"REF_kwDOOolQYrxyZWZzL2hlYWRzL2J1Z2ZpeC9hcGktY2xpZW50","url":"https://api.github.com/repos/tacogips/gitcodes-mcp-test-1/git/refs/heads/bugfix/api-client","object":{"sha":"e32ffddbfd02194dcec46c990bcf30cbe22d7ada","type":"commit","url":"https://api.github.com/repos/tacogips/gitcodes-mcp-test-1/git/commits/e32ffddbfd02194dcec46c990bcf30cbe22d7ada"}},{"ref":"refs/heads/feature/authentication","node_id":"REF_kwDOOolQYtoAIXJlZnMvaGVhZHMvZmVhdHVyZS9hdXRoZW50aWNhdGlvbg","url":"https://api.github.com/repos/tacogips/gitcodes-mcp-test-1/git/refs/heads/feature/authentication","object":{"sha":"f3eee488ecc18ef7ca089f5e5788a8f40ef26357","type":"commit","url":"https://api.github.com/repos/tacogips/gitcodes-mcp-test-1/git/commits/f3eee488ecc18ef7ca089f5e5788a8f40ef26357"}},{"ref":"refs/heads/feature/metrics","node_id":"REF_kwDOOolQYrpyZWZzL2hlYWRzL2ZlYXR1cmUvbWV0cmljcw","url":"https://api.github.com/repos/tacogips/gitcodes-mcp-test-1/git/refs/heads/feature/metrics","object":{"sha":"fef32b503c2d090392e0cd7a3f6c3e1d05d3276d","type":"commit","url":"https://api.github.com/repos/tacogips/gitcodes-mcp-test-1/git/commits/fef32b503c2d090392e0cd7a3f6c3e1d05d3276d"}},{"ref":"refs/heads/main","node_id":"REF_kwDOOolQYq9yZWZzL2hlYWRzL21haW4","url":"https://api.github.com/repos/tacogips/gitcodes-mcp-test-1/git/refs/heads/main","object":{"sha":"831bab9e8b529d3f2f430c4d797f440d7c8e8a27","type":"commit","url":"https://api.github.com/repos/tacogips/gitcodes-mcp-test-1/git/commits/831bab9e8b529d3f2f430c4d797f440d7c8e8a27"}},{"ref":"refs/tags/v0.0.0","node_id":"REF_kwDOOolQYrByZWZzL3RhZ3MvdjAuMC4w","url":"https://api.github.com/repos/tacogips/gitcodes-mcp-test-1/git/refs/tags/v0.0.0","object":{"sha":"831bab9e8b529d3f2f430c4d797f440d7c8e8a27","type":"commit","url":"https://api.github.com/repos/tacogips/gitcodes-mcp-test-1/git/commits/831bab9e8b529d3f2f430c4d797f440d7c8e8a27"}}]
```

**Issues:**
- No issues noted, but the output is raw JSON without any formatting, which could be difficult to read for users

## Improvements Made

### Improved Output Formatting

We improved the output formatting for commands that previously returned raw JSON data. Here are examples of the new output formats:

#### Repository Search - New Format

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

3. tauri-apps/tauri - 92661 stars
   Description: Build smaller, faster, and more secure desktop and mobile applications with a web frontend.
   URL: https://github.com/tauri-apps/tauri

4. rustdesk/rustdesk - 88603 stars
   Description: An open-source remote desktop application designed for self-hosting, as an alternative to TeamViewer.
   URL: https://github.com/rustdesk/rustdesk

5. unionlabs/union - 63685 stars
   Description: The trust-minimized, zero-knowledge bridging protocol, designed for censorship resistance, extremely high security, and usage in decentralized finance.
   URL: https://github.com/unionlabs/union
```

#### List Refs - New Format

```bash
cargo run --bin gitcodes -- list-refs 'https://github.com/tacogips/gitcodes-mcp-test-1'
```

**Output:**
```
Found 5 references:
Reference                                Type    SHA
---------------------------------------- ------- ----------------------------------------
branch: bugfix/api-client                commit  e32ffddbfd02194dcec46c990bcf30cbe22d7ada
branch: feature/authentication           commit  f3eee488ecc18ef7ca089f5e5788a8f40ef26357
branch: feature/metrics                  commit  fef32b503c2d090392e0cd7a3f6c3e1d05d3276d
branch: main                             commit  831bab9e8b529d3f2f430c4d797f440d7c8e8a27
tag: v0.0.0                              commit  831bab9e8b529d3f2f430c4d797f440d7c8e8a27
```

## Overall Issues and Observations

1. **Output Format Inconsistency**: 
   - The `repository-search` and `list-refs` commands originally output raw data (JSON) without any formatting or user-friendly presentation
   - The `grep` command outputs results in a simple file:line:content format
   - This inconsistency in output presentation made it difficult for users to work with the different commands in a unified way
   - [FIXED] Improved the formatting for both `repository-search` and `list-refs` commands to display relevant information in a user-friendly format:
     - For `repository-search`: Shows repository name, stars, description, and URL in a readable format
     - For `list-refs`: Shows formatted reference names (branches/tags), type, and SHA values in a tabular format

2. **Logging Noise**:
   - All commands produce INFO-level logging to stderr by default, which clutters the output
   - Even without the debug flag, users still see log messages mixed with command output
   - Consider making the default log level more quiet (WARNING or ERROR only) and only show INFO with a verbose flag

3. **Error Handling**:
   - The error messages are generally informative but could benefit from more user-friendly suggestions for resolution
   - For example, when repository cloning fails, it could suggest checking network connectivity or GitHub token permissions

4. **Binary Name Confusion**:
   - There are multiple binaries in the project (gitcodes, gitcodes-mcp) which could cause confusion
   - It might be better to standardize on a single main binary name or clarify their distinct purposes

5. **Cleanup Issues**:
   - The tool appropriately avoids deleting local repositories but emits a warning when it doesn't clean up
   - Consider silencing these warnings for local repositories or documenting this behavior better

6. **JSON Output Readability**:
   - Consider adding a `--pretty` flag for commands that output JSON to format it in a more readable way
   - Alternatively, provide a structured output format that's easier to read without additional processing