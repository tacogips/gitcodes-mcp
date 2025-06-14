//! Octocrab-based GitHub client implementation
//!
//! This module provides a GitHub API client using the octocrab library,
//! replacing the custom HTTP implementation while maintaining the same interface.

use super::{GitRemoteRepositoryInfo, GithubIssueSearchParams, GithubSearchParams};
use crate::gitcodes::repository_manager::providers::*;
use octocrab::models::{issues::Issue as OctocrabIssue, Repository as OctocrabRepository};
use octocrab::{Octocrab, Page};

/// Octocrab-based GitHub client
#[derive(Debug, Clone)]
pub struct OctocrabGithubClient {
    client: Octocrab,
}

impl OctocrabGithubClient {
    /// Create a new OctocrabGithubClient
    pub fn new(github_token: Option<String>) -> Result<Self, String> {
        let client = if let Some(token) = github_token {
            Octocrab::builder()
                .personal_token(token)
                .build()
                .map_err(|e| format!("Failed to create octocrab client: {}", e))?
        } else {
            Octocrab::builder()
                .build()
                .map_err(|e| format!("Failed to create octocrab client: {}", e))?
        };

        Ok(Self { client })
    }

    /// Search repositories using octocrab
    pub async fn search_repositories(
        &self,
        params: GithubSearchParams,
    ) -> Result<RepositorySearchResults, String> {
        let query = &params.query;

        let mut search_builder = self.client.search().repositories(query);

        if let Some(sort) = params.sort_by {
            search_builder = search_builder.sort(sort.to_str());
        }

        if let Some(order) = params.order {
            search_builder = search_builder.order(order.to_str());
        }

        if let Some(per_page) = params.per_page {
            search_builder = search_builder.per_page(per_page);
        }

        if let Some(page) = params.page {
            search_builder = search_builder.page(page);
        }

        let results = search_builder
            .send()
            .await
            .map_err(|e| format!("Repository search failed: {}", e))?;

        Ok(Self::convert_repository_search_results(results))
    }

    /// Search issues and pull requests using octocrab
    pub async fn search_issues_and_pull_requests(
        &self,
        params: GithubIssueSearchParams,
    ) -> Result<IssueSearchResults, String> {
        let query = Self::build_issue_and_pull_request_search_query(&params)?;
        tracing::debug!("GitHub issue and pull request search query: {}", query);

        let mut search_builder = self.client.search().issues_and_pull_requests(&query);

        if let Some(sort) = params.sort_by {
            search_builder = search_builder.sort(sort.to_str());
        }

        if let Some(order) = params.order {
            search_builder = search_builder.order(order.to_str());
        }

        if let Some(per_page) = params.per_page {
            search_builder = search_builder.per_page(per_page);
        }

        if let Some(page) = params.page {
            search_builder = search_builder.page(page);
        }

        let results = search_builder
            .send()
            .await
            .map_err(|e| {
                tracing::error!("GitHub API request failed: {:?}", e);
                format!("Issue search failed: {}", e)
            })?;

        Ok(Self::convert_issue_search_results(results))
    }

    /// List repository refs (branches and tags)
    pub async fn list_repository_refs(
        &self,
        repo_info: &GitRemoteRepositoryInfo,
    ) -> Result<RepositoryRefs, String> {
        let repo_handler = self.client.repos(&repo_info.user, &repo_info.repo);

        // Get branches
        let branches_result = repo_handler
            .list_branches()
            .send()
            .await
            .map_err(|e| format!("Failed to list branches: {}", e))?;

        let branches = branches_result
            .items
            .into_iter()
            .map(|branch| {
                let name = branch.name.clone();
                ReferenceInfo {
                    name,
                    full_ref: format!("refs/heads/{}", branch.name),
                    commit_id: branch.commit.sha,
                }
            })
            .collect();

        // Get tags
        let tags_result = repo_handler
            .list_tags()
            .send()
            .await
            .map_err(|e| format!("Failed to list tags: {}", e))?;

        let tags = tags_result
            .items
            .into_iter()
            .map(|tag| {
                let name = tag.name.clone();
                ReferenceInfo {
                    name,
                    full_ref: format!("refs/tags/{}", tag.name),
                    commit_id: tag.commit.sha,
                }
            })
            .collect();

        Ok(RepositoryRefs { branches, tags })
    }

