# Glob Pattern Guide for File Filtering

This document provides detailed information about using glob patterns with the `include_globs` parameter in the `grep_repository` tool. Glob patterns allow for powerful and flexible file filtering beyond simple extension matching.

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

## Common Glob Pattern Examples

| Pattern | Description |
|---------|-------------|
| `**/*.rs` | All Rust (`.rs`) files anywhere in the repository |
| `src/**/*.rs` | Rust files in the src directory (including subdirectories) |
| `src/*.rs` | Rust files in src directory (excluding subdirectories) |
| `**/*.{rs,toml}` | All Rust and TOML files in one pattern |
| `**/test_*.rs` | All Rust files with names starting with "test_" |
| `**/*_test.rs` | All Rust files with names ending with "_test" |
| `!**/target/**` | Exclude all files in any target directory |

## Best Practices

1. **Always prefix with `**/` for repository-wide searches**
   - `**/*.rs` instead of `*.rs` to find files at any depth

2. **Use `**/` suffix for directory patterns**
   - `**/target/**` instead of `target` to match directories at any level

3. **Combine multiple patterns for complex filtering**
   - `["**/*.rs", "**/*.toml"]` to match multiple file types
   - `["src/**/*.rs", "!**/target/**"]` to match Rust files in src but exclude target directory

4. **Place more specific patterns first**
   - More specific patterns should come before general patterns

## Examples in JSON Format

```json
{
  "name": "grep_repository",
  "arguments": {
    "repository": "github:user/repo",
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
- Some advanced features may not be available in all implementations
- Patterns with special characters may need proper escaping in JSON

## Migration from file_extensions

If you previously used the `file_extensions` parameter, here's how to migrate to `include_globs`:

| Old | New |
|-----|-----|
| `"file_extensions": ["rs", "toml"]` | `"include_globs": ["**/*.rs", "**/*.toml"]` |
| `"file_extensions": ["rs"]` | `"include_globs": ["**/*.rs"]` |
| `"file_extensions": ["js", "ts", "jsx", "tsx"]` | `"include_globs": ["**/*.{js,ts,jsx,tsx}"]` |