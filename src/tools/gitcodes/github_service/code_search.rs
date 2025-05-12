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
    // Clone values for the thread
    let repo_dir_clone = repo_dir.to_path_buf();
    let pattern_clone = pattern.to_string();

    // Execute search in a blocking task
    tokio::task::spawn_blocking(move || {
        // Create search options
        let search_options = SearchOptions {
            case_sensitive: case_sensitive.unwrap_or(false),
            ..SearchOptions::default()
        };

        // Execute the search
        match search::search_files(
            &pattern_clone,
            &repo_dir_clone,
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
    })
    .await
    .map_err(|e| format!("Search task failed: {}", e))?
}

/// Formats the search results for output
///
/// This function takes the raw search results and formats them into
/// a user-friendly message.
///
/// # Parameters
///
/// * `search_result` - The raw search result to format
/// * `pattern` - The search pattern that was used
/// * `repository` - The repository URL or local file path that was searched
//TODO(tacogips) should return Result<String,String>
pub fn format_search_results(
    search_result: &Result<String, String>,
    pattern: &str,
    repository: &str,
) -> String {
    // Determine if the repository is a local path or a GitHub URL
    let is_local_path = Path::new(repository).exists();
    let location_type = if is_local_path { "local directory" } else { "repository" };
    
    match search_result {
        Ok(search_output) => {
            if search_output.trim().is_empty() {
                format!(
                    "No matches found for pattern '{}' in {} {}",
                    pattern, location_type, repository
                )
            } else {
                format!(
                    "Search results for '{}' in {} {}:\n\n{}",
                    pattern, location_type, repository, search_output
                )
            }
        }
        Err(e) => format!("Search failed: {}", e),
    }
}
