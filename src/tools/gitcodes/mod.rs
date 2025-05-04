mod git_repository;

use lumin::{search, search::SearchOptions};
use rand::Rng;
use reqwest::Client;

use rmcp::{model::*, schemars, tool, ServerHandler};

// Repository manager for clone operations
#[derive(Clone)]
pub struct RepositoryManager {
    temp_dir_base: String,
}

impl Default for RepositoryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RepositoryManager {
    pub fn new() -> Self {
        let system_temp = std::env::temp_dir().to_string_lossy().to_string();
        Self {
            temp_dir_base: system_temp,
        }
    }

    // Parse repository URL to extract user and repo name
    pub fn parse_repository_url(&self, url: &str) -> Result<(String, String), String> {
        let user_repo = if url.starts_with("https://github.com/") {
            url.trim_start_matches("https://github.com/")
                .trim_end_matches(".git")
                .to_string()
        } else if url.starts_with("git@github.com:") {
            url.trim_start_matches("git@github.com:")
                .trim_end_matches(".git")
                .to_string()
        } else if url.starts_with("github:") {
            url.trim_start_matches("github:").to_string()
        } else {
            return Err("Invalid GitHub repository URL format".to_string());
        };

        let parts: Vec<&str> = user_repo.split('/').collect();
        if parts.len() != 2 {
            return Err("Invalid GitHub repository URL format".to_string());
        }

        Ok((parts[0].to_string(), parts[1].to_string()))
    }

    // Generate a unique directory name for the repository
    pub fn get_repo_dir(&self, user: &str, repo: &str) -> String {
        format!(
            "{}/mcp_github_{}_{}_{}",
            self.temp_dir_base,
            user,
            repo,
            rand::thread_rng().gen::<u32>() % 10000
        )
    }

    // Check if repository is already cloned
    pub async fn is_repo_cloned(&self, dir: &str) -> bool {
        tokio::fs::metadata(dir).await.is_ok()
    }
}

#[derive(Clone)]
pub struct CargoDocRouter {
    pub client: Client,
    pub repo_manager: RepositoryManager,
    pub github_token: Option<String>,
}

impl Default for CargoDocRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[tool(tool_box)]
impl CargoDocRouter {
    pub fn new() -> Self {
        let github_token = std::env::var("GITCODE_MCP_GITHUB_TOKEN").ok();

        Self {
            client: Client::new(),
            repo_manager: RepositoryManager::new(),
            github_token,
        }
    }

