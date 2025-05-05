# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Documentation

When working with this codebase, refer to the following key documents:

- **CLAUDE.md** (this file): Primary instructions for working with the codebase
- **spec.md**: Library specifications and implementation details
- **devlog.md**: Development log with design decisions, implementation details, and roadmap
- **Rustdoc comments**: In-code documentation accessible via `cargo doc --open`
- **README.md**: User-facing documentation and usage instructions

For library-specific specifications and implementation details, refer to spec.md.
To understand the history, architecture decisions, and implementation details of this project, always refer to the devlog.md file.
For API details and function-level documentation, consult the Rustdoc comments in the source code.
When making significant changes, update both devlog.md and relevant Rustdoc comments to document your work.

## Build & Run Commands

- Build: `cargo build`
- Run: `cargo run`
- Release build: `cargo build --release`
- Test: `cargo test`
- Run single test: `cargo test test_name`
- Lint: `cargo clippy`
- Format: `cargo fmt`

## Code Style Guidelines

- Use Rust 2024 edition conventions
- Format with `rustfmt` (default settings)
- Use descriptive variable and function names in snake_case
- Prefer Result<T, E> over unwrap()/expect() for error handling
- Organize imports alphabetically with std first, then external crates
- Use structured logging via env_logger and tracing with stderr output for console visibility
- Add type annotations for public functions/methods
- Match arms should be aligned
- Use Rust's ownership system effectively (avoid unnecessary clones)
- Actively use cargo-docs (mcp) to investigate crate usage patterns
- Organize module and import declarations in the following order, with each block separated by a blank line:
  1. `pub mod` declarations (no line breaks within this block)
  2. `mod` declarations (no line breaks within this block)
  3. `pub use` declarations (no line breaks within this block)
  4. `use` declarations (no line breaks within this block)

Example of proper module and import organization:

```rust
pub mod git_repository;
pub mod params;

mod aaa;

pub use git_repository::*;
pub use params::*;

use lumin::{search, search::SearchOptions};
use reqwest::Client;
```

## MCP Tool Guidelines

When working with MCP tool functions, always enhance tool descriptions and provide detailed usage examples without being explicitly asked:

### Tool Description Annotations

For the `#[tool(description = "...")]` annotation:

- Write 2-3 sentences explaining the tool's purpose
- Include what the tool returns (format, structure)
- Explain when an AI agent should use this tool vs. other similar tools
- Add 2-4 complete JSON call examples showing different parameter combinations
- Format examples using code blocks with proper JSON syntax:
  ```
  `{"name": "tool_name", "arguments": {"param1": "value1", "param2": "value2"}}`
  ```

Example of good tool description:

```rust
#[tool(description = "Search for Rust crates on crates.io (returns JSON or markdown). This tool helps you discover relevant Rust libraries by searching the official registry. Use this when you need to find crates for specific functionality or alternatives to known crates. Example usage: `{\"name\": \"search_crates\", \"arguments\": {\"query\": \"http client\"}}`. With limit: `{\"name\": \"search_crates\", \"arguments\": {\"query\": \"json serialization\", \"limit\": 20}}`. For specific features: `{\"name\": \"search_crates\", \"arguments\": {\"query\": \"async database\", \"limit\": 5}}`")]
```

### Parameter Description Annotations

For the `#[schemars(description = "...")]` annotation:

- Explain each parameter's purpose in detail (1-2 sentences)
- Specify expected format and valid values
- Include constraints and validation rules
- Provide example values showing format variations
- For optional parameters, explain default behavior when omitted

Example of good parameter description:

```rust
#[schemars(description = "The name of the crate to look up. Must be the exact crate name as published on crates.io (e.g., 'serde', 'tokio', 'reqwest'). This parameter is case-sensitive and must match exactly how the crate is published. For standard library types, use 'std' as the crate name.")]
```

### Server Instructions

In the ServerHandler's get_info() implementation:

- Organize instructions with clear markdown headings
- Include a concise tool overview section
- Provide JSON examples for each tool with proper formatting
- Show tool combinations for common use cases
- Include a troubleshooting section for common errors

## Development Guidelines

### Documentation Terminology

In this project, the term "documentation" or "project documentation" refers to the following:

- Source code comments and documentation strings: Content reflected in rustdoc and similar documentation generators
- CLAUDE.md (this file): Guidelines and rules for AI agents working with this repository
- spec.md: Detailed specifications and technical documentation for developers and AI agents
- devlog.md: Development history documentation for AI agents who will develop the code in the future
- README.md: Documentation for users of this library or application

When asked to "update documentation", "add to the documentation", or "edit the documentation", you should:

1. Update all relevant markdown documentation files (README.md, spec.md, devlog.md)
2. Update Rustdoc comments in the source code when applicable
3. Ensure consistency across all documentation sources
4. Follow Rust documentation best practices (///, //! format for Rustdoc)

IMPORTANT: When a user asks to document something, always include updates to both the markdown files AND Rustdoc comments in the code itself. This dual-documentation approach ensures that information is available both to users reading the documentation files and to developers examining the code directly.

### Generating Process

You should think and output in English

### Making Changes

When making significant changes to the codebase:

1. Follow the code style guidelines
2. Verify code compiles with `cargo check` to catch basic compilation errors quickly
3. Ensure all tests pass with `cargo test`
4. Run linting with `cargo clippy` 
5. Format code with `cargo fmt`
6. **Update Rustdoc comments in source code:**
   - Add or update module-level documentation (//!)
   - Add or update item-level documentation (///)
   - Verify documentation compiles with `cargo doc --no-deps`
7. **Document your changes in devlog.md**
8. **Document your changes in spec.md**

Remember that documentation is part of the codebase and should be held to the same quality standards as the code itself. When updating documentation:
- Ensure Rustdoc comments compile without warnings
- Make sure examples in documentation are correct and up-to-date
- Keep code and documentation in sync

### Documenting in devlog.md

After implementing significant changes:

1. Review your changes to understand what should be documented
2. Edit devlog.md to update relevant sections
3. Add a new entry in the "Recent Changes" section with today's date and a summary of changes
4. Follow the instructions at the top of devlog.md for proper documentation format

When asked to "update devlog.md", proceed directly to editing the file following the guidelines contained within it. This ensures that design decisions and implementation details are properly documented for future reference.

IMPORTANT: Always update devlog.md after making significant changes to the codebase, especially when:

- Adding new features or modules
- Refactoring existing code
- Making architectural changes
- Implementing new functionality

The devlog update should include:

- A summary of what was changed or added
- Any design decisions that were made
- Implementation challenges and solutions
- Tests that were added or modified

IMPORTANT: When interpreting devlog.md to analyze code changes, be aware that the file may contain only changes made by AI agents. It may not include changes made directly by human programmers. This can lead to discrepancies between the current source code and what is documented in devlog.md. When creating a new devlog.md file, include a note stating that "This devlog contains only changes made by AI agents and may not include modifications made directly by human programmers. There may be discrepancies between the current source code and the history documented here."
