# Glob Pattern Guide for File Filtering

This document provides detailed information about using glob patterns with the `include_globs` and `exclude_dirs` parameters in the `grep_repository` tool. Glob patterns allow for powerful and flexible file filtering beyond simple extension matching.

## Important: Relative Path Requirements

With lumin 0.1.16, both `include_globs` and `exclude_dirs` parameters expect **relative paths** from the repository root:

- Leading slashes (`/`) are automatically stripped from patterns
- Patterns must be relative to the search directory
- Absolute paths will be converted to relative paths automatically

## Glob Pattern Syntax

| Pattern | Description |
|---------|-------------|
| `*` | Matches any number of characters within a single directory |
| `**` | Matches any number of directories recursively |
| `?` | Matches exactly one character |
| `[abc]` | Matches any character in the brackets |
| `[a-z]` | Matches any character in the range |
| `{a,b}` | Matches any of the comma-separated patterns |
| `!pattern` | Excludes files matching the pattern (in some implementations) |

## Common Glob Pattern Examples for include_globs

| Pattern | Description |
|---------|-------------|
| `**/*.rs` | All Rust (`.rs`) files anywhere in the repository |
| `src/**/*.rs` | Rust files in the src directory (including subdirectories) |
| `src/*.rs` | Rust files in src directory (excluding subdirectories) |
| `**/*.{rs,toml}` | All Rust and TOML files in one pattern |
| `**/test_*.rs` | All Rust files with names starting with "test_" |
| `**/*_test.rs` | All Rust files with names ending with "_test" |

## exclude_dirs Parameter

The `exclude_dirs` parameter accepts two formats:

| Input | Result | Description |
|-------|--------|-------------|
| `["target", "node_modules"]` | `["**/target/**", "**/node_modules/**"]` | Directory names (converted to glob patterns) |
| `["**/target/**", "src/**/*.tmp"]` | `["**/target/**", "src/**/*.tmp"]` | Direct glob patterns |
| `["/absolute/path"]` | `["absolute/path"]` | Leading slashes automatically stripped |

## Best Practices

1. **Always use relative paths**
   - Patterns are relative to repository root (leading slashes stripped automatically)
   - `src/**/*.rs` not `/src/**/*.rs`

2. **Always prefix with `**/` for repository-wide searches**
   - `**/*.rs` instead of `*.rs` to find files at any depth

3. **Use `**/` suffix for directory patterns**
   - `**/target/**` instead of `target` to match directories at any level

4. **For exclude_dirs, choose the appropriate format**
   - Simple directory names: `["target", "node_modules"]` (automatically converted)
   - Complex patterns: `["**/target/**", "src/**/*.tmp"]` (used directly)

5. **Combine multiple patterns for complex filtering**
   - `["**/*.rs", "**/*.toml"]` to match multiple file types

6. **Place more specific patterns first**
   - More specific patterns should come before general patterns

## Examples in JSON Format

```json
{
  "name": "grep_repository",
  "arguments": {
    "repository_location": "github:user/repo",
    "pattern": "search term",
    "include_globs": ["**/*.rs", "**/*.toml"],
    "exclude_dirs": ["target", "node_modules"]
  }
}
```

## Performance Considerations

- Use simpler patterns when possible for better performance
- Avoid excessive use of `**` patterns which can be computationally expensive
- Balance between specificity and simplicity

## Limitations

- Very complex patterns with multiple wildcards may impact performance
- Patterns must be relative paths (absolute paths are automatically converted)
- Both `include_globs` and `exclude_dirs` expect the same relative path format in lumin 0.1.16
- Patterns with special characters may need proper escaping in JSON

## Migration from file_extensions

If you previously used the `file_extensions` parameter, here's how to migrate to `include_globs`:

| Old | New |
|-----|-----|
| `"file_extensions": ["rs", "toml"]` | `"include_globs": ["**/*.rs", "**/*.toml"]` |
| `"file_extensions": ["rs"]` | `"include_globs": ["**/*.rs"]` |
| `"file_extensions": ["js", "ts", "jsx", "tsx"]` | `"include_globs": ["**/*.{js,ts,jsx,tsx}"]` |