    // GitHub Repository Search Tool
    #[tool(description = "Search for GitHub repositories")]
    async fn search_repositories(
        &self,
        #[tool(param)]
        #[schemars(description = "Search query (required)")]
        query: String,

        #[tool(param)]
        #[schemars(description = "How to sort results (optional, default is 'relevance')")]
        sort_by: Option<SortOption>,

        #[tool(param)]
        #[schemars(description = "Sort order (optional, default is 'descending')")]
        order: Option<OrderOption>,

        #[tool(param)]
        #[schemars(description = "Results per page (optional, default is 30, max 100)")]
        per_page: Option<u8>,

        #[tool(param)]
        #[schemars(description = "Result page number (optional, default is 1)")]
        page: Option<u32>,
    ) -> String {
        // Set up request parameters
        let sort = match sort_by {
            Some(SortOption::Stars) => "stars",
            Some(SortOption::Forks) => "forks",
            Some(SortOption::Updated) => "updated",
            None => "", // Default is relevance
        };
        let order_param = match order {
            Some(OrderOption::Ascending) => "asc",
            Some(OrderOption::Descending) => "desc",
            None => "desc", // Default is descending
        };
        // Ensure per_page is within limits
        let per_page = per_page.unwrap_or(30).min(100);
        let page = page.unwrap_or(1);

        // Construct the API URL
        let mut url = format!(
            "https://api.github.com/search/repositories?q={}",
            urlencoding::encode(&query)
        );

        if !sort.is_empty() {
            url.push_str(&format!("&sort={}", sort));
        }

        url.push_str(&format!("&order={}", order_param));
        url.push_str(&format!("&per_page={}&page={}", per_page, page));

        // Set up the API request
        let mut req_builder = self.client.get(&url).header(
            "User-Agent",
            "gitcodes-mcp/0.1.0 (https://github.com/d6e/gitcodes-mcp)",
        );

        // Add authentication token if available
        if let Some(token) = &self.github_token {
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

    // GitHub Repository Code Grep Tool
    #[tool(description = "Search code in a GitHub repository")]
    async fn grep_repository(
        &self,
        #[tool(param)]
        #[schemars(description = "Repository URL (required) - supports GitHub formats")]
        repository: String,

        #[tool(param)]
        #[schemars(description = "Branch or tag (optional, default is main or master)")]
        ref_name: Option<String>,

        #[tool(param)]
        #[schemars(description = "Search pattern (required)")]
        pattern: String,

        #[tool(param)]
        #[schemars(description = "Whether to be case-sensitive (optional, default is false)")]
        case_sensitive: Option<bool>,

        #[tool(param)]
        #[schemars(description = "Whether to use regex (optional, default is true)")]
        use_regex: Option<bool>,

        #[tool(param)]
        #[schemars(description = "File extensions to search (optional, e.g., [\"rs\", \"toml\"])")]
        file_extensions: Option<Vec<String>>,

        #[tool(param)]
        #[schemars(
            description = "Directories to exclude from search (optional, e.g., [\"target\", \"node_modules\"])"
        )]
        _exclude_dirs: Option<Vec<String>>,
    ) -> String {
        // Parse repository URL
        let (user, repo) = match self.repo_manager.parse_repository_url(&repository) {
            Ok(result) => result,
            Err(e) => return format!("Error: {}", e),
        };

        // Default branch if not specified
        let ref_name = ref_name.unwrap_or_else(|| "main".to_string());

        // Get a temporary directory for the repository
        let repo_dir = self.repo_manager.get_repo_dir(&user, &repo);

        // Check if repo is already cloned
        let is_cloned = self.repo_manager.is_repo_cloned(&repo_dir).await;

        // If repo is not cloned, clone it
        if !is_cloned {
            let result = self
                .clone_repository(&repo_dir, &user, &repo, &ref_name)
                .await;
            if let Err(e) = result {
                return e;
            }
        } else {
            let result = self.update_repository(&repo_dir, &ref_name).await;
            if let Err(e) = result {
                return e;
            }
        }

        // Use lumin for search
        let repo_dir_clone = repo_dir.clone();
        let pattern_clone = pattern.clone();
        let search_result = tokio::task::spawn_blocking(move || {
            // Create search options
            let mut search_options = SearchOptions::default();

            // Configure case sensitivity
            search_options.case_sensitive = case_sensitive.unwrap_or(false);

            // Execute the search
            match search::search_files(&pattern_clone, &repo_dir_clone, &search_options) {
                Ok(result) => {
                    // Format results
                    let mut output = String::new();

                    for file_match in result.matches {
                        for line_match in file_match.line_matches {
                            output.push_str(&format!(
                                "{}:{}: {}\n",
                                file_match.path.display(),
                                line_match.line_number,
                                line_match.line
                            ));
                        }
                    }

                    output
                }
                Err(e) => format!("Lumin search failed: {}", e),
            }
        })
        .await
        .map_err(|e| format!("Search task failed: {}", e));

        // Handle search errors
        if let Err(e) = &search_result {
            return format!("Search failed: {}", e);
        }

        let search_output = search_result.unwrap();
        if search_output.trim().is_empty() {
            format!(
                "No matches found for pattern '{}' in repository {}",
                pattern, repository
            )
        } else {
            format!(
                "Search results for '{}' in repository {}:\n\n{}",
                pattern, repository, search_output
            )
        }
    }

