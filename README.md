# gitcodes MCP

This is an MCP (Model Context Protocol) server that provides tools for GitHub code search and repository exploration. It allows LLMs to search GitHub repositories, grep through code, view file contents, and explore repository structures.

## Features

- GitHub repository search: Search for repositories on GitHub
- GitHub code search: Grep through code in GitHub repositories using regex patterns
- File content viewing: View file contents with line range support
- Repository exploration: List branches, tags, and directory trees
- Local repository caching: Clone and cache repositories for efficient searching
- Authentication support: Works with both public and private repositories

## Installation

```bash
git clone https://github.com/d6e/gitcodes-mcp.git
cd gitcodes-mcp
cargo build --release
```

## Running the Server

There are multiple ways to run the documentation server:

### Using the Unified CLI

The unified command-line interface provides subcommands for all server modes:

```bash
# Run in STDIN/STDOUT mode
cargo run --bin gitcodes-mcp stdio

# Run in HTTP/SSE mode (default address: 127.0.0.1:8080)
cargo run --bin gitcodes-mcp http

# Run in HTTP/SSE mode with custom address
cargo run --bin gitcodes-mcp http --address 0.0.0.0:3000

# Enable debug logging
cargo run --bin gitcodes-mcp http --debug
```

### GitHub Authentication

The GitHub API tools support both authenticated and unauthenticated requests with multiple authentication methods:

#### Method 1: Command Line Argument (Highest Priority)

```bash
# Provide GitHub token directly via command line
cargo run --bin gitcodes-mcp http --github-token your_github_token
```

#### Method 2: Environment Variable

```bash
# Set GitHub personal access token via environment variable
export GITCODES_MCP_GITHUB_TOKEN=your_github_token

# Run with authentication
cargo run --bin gitcodes-mcp http
```

#### Method 3: Custom Repository Cache Directory

You can also specify a custom directory for storing cloned repositories:

```bash
# Use a custom cache directory
cargo run --bin gitcodes-mcp http --cache-dir /path/to/cache/dir
```

**Note**:

- Authentication is **optional** but recommended to avoid rate limits
- Without a token, you're limited to 60 requests/hour (vs 5,000/hour with a token)
- All operations on public repositories work without a token
- Private repositories require a token with the `repo` scope
- The token is read once at startup and stored in memory
- Command line token takes precedence over environment variable if both are provided

````

### Directly Testing GitHub Code Tools

You can directly test the GitHub code tools from the command line without starting a server:

```bash
# Search for repositories
cargo run --bin gitcodes-cli repository-search "rust http client"

# Search code in a repository
cargo run --bin gitcodes-cli grep "git@github.com:rust-lang/rust.git" "fn main"

# View file contents
cargo run --bin gitcodes-cli show-file "github:user/repo" "README.md"

# List repository branches and tags
cargo run --bin gitcodes-cli list-refs "https://github.com/rust-lang/rust"

# Get repository directory tree
cargo run --bin gitcodes-cli tree "git@github.com:user/repo.git"
```

By default, the HTTP server will listen on `http://127.0.0.1:8080/sse`.

## Available Tools

The server provides the following tools:

### 1. `search_repositories`

Searches for repositories on GitHub.

Parameters:

- `query` (required): The search query string
- `sort_by` (optional): How to sort results (Options: "Relevance", "Stars", "Forks", "Updated")
- `order` (optional): Sort order (Options: "Ascending", "Descending", default: "Descending")
- `per_page` (optional): Results per page (default: 30, max: 100)
- `page` (optional): Page number (default: 1)

Example:

```json
{
  "name": "search_repositories",
  "arguments": {
    "query": "rust http client",
    "sort_by": "Stars",
    "order": "Descending",
    "per_page": 10,
    "page": 1
  }
}
```

**Note**: The `sort_by` and `order` parameters now accept enum values directly rather than strings, improving type safety and enabling IDE autocompletion. These options are standardized across different Git providers through a unified type system.

### 2. `grep_repository`

Searches for code patterns within a GitHub repository.

Parameters:

- `repository_location` (required): Repository URL or local path (formats: "git@github.com:user/repo.git" (SSH, recommended), "https://github.com/user/repo", "github:user/repo", or absolute local paths)
- `ref_name` (optional): Branch, commit, or tag (default: main or master)
- `pattern` (required): Regular expression pattern to search for
- `case_sensitive` (optional): Case-sensitive matching (default: false)
- `file_extensions` (optional, deprecated): File extensions to search - use `include_globs` instead
- `include_globs` (optional): Glob patterns to include in search (e.g., ["**/*.rs", "src/**/*.md"])
- `exclude_dirs` (optional): Directories to exclude (e.g., ["target", "node_modules"])
- `before_context` (optional): Lines of context before each match (default: 0)
- `after_context` (optional): Lines of context after each match (default: 0)
- `skip` (optional): Number of results to skip for pagination
- `take` (optional): Maximum number of results to return
- `max_content_length` (optional): Maximum characters to show from matched content (default: 150)