    /// Build issue and pull request search query from parameters
    fn build_issue_and_pull_request_search_query(params: &GithubIssueSearchParams) -> Result<String, String> {
        let mut query_parts = vec![params.query.clone()];

        let query_lower = params.query.to_lowercase();
        
        // Add both 'is:issue' and 'is:pull-request' if neither is present
        if !query_lower.contains("is:issue") && !query_lower.contains("is:pull-request") {
            query_parts.push("(is:issue OR is:pull-request)".to_string());
        }

        if let Some(repo) = &params.repository {
            let normalized_repo = Self::normalize_repository_identifier(repo);
            query_parts.push(format!("repo:{}", normalized_repo));
        }

        if let Some(labels) = &params.labels {
            query_parts.push(format!("label:{}", labels));
        }

        if let Some(state) = &params.state {
            query_parts.push(format!("state:{}", state));
        }

        if let Some(creator) = &params.creator {
            query_parts.push(format!("author:{}", creator));
        }

        if let Some(mentioned) = &params.mentioned {
            query_parts.push(format!("mentions:{}", mentioned));
        }

        if let Some(assignee) = &params.assignee {
            query_parts.push(format!("assignee:{}", assignee));
        }

        if let Some(milestone) = &params.milestone {
            query_parts.push(format!("milestone:{}", milestone));
        }

        if let Some(issue_type) = &params.issue_type {
            query_parts.push(format!("type:{}", issue_type));
        }

        Ok(query_parts.join(" "))
    }

    /// Normalize repository identifier from various formats to owner/repo
    fn normalize_repository_identifier(repo_input: &str) -> String {
        // If it's already in owner/repo format (no protocol prefixes), return as-is
        if !repo_input.contains("://")
            && !repo_input.starts_with("git@")
            && !repo_input.starts_with("github:")
            && repo_input.matches('/').count() == 1
        {
            return repo_input.to_string();
        }

        // Try to parse as GitHub URL and extract owner/repo
        if let Ok(github_info) = super::parse_github_repository_url_internal(repo_input) {
            format!(
                "{}/{}",
                github_info.repo_info.user, github_info.repo_info.repo
            )
        } else {
            // If parsing fails, return the input as-is (let GitHub API handle the error)
            repo_input.to_string()
        }
    }