    // Function to fetch repository refs (branches and tags)
    async fn fetch_repository_refs(
        &self,
        repo_dir: &str,
        user: &str,
        repo: &str,
    ) -> Result<String, String> {
        // Get branches and tags
        let repo_dir_clone = repo_dir.to_string();
        let user_clone = user.to_string();
        let repo_clone = repo.to_string();

        // Change to the repository directory
        let current_dir = match std::env::current_dir() {
            Ok(dir) => dir,
            Err(e) => return Err(format!("Failed to get current directory: {}", e)),
        };

        if let Err(e) = std::env::set_current_dir(&repo_dir_clone) {
            return Err(format!("Failed to change directory: {}", e));
        }

        // First run git fetch to make sure we have all refs
        let fetch_status = std::process::Command::new("git")
            .args(["fetch", "--all"])
            .status();

        if let Err(e) = fetch_status {
            let _ = std::env::set_current_dir(current_dir);
            return Err(format!("Git fetch failed: {}", e));
        }

        if !fetch_status.unwrap().success() {
            let _ = std::env::set_current_dir(current_dir);
            return Err("Git fetch failed".to_string());
        }

        // Get branches
        let branches_output = std::process::Command::new("git")
            .args(["branch", "-r"])
            .output();

        let branches_output = match branches_output {
            Ok(output) => output,
            Err(e) => {
                let _ = std::env::set_current_dir(current_dir);
                return Err(format!("Failed to list branches: {}", e));
            }
        };

        let branches_str = String::from_utf8_lossy(&branches_output.stdout).to_string();

        // Get tags
        let tags_output = std::process::Command::new("git").args(["tag"]).output();

        let tags_output = match tags_output {
            Ok(output) => output,
            Err(e) => {
                let _ = std::env::set_current_dir(current_dir);
                return Err(format!("Failed to list tags: {}", e));
            }
        };

        let tags_str = String::from_utf8_lossy(&tags_output.stdout).to_string();

        // Change back to the original directory
        if let Err(e) = std::env::set_current_dir(current_dir) {
            return Err(format!("Failed to restore directory: {}", e));
        }

        // Format the output
        let mut result = String::new();
        result.push_str(&format!(
            "Repository: {}/{}

",
            user_clone, repo_clone
        ));

        // Extract and format branches
        let branches: Vec<String> = branches_str
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.starts_with("origin/") && !line.contains("HEAD") {
                    Some(line.trim_start_matches("origin/").to_string())
                } else {
                    None
                }
            })
            .collect();

        // Extract and format tags
        let tags: Vec<String> = tags_str
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();

        // Add branches section
        result.push_str(
            "## Branches
",
        );
        if branches.is_empty() {
            result.push_str(
                "No branches found
",
            );
        } else {
            for branch in branches {
                result.push_str(&format!("- {}\n", branch));
            }
        }

        // Add tags section
        result.push_str(
            "
## Tags
",
        );
        if tags.is_empty() {
            result.push_str(
                "No tags found
",
            );
        } else {
            for tag in tags {
                result.push_str(&format!("- {}\n", tag));
            }
        }

        Ok(result)
    }

    // GitHub Repository Branches/Tags List Tool
    #[tool(description = "List branches and tags for a GitHub repository")]
    async fn list_repository_refs(
        &self,
        #[tool(param)]
        #[schemars(description = "Repository URL (required) - supports GitHub formats")]
        repository: String,
    ) -> String {
        // Parse repository URL
        let (user, repo) = match self.repo_manager.parse_repository_url(&repository) {
            Ok(result) => result,
            Err(e) => return format!("Error: {}", e),
        };

        // Get a temporary directory for the repository
        let repo_dir = self.repo_manager.get_repo_dir(&user, &repo);

        // Check if repo is already cloned
        let is_cloned = self.repo_manager.is_repo_cloned(&repo_dir).await;

        // If repo is not cloned, clone it
        if !is_cloned {
            match self.clone_repository(&repo_dir, &user, &repo, "main").await {
                Ok(_) => {}
                Err(e) => return e,
            }
        }

        // Fetch repository refs using the extracted function
        match self.fetch_repository_refs(&repo_dir, &user, &repo).await {
            Ok(result) => result,
            Err(e) => format!("Failed to list refs: {}", e),
        }
    }

    ////
    // Look up documentation for a Rust crate
    //    #[tool(description = "Look up documentation for a Rust crate")]
    //    async fn lookup_crate(
    //        &self,
    //        #[tool(param)]
    //        #[schemars(description = "The name of the crate to look up")]
    //        crate_name: String,
    //
    //        #[tool(param)]
    //        #[schemars(description = "The version of the crate (optional, defaults to latest)")]
    //        version: Option<String>,
    //    ) -> String {
    //        // Check cache first
    //        let cache_key = if let Some(ver) = &version {
    //            format!("{}}:{}", crate_name, ver)
    //        } else {
    //            crate_name.clone()
    //        };
    //
    //        // Construct the docs.rs URL for the crate
    //        let url = if let Some(ver) = version {
    //            format!("https://docs.rs/crate/{}/{}/", crate_name, ver)
    //        } else {
    //            format!("https://docs.rs/crate/{}/", crate_name)
    //        };
    //
    //        // Fetch the documentation page
    //        let response = match self
    //            .client
    //            .get(&url)
    //            .header(
    //                "User-Agent",
    //                "gitcodes/0.1.0 (https://github.com/d6e/gitcodes-mcp)",
    //            )
    //            .send()
    //            .await
    //        {
    //            Ok(resp) => resp,
    //            Err(e) => return format!("Failed to fetch documentation: {}", e),
    //        };
    //
    //        if !response.status().is_success() {
    //            return format!(
    //                "Failed to fetch documentation. Status: {}",
    //                response.status()
    //            );
    //        }
    //
    //        let html_body = match response.text().await {
    //            Ok(body) => body,
    //            Err(e) => return format!("Failed to read response body: {}", e),
    //        };
    //
    //        // Convert HTML to markdown
    //        let markdown_body = parse_html(&html_body);
    //
    //        // Cache the markdown result
    //        self.cache.set(cache_key, markdown_body.clone()).await;
    //
    //        markdown_body
    //    }

    // Clone repository function
    async fn clone_repository(
        &self,
        repo_dir: &str,
        user: &str,
        repo: &str,
        ref_name: &str,
    ) -> Result<(), String> {
        // Create directory if it doesn't exist
        if let Err(e) = tokio::fs::create_dir_all(repo_dir).await {
            return Err(format!("Failed to create directory: {}", e));
        }

        // Clone repository
        let clone_url = format!("https://github.com/{}/{}.git", user, repo);

        // Clone with git command
        let repo_dir_clone = repo_dir.to_string();
        let ref_name_clone = ref_name.to_string();
        let clone_result = tokio::task::spawn_blocking(move || {
            let status = std::process::Command::new("git")
                .args([
                    "clone",
                    "--depth=1",
                    "--branch",
                    &ref_name_clone,
                    &clone_url,
                    &repo_dir_clone,
                ])
                .status();

            match status {
                Ok(exit_status) if exit_status.success() => Ok(()),
                Ok(exit_status) => Err(format!("Git clone failed with status: {}", exit_status)),
                Err(e) => Err(format!("Failed to execute git clone: {}", e)),
            }
        })
        .await;

        // Handle errors during cloning
        if let Err(e) = clone_result {
            return Err(format!("Failed to run git clone: {}", e));
        }

        clone_result.unwrap()
    }

    // Update repository function
    async fn update_repository(&self, repo_dir: &str, ref_name: &str) -> Result<(), String> {
        // Repository exists, update it
        let repo_dir_clone = repo_dir.to_string();
        let ref_name_clone = ref_name.to_string();
        let update_result = tokio::task::spawn_blocking(move || {
            // Change to the repository directory
            let current_dir = match std::env::current_dir() {
                Ok(dir) => dir,
                Err(e) => return Err(format!("Failed to get current directory: {}", e)),
            };

            if let Err(e) = std::env::set_current_dir(&repo_dir_clone) {
                return Err(format!("Failed to change directory: {}", e));
            }

            // Fetch updates
            let fetch_status = std::process::Command::new("git")
                .args(["fetch", "--depth=1", "origin"])
                .status();

            if let Err(e) = fetch_status {
                let _ = std::env::set_current_dir(current_dir);
                return Err(format!("Git fetch failed: {}", e));
            }

            if !fetch_status.unwrap().success() {
                let _ = std::env::set_current_dir(current_dir);
                return Err("Git fetch failed".to_string());
            }

            // Try to checkout the requested branch
            let checkout_status = std::process::Command::new("git")
                .args(["checkout", &ref_name_clone])
                .status();

            if let Err(e) = checkout_status {
                let _ = std::env::set_current_dir(current_dir);
                return Err(format!("Git checkout failed: {}", e));
            }

            if !checkout_status.unwrap().success() {
                // Try origin/branch_name
                let origin_checkout = std::process::Command::new("git")
                    .args(["checkout", &format!("origin/{}", ref_name_clone)])
                    .status();

                if let Err(e) = origin_checkout {
                    let _ = std::env::set_current_dir(current_dir);
                    return Err(format!("Git checkout failed: {}", e));
                }

                if !origin_checkout.unwrap().success() {
                    let _ = std::env::set_current_dir(current_dir);
                    return Err(format!("Branch/tag not found: {}", ref_name_clone));
                }
            }

            // Change back to the original directory
            if let Err(e) = std::env::set_current_dir(current_dir) {
                return Err(format!("Failed to restore directory: {}", e));
            }

            Ok(())
        })
        .await;

        // Handle update errors
        if let Err(e) = update_result {
            return Err(format!("Failed to update repository: {}", e));
        }

        update_result.unwrap()
    }
}

#[tool(tool_box)]
impl ServerHandler for CargoDocRouter {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "GitHub and Rust Documentation MCP Server for accessing repository information and Rust crate documentation.".to_string(),
            ),
        }
    }
}

// Define the SortOption enum for GitHub repository sorting
#[derive(Debug, schemars::JsonSchema, serde::Serialize, serde::Deserialize)]
pub enum SortOption {
    Stars,
    Forks,
    Updated,
}

// Define the OrderOption enum for GitHub repository sort order
#[derive(Debug, schemars::JsonSchema, serde::Serialize, serde::Deserialize)]
pub enum OrderOption {
    Ascending,
    Descending,
}