Example:

```json
{
  "name": "grep_repository",
  "arguments": {
    "repository_location": "git@github.com:tokio-rs/tokio.git",
    "ref_name": "master",
    "pattern": "async fn",
    "case_sensitive": false,
    "include_globs": ["**/*.rs"],
    "exclude_dirs": ["target"],
    "before_context": 2,
    "after_context": 3
  }
}
```

### 3. `grep_repository_match_line_number`

Counts matching lines in repository code search (like `grep_repository` but returns only the total count).

Parameters: Same as `grep_repository`

Example:

```json
{
  "name": "grep_repository_match_line_number",
  "arguments": {
    "repository_location": "git@github.com:user/repo.git",
    "pattern": "fn main"
  }
}
```

### 4. `list_repository_refs`

Lists all branches and tags for a repository.

Parameters:

- `repository_location` (required): Repository URL or local path (formats: "git@github.com:user/repo.git" (SSH, recommended), "https://github.com/user/repo", "github:user/repo", or absolute local paths)

Example:

```json
{
  "name": "list_repository_refs",
  "arguments": {
    "repository_location": "https://github.com/rust-lang/rust"
  }
}
```

### 5. `show_file_contents`

Views file contents from repositories or local directories in compact format. Returns concatenated line contents with line numbers and enhanced metadata including file path.

Parameters:

- `repository_location` (required): Repository URL or local path (formats: "git@github.com:user/repo.git" (SSH, recommended), "https://github.com/user/repo", "github:user/repo", or absolute local paths)
- `ref_name` (optional): Branch, commit, or tag (default: main or master)
- `file_path` (required): File path relative to repository root
- `max_size` (optional): Maximum file size in bytes
- `line_from` (optional): Start line number (1-indexed)
- `line_to` (optional): End line number (1-indexed, inclusive)
- `without_line_numbers` (optional): Show content without line numbers (default: false)

Example:

```json
{
  "name": "show_file_contents",
  "arguments": {
    "repository_location": "git@github.com:rust-lang/rust.git",
    "ref_name": "master",
    "file_path": "README.md",
    "line_from": 10,
    "line_to": 30,
    "without_line_numbers": true
  }
}
```

**Response Format:**

The tool returns a compact JSON structure:

```json
{
  "type": "text",
  "line_contents": "1:## User Guide\n2:\n3:This guide explains how to use...",
  "metadata": {
    "file_path": "README.md",
    "line_count": 100,
    "size": 1234
  }
}
```

Key features of the compact format:
- Line contents are concatenated into a single string with line numbers (format: `line_number:content`)
- Metadata includes full file path instead of just filename
- Size field shows total bytes/characters
- Significantly reduced JSON verbosity compared to individual line objects

### 6. `get_repository_tree`

Gets repository directory tree in hierarchical format.

Parameters:

- `repository_location` (required): Repository URL or local path (formats: "git@github.com:user/repo.git" (SSH, recommended), "https://github.com/user/repo", "github:user/repo", or absolute local paths)
- `ref_name` (optional): Branch, commit, or tag (default: main or master)
- `case_sensitive` (optional): Case-sensitive path matching (default: false)
- `respect_gitignore` (optional): Respect .gitignore files (default: true)
- `depth` (optional): Maximum traversal depth (default: unlimited)
- `strip_path_prefix` (optional): Strip repository path prefix (default: true)
- `search_relative_path` (optional): Relative path to start tree generation from

Example:

```json
{
  "name": "get_repository_tree",
  "arguments": {
    "repository_location": "github:user/repo",
    "depth": 2,
    "search_relative_path": "src"
  }
}
```

## Implementation Notes

### GitHub Code Search Features
- Repositories are cloned to a local cache directory using shallow clones (--depth=1)
- Repository cache directories are reused for subsequent searches on the same repository
- Repositories are automatically updated (git pull) when accessed
- Cache directory paths follow a deterministic naming pattern based on repository owner and name
- Git references (branches, tags) can be specified for search operations
- Automatic URL format fallback: HTTPS URLs are automatically converted to SSH format if the initial clone fails
- Detailed error messages with specific suggestions based on error type
- Type-safe enumeration system for search options with compile-time validation
- Domain-specific enum types with automatic conversion between generic and provider-specific options
- Lightweight type system for repository locations (GitHub URL vs. local path)
- Support for multiple authentication methods (command line, environment variable)
- Context-aware search results with configurable lines before and after matches
- File viewing with optional line number display

## MCP Protocol Integration

This server implements the Model Context Protocol (MCP) which allows it to be easily integrated with LLM clients that support the protocol. For more information about MCP, visit [the MCP repository](https://github.com/modelcontextprotocol/mcp).

## License

MIT License

