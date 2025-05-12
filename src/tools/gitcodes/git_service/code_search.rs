use lumin::{search, search::SearchOptions};
use std::path::Path;

/// Performs a code search on a prepared repository
///
/// This function executes the search using the lumin search library
/// and processes the results.
///
/// # Parameters
///
/// * `repo_dir` - The directory containing the repository
/// * `pattern` - The search pattern to look for
/// * `case_sensitive` - Whether the search should be case-sensitive (default: false)
/// * `_use_regex` - Whether to use regex for the search (not currently implemented)
/// * `_file_extensions` - Filter by file extensions (not currently implemented)
pub async fn perform_code_search(
    repo_dir: &Path,
    pattern: &str,
    case_sensitive: Option<bool>,
    _use_regex: Option<bool>,
    _file_extensions: Option<Vec<String>>,
) -> Result<String, String> {
    // Create search options
    let search_options = SearchOptions {
        case_sensitive: case_sensitive.unwrap_or(false),
        ..SearchOptions::default()
    };

    // Execute the search
    match search::search_files(
        pattern,
        repo_dir,
        &search_options,
    ) {
        Ok(results) => {
            // Format results
            let mut output = String::new();

            for result in results {
                output.push_str(&format!(
                    "{}:{}: {}\n",
                    result.file_path.display(),
                    result.line_number,
                    result.line_content
                ));
            }

            Ok(output)
        }
        Err(e) => Err(format!("Lumin search failed: {}", e)),
    }
}