    /// Convert octocrab repository search results to our format
    fn convert_repository_search_results(
        results: Page<OctocrabRepository>,
    ) -> RepositorySearchResults {
        let items = results
            .items
            .into_iter()
            .map(|repo| RepositoryItem {
                id: repo.id.to_string(),
                name: repo.name,
                description: repo.description,
                html_url: repo.html_url,
                homepage: repo.homepage,
                language: repo
                    .language
                    .as_ref()
                    .and_then(|l| l.as_str())
                    .map(|s| s.to_string()),
                stargazers_count: repo.stargazers_count.map(|c| c as u64),
                watchers_count: repo.watchers_count.map(|c| c as u64),
                forks_count: repo.forks_count.map(|c| c as u64),
                open_issues_count: repo.open_issues_count.map(|c| c as u64),
                topics: repo.topics,
                created_at: repo.created_at.map(|dt| dt.to_rfc3339()),
                updated_at: repo.updated_at.map(|dt| dt.to_rfc3339()),
                pushed_at: repo.pushed_at.map(|dt| dt.to_rfc3339()),
                size: repo.size.map(|s| s as u64),
                default_branch: repo.default_branch,
                archived: repo.archived,
                fork: repo.fork,
                private: repo.private,
                score: None, // octocrab doesn't expose score, use default
                owner: RepositoryOwner {
                    id: repo.owner.as_ref().map(|o| o.id.0.to_string()),
                    type_field: repo.owner.as_ref().map(|o| format!("{:?}", o.r#type)),
                },
                license: repo.license.map(|license| RepositoryLicense {
                    key: license.key,
                    name: license.name,
                }),
            })
            .collect();

        RepositorySearchResults {
            total_count: results.total_count.unwrap_or(0),
            incomplete_results: false, // octocrab doesn't expose this
            items,
        }
    }

    /// Convert octocrab issue search results to our format
    fn convert_issue_search_results(results: Page<OctocrabIssue>) -> IssueSearchResults {
        let items = results
            .items
            .into_iter()
            .map(|issue| IssueItem {
                id: issue.id.0.to_string(),
                number: issue.number,
                title: issue.title,
                body: issue.body,
                html_url: issue.html_url.to_string(),
                state: format!("{:?}", issue.state).to_lowercase(),
                user: IssueUser {
                    login: issue.user.login.clone(),
                    id: issue.user.id.0.to_string(),
                },
                assignee: issue.assignee.map(|assignee| IssueUser {
                    login: assignee.login,
                    id: assignee.id.0.to_string(),
                }),
                assignees: issue
                    .assignees
                    .into_iter()
                    .map(|assignee| IssueUser {
                        login: assignee.login,
                        id: assignee.id.0.to_string(),
                    })
                    .collect(),
                labels: issue
                    .labels
                    .into_iter()
                    .map(|label| IssueLabel {
                        id: label.id.0.to_string(),
                        name: label.name,
                        color: label.color,
                        description: label.description,
                    })
                    .collect(),
                created_at: issue.created_at.to_rfc3339(),
                updated_at: issue.updated_at.to_rfc3339(),
                closed_at: issue.closed_at.map(|dt| dt.to_rfc3339()),
                comments: issue.comments,
                score: None, // octocrab doesn't expose score
                repository: IssueRepository {
                    id: issue.id.0.to_string(),
                    name: issue
                        .repository_url
                        .path_segments()
                        .and_then(|mut segments| segments.nth(2))
                        .unwrap_or("")
                        .to_string(),

                    html_url: issue
                        .html_url
                        .to_string()
                        .replace("/issues/", "/")
                        .split("/issues/")
                        .next()
                        .unwrap_or("")
                        .to_string(),
                    description: None, // Not available in issue search
                    private: false,    // Not available in issue search
                    owner: RepositoryOwner {
                        id: Some(issue.user.id.0.to_string()),
                        type_field: Some(format!("{:?}", issue.user.r#type)),
                    },
                },
            })
            .collect();

        IssueSearchResults {
            total_count: results.total_count.unwrap_or(0),
            incomplete_results: false, // octocrab doesn't expose this
            items,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_issue_and_pull_request_search_query() {
        // Test basic query without is:issue or is:pull-request
        let params = GithubIssueSearchParams {
            query: "memory leak".to_string(),
            sort_by: None,
            order: None,
            per_page: None,
            page: None,
            repository: None,
            labels: None,
            state: None,
            creator: None,
            mentioned: None,
            assignee: None,
            milestone: None,
            issue_type: None,

        };
        
        let result = OctocrabGithubClient::build_issue_and_pull_request_search_query(&params).unwrap();
        assert_eq!(result, "memory leak (is:issue OR is:pull-request)");

        // Test query that already contains is:issue
        let params = GithubIssueSearchParams {
            query: "memory leak is:issue".to_string(),
            sort_by: None,
            order: None,
            per_page: None,
            page: None,
            repository: None,
            labels: None,
            state: None,
            creator: None,
            mentioned: None,
            assignee: None,
            milestone: None,
            issue_type: None,

        };
        
        let result = OctocrabGithubClient::build_issue_and_pull_request_search_query(&params).unwrap();
        assert_eq!(result, "memory leak is:issue");

        // Test query that already contains is:pull-request
        let params = GithubIssueSearchParams {
            query: "memory leak is:pull-request".to_string(),
            sort_by: None,
            order: None,
            per_page: None,
            page: None,
            repository: None,
            labels: None,
            state: None,
            creator: None,
            mentioned: None,
            assignee: None,
            milestone: None,
            issue_type: None,

        };
        
        let result = OctocrabGithubClient::build_issue_and_pull_request_search_query(&params).unwrap();
        assert_eq!(result, "memory leak is:pull-request");

        // Test with additional parameters
        let params = GithubIssueSearchParams {
            query: "bug".to_string(),
            sort_by: None,
            order: None,
            per_page: None,
            page: None,
            repository: Some("rust-lang/rust".to_string()),
            labels: Some("enhancement".to_string()),
            state: Some("open".to_string()),
            creator: Some("user123".to_string()),
            mentioned: None,
            assignee: None,
            milestone: None,
            issue_type: None,

        };
        
        let result = OctocrabGithubClient::build_issue_and_pull_request_search_query(&params).unwrap();
        assert_eq!(result, "bug (is:issue OR is:pull-request) repo:rust-lang/rust label:enhancement state:open author:user123");
    }
}
