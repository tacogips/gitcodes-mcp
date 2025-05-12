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

### Quieter Cargo Commands

When executing Cargo commands to avoid sending excessive output to LLMs, use these environment variables:

```bash
CARGO_TERM_QUIET=true cargo build   # Reduces Cargo's own output
CARGO_TERM_QUIET=true cargo check    # Suppresses progress output
CARGO_TERM_QUIET=true cargo test     # For basic test runs

# For nextest users (better test output control)
CARGO_TERM_QUIET=true NEXTEST_STATUS_LEVEL=fail NEXTEST_FAILURE_OUTPUT=immediate_final NEXTEST_HIDE_PROGRESS_BAR=1 cargo nextest run
```

These environment variables do the following:
- `CARGO_TERM_QUIET=true`: Suppresses Cargo's own progress output
- `NEXTEST_STATUS_LEVEL=fail`: Only show status output for failed tests
- `NEXTEST_FAILURE_OUTPUT=immediate_final`: Shows failed test output both immediately and in the final summary
- `NEXTEST_HIDE_PROGRESS_BAR=1`: Disables the nextest progress bar completely

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

- All `use` and `mod` declarations must be placed at the beginning of the Rust file (or package or module). If the file begins with Rustdoc comments, the declarations should immediately follow those comments.

- Place struct/enum definitions and their implementations together. A struct/enum declaration should be immediately followed by its implementations, rather than interspersing other type definitions between them.

Example of proper module and import organization:

```rust
// Example with rustdoc at the top
//! Module documentation comment
//! Additional documentation

pub mod git_repository;
pub mod params;

mod aaa;

pub use git_repository::*;
pub use params::*;

use lumin::{search, search::SearchOptions};
use reqwest::Client;

// Rest of the code follows...
```

Example of proper struct/enum organization (keeping implementations with their definitions):

```rust
// SearchParams and its implementation are kept together
pub struct SearchParams {
    pub query: String,
}

impl SearchParams {
    pub fn construct_search_url(&self) -> String {
        String::new()
    }
}

// GrepParams (no implementations needed)
pub struct GrepParams {
    pub exclude_dirs: Option<Vec<String>>,
}

// SortOption and all its implementations are kept together
pub enum SortOption {
    Updated,
    Relevance,
}

impl SortOption {
    pub fn to_str(&self) -> &str {
        match self {
            SortOption::Updated => "updated",
            SortOption::Relevance => "relevance",
        }
    }
}

impl Default for SortOption {
    fn default() -> Self {
        SortOption::Relevance
    }
}

// OrderOption and all its implementations are kept together
pub enum OrderOption {
    Ascending,
    Descending,
}

impl OrderOption {
    pub fn to_str(&self) -> &str {
        match self {
            OrderOption::Ascending => "asc",
            OrderOption::Descending => "desc",
        }
    }
}

impl Default for OrderOption {
    fn default() -> Self {
        OrderOption::Descending
    }
}
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
3. Add a new entry in the appropriate section, focusing on patterns and design decisions rather than timestamps
4. Follow the instructions at the top of devlog.md for proper documentation format

When asked to "update devlog.md", proceed directly to editing the file following the guidelines contained within it. This ensures that design decisions and implementation details are properly documented for future reference.

IMPORTANT: Always update devlog.md after making significant changes to the codebase, especially when:

- Adding new features or modules
- Refactoring existing code
- Making architectural changes
- Implementing new functionality

### Maintaining a Concise and Effective devlog.md

The primary purpose of devlog.md is to guide future LLM models in code generation by documenting key design patterns and architectural decisions. When asked to make the devlog more compact or to optimize it:

1. **Remove timestamps and dates**:
   - Dates are not relevant for code generation patterns
   - Section headers should focus on the change content, not when it happened
   - Use pattern-oriented section names (e.g., "Type System Improvements" instead of "2024-05-12: Type System Improvements")

2. **Prioritize design patterns over implementation details**:
   - Emphasize reusable patterns that can guide future code generation
   - Include code examples that demonstrate the pattern
   - Explain the rationale behind architectural decisions
   - Show the proper way to implement similar patterns in future code

3. **Organize by pattern categories**:
   - Group related changes under architectural themes (e.g., "Type System Improvements", "Error Handling Patterns")
   - Use clear, descriptive section headers that identify the pattern category
   - Sort by importance and reusability rather than chronologically

4. **Include application guidance**:
   - For each pattern, explain when and where to apply it
   - Note any constraints or conditions for using the pattern
   - Mention potential future extensions of the pattern

5. **Omit trivial changes and fixes**:
   - Focus only on significant architectural and design decisions
   - Skip minor refactorings, typo fixes, and routine maintenance
   - Consolidate similar small changes into pattern-level descriptions

6. **Use illustrative code examples**:
   - Include small, focused code snippets that demonstrate the pattern
   - Comment the code examples to highlight key aspects
   - Show both "before" and "after" when appropriate

When asked to "make devlog.md more compact" or "optimize devlog.md for code generation," apply these guidelines to transform the document into a more effective guide for future code generation.

IMPORTANT: When interpreting devlog.md to analyze code changes, be aware that the file may contain only changes made by AI agents. It may not include changes made directly by human programmers. This can lead to discrepancies between the current source code and what is documented in devlog.md. When creating a new devlog.md file, include a note stating that "This devlog contains only changes made by AI agents and may not include modifications made directly by human programmers. There may be discrepancies between the current source code and the history documented here."
