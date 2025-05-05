use super::{OrderOption, SearchParams, SortOption};
use reqwest::Client;

/// Constructs the GitHub API URL for repository search
///
/// Builds the complete URL with query parameters for the GitHub search API.
/// This method handles parameter defaults, validation, and proper URL encoding.
///
/// # Returns
///
/// A fully formed URL string ready for HTTP request to GitHub's search API
///
/// # Parameter Handling
///
/// - `sort_by`: Uses SortOption::Relevance if None (empty string in the URL)
/// - `order`: Uses OrderOption::Descending if None ("desc" in the URL)
/// - `per_page`: Uses 30 if None, caps at 100 (GitHub API limit)
/// - `page`: Uses 1 if None
/// - `query`: URL encoded to handle special characters
///
/// # Examples
///
/// ```
/// use gitcodes_mcp::tools::gitcodes::github_service::params::{SearchParams, SortOption, OrderOption};
///
/// let params = SearchParams {
///     query: "rust web framework".to_string(),
///     sort_by: Some(SortOption::Stars),
///     order: Some(OrderOption::Descending),
///     per_page: Some(50),
///     page: Some(1),
/// };
///
/// let url = params.construct_search_url();
/// // Result: "https://api.github.com/search/repositories?q=rust%20web%20framework&sort=stars&order=desc&per_page=50&page=1"
/// ```
fn construct_search_url(param: &SearchParams) -> String {
    // Set up sort parameter using Default implementation
    let default_sort = SortOption::default();
    let sort = param.sort_by.as_ref().unwrap_or(&default_sort).to_str();

    // Set up order parameter using Default implementation
    let default_order = OrderOption::default();
    let order = param.order.as_ref().unwrap_or(&default_order).to_str();

    // Set default values for pagination
    let per_page = param.per_page.unwrap_or(30).min(100); // GitHub API limit is 100
    let page = param.page.unwrap_or(1);

    let mut url = format!(
        "https://api.github.com/search/repositories?q={}",
        urlencoding::encode(&param.query)
    );

    if !sort.is_empty() {
        url.push_str(&format!("&sort={}", sort));
    }

    url.push_str(&format!("&order={}", order));
    url.push_str(&format!("&per_page={}&page={}", per_page, page));

    url
}

/// Executes a GitHub API search request
///
/// Sends the HTTP request to the GitHub API and handles the response.
pub async fn execute_search_request(
    param: &SearchParams,
    client: &Client,
    github_token: Option<&String>,
) -> String {
    //TODO(tacogips) this method should return anyhow::Result<String> instead of Strin
    let url = construct_search_url(param);
    // Set up the API request
    let mut req_builder = client.get(url).header(
        "User-Agent",
        "gitcodes-mcp/0.1.0 (https://github.com/d6e/gitcodes-mcp)",
    );

    // Add authentication token if available
    if let Some(token) = &github_token {
        req_builder = req_builder.header("Authorization", format!("token {}", token));
    }

    // Execute API request
    let response = match req_builder.send().await {
        Ok(resp) => resp,
        Err(e) => return format!("Failed to search repositories: {}", e),
    };

    // Check if the request was successful
    let status = response.status();
    if !status.is_success() {
        let error_text = match response.text().await {
            Ok(text) => text,
            Err(_) => "Unknown error".to_string(),
        };

        return format!("GitHub API error {}: {}", status, error_text);
    }

    // Return the raw JSON response
    match response.text().await {
        Ok(text) => text,
        Err(e) => format!("Failed to read response body: {}", e),
    }
}
