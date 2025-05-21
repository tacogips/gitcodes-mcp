# gitcodes MCP

This is an MCP (Model Context Protocol) server that provides tools for Rust crate documentation lookup and GitHub code search. It allows LLMs to look up documentation for Rust crates they are unfamiliar with and search code repositories on GitHub.

## Features

- Lookup crate documentation: Get general documentation for a Rust crate
- Search crates: Search for crates on crates.io based on keywords
- Lookup item documentation: Get documentation for a specific item (e.g., struct, function, trait) within a crate
- GitHub repository search: Search for repositories on GitHub
- GitHub code search: Grep through code in GitHub repositories
- GitHub repository exploration: List branches and tags in repositories

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
cargo run --bin gitcodes-cli stdio

# Run in HTTP/SSE mode (default address: 127.0.0.1:8080)
cargo run --bin gitcodes-cli http

# Run in HTTP/SSE mode with custom address
cargo run --bin gitcodes-cli http --address 0.0.0.0:3000

# Enable debug logging
cargo run --bin gitcodes-cli http --debug
```

### GitHub Authentication

The GitHub API tools support both authenticated and unauthenticated requests with multiple authentication methods:

#### Method 1: Command Line Argument (Highest Priority)

```bash
# Provide GitHub token directly via command line
cargo run --bin gitcodes-cli http --github-token your_github_token
```

#### Method 2: Environment Variable

```bash
# Set GitHub personal access token via environment variable
export GITCODE_MCP_GITHUB_TOKEN=your_github_token

# Run with authentication
cargo run --bin gitcodes-cli http
```

#### Method 3: Custom Repository Cache Directory

You can also specify a custom directory for storing cloned repositories:

```bash
# Use a custom cache directory
cargo run --bin gitcodes-cli http --cache-dir /path/to/cache/dir
```

**Note**:

- Authentication is **optional** but recommended to avoid rate limits
- Without a token, you're limited to 60 requests/hour (vs 5,000/hour with a token)
- All operations on public repositories work without a token
- Private repositories require a token with the `repo` scope
- The token is read once at startup and stored in memory
- Command line token takes precedence over environment variable if both are provided

````

### Directly Testing Documentation Tools

You can directly test the documentation tools from the command line without starting a server:

```bash
# Get help for the test command
cargo run --bin gitcodes-cli test --tool help

# Look up crate documentation
cargo run --bin gitcodes-cli test --tool lookup_crate --crate-name tokio

# Look up item documentation
cargo run --bin gitcodes-cli test --tool lookup_item --crate-name tokio --item-path sync::mpsc::Sender

# Look up documentation for a specific version
cargo run --bin gitcodes-cli test --tool lookup_item --crate-name serde --item-path Serialize --version 1.0.147

# Search for crates
cargo run --bin gitcodes-cli test --tool search_crates --query logger --limit 5

# Output in different formats (markdown, text, json)
cargo run --bin gitcodes-cli test --tool search_crates --query logger --format json
cargo run --bin gitcodes-cli test --tool lookup_crate --crate-name tokio --format text

# Save output to a file
cargo run --bin gitcodes-cli test --tool lookup_crate --crate-name tokio --output tokio-docs.md
````

By default, the HTTP server will listen on `http://127.0.0.1:8080/sse`.

## Available Tools

The server provides the following tools:

### 1. `lookup_crate`

Retrieves documentation for a specified Rust crate.

Parameters:

- `crate_name` (required): The name of the crate to look up
- `version` (optional): The version of the crate (defaults to latest)

Example:

```json
{
  "name": "lookup_crate",
  "arguments": {
    "crate_name": "tokio",
    "version": "1.28.0"
  }
}
```

### 2. `search_crates`

Searches for Rust crates on crates.io.

Parameters:

- `query` (required): The search query
- `limit` (optional): Maximum number of results to return (defaults to 10, max 100)

Example:

```json
{
  "name": "search_crates",
  "arguments": {
    "query": "async runtime",
    "limit": 5
  }
}
```

### 3. `lookup_item`

Retrieves documentation for a specific item in a crate.

Parameters:

- `crate_name` (required): The name of the crate
- `item_path` (required): Path to the item (e.g., 'std::vec::Vec')
- `version` (optional): The version of the crate (defaults to latest)

Example:

```json
{
  "name": "lookup_item",
  "arguments": {
    "crate_name": "serde",
    "item_path": "serde::Deserialize",
    "version": "1.0.160"
  }
}
```

### 4. `search_repositories`

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

### 5. `grep_repository`

Searches for code patterns within a GitHub repository.

Parameters:

- `repository` (required): Repository URL (formats: "git@github.com:user/repo.git" (most reliable for grep), "https://github.com/user/repo", "github:user/repo")
- `ref_name` (optional): Branch or tag name (default: main or master)
- `pattern` (required): Search pattern
- `case_sensitive` (optional): Whether to be case-sensitive (default: false)
- `use_regex` (optional): Whether to use regex (default: true)
- `file_extensions` (optional): File extensions to search (e.g., ["rs", "toml"])
- `exclude_dirs` (optional): Directories to exclude (e.g., ["target", "node_modules"])
- `before_context` (optional): Number of lines to include before each match (default: 0)
- `after_context` (optional): Number of lines to include after each match (default: 0)

Example:

```json
{
  "name": "grep_repository",
  "arguments": {
    "repository": "git@github.com:tokio-rs/tokio.git",
    "ref_name": "master",
    "pattern": "async fn",
    "case_sensitive": false,
    "file_extensions": ["rs"],
    "before_context": 2,
    "after_context": 3
  }
}
```

### 6. `list_repository_refs`

Lists branches and tags for a GitHub repository.

Parameters:

- `repository` (required): Repository URL (formats: "git@github.com:user/repo.git" (most reliable), "https://github.com/user/repo", "github:user/repo")

Example:

```json
{
  "name": "list_repository_refs",
  "arguments": {
    "repository": "https://github.com/rust-lang/rust"
  }
}
```

### 7. `view_file_contents`

Retrieves the contents of a file from a GitHub repository.

Parameters:

- `repository` (required): Repository URL (formats: "git@github.com:user/repo.git", "https://github.com/user/repo", "github:user/repo")
- `ref_name` (optional): Branch or tag name (default: main or master)
- `file_path` (required): Path to the file within the repository
- `max_size` (optional): Maximum file size to read in bytes
- `line_from` (optional): Start line number (1-indexed)
- `line_to` (optional): End line number (1-indexed, inclusive)
- `without_line_numbers` (optional): Whether to display the file without line numbers (default: false)

Example:

```json
{
  "name": "view_file_contents",
  "arguments": {
    "repository": "git@github.com:rust-lang/rust.git",
    "ref_name": "master",
    "file_path": "src/libstd/lib.rs",
    "line_from": 10,
    "line_to": 30,
    "without_line_numbers": true
  }
}
```

## Implementation Notes

### Crate Documentation Features
- The server includes a caching mechanism to prevent redundant API calls for the same documentation
- It interfaces with docs.rs for crate documentation and crates.io for search functionality
- Results are returned as plain text/HTML content that can be parsed and presented by the client

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

