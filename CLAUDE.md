# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Documentation

When working with this codebase, refer to the following key documents:

- **CLAUDE.md** (this file): Primary instructions for working with the codebase
- **spec.md**: Library specifications and implementation details
- **devlog.md**: Development log with design decisions, implementation details, and roadmap

For library-specific specifications and implementation details, refer to spec.md.
To understand the history, architecture decisions, and implementation details of this project, always refer to the devlog.md file.
When making significant changes, update devlog.md to document your work by following the instructions at the top of that file.

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

## Development Guidelines

### Documentation Terminology

In this project, the term "documentation" or "project documentation" refers to the following:

- Source code comments and documentation strings
- CLAUDE.md (this file)
- spec.md
- devlog.md

When asked to update "documentation", you should check and update all of these documentation sources for consistency.

### Rule of Thumb

You should think and output in English

### Making Changes

When making significant changes to the codebase:

1. Follow the code style guidelines
2. Ensure all tests pass with `cargo test`
3. Run linting with `cargo clippy`
4. Format code with `cargo fmt`
5. **Document your changes in devlog.md**

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
- Details of errors encountered during development and their solutions
- Explanations for dependency changes (additions/removals) in Cargo.toml
- Rationale for major architectural decisions

In all cases, you may omit trivial or minor implementation details to keep the devlog focused on significant aspects of the development process.
