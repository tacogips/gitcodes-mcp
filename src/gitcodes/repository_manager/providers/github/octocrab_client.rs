//! Octocrab-based GitHub client implementation
//!
//! This module provides a GitHub API client using the octocrab library,
//! replacing the custom HTTP implementation while maintaining the same interface.

use super::{GithubIssueSearchParams, GithubSearchParams, GitRemoteRepositoryInfo};
use crate::gitcodes::repository_manager::providers::*;
use octocrab::{Octocrab, Page};
use octocrab::models::{Repository as OctocrabRepository, issues::Issue as OctocrabIssue};

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

    /// Search issues using octocrab
    pub async fn search_issues(
        &self,
        params: GithubIssueSearchParams,
    ) -> Result<IssueSearchResults, String> {
        let query = Self::build_issue_search_query(&params);
        
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
            .map_err(|e| format!("Issue search failed: {}", e))?;

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

    /// Build issue search query from parameters
    fn build_issue_search_query(params: &GithubIssueSearchParams) -> String {
        let mut query_parts = vec![params.query.clone()];

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

        query_parts.join(" ")
    }

    /// Normalize repository identifier from various formats to owner/repo
    fn normalize_repository_identifier(repo_input: &str) -> String {
        // If it's already in owner/repo format (no protocol prefixes), return as-is
        if !repo_input.contains("://") && !repo_input.starts_with("git@") && !repo_input.starts_with("github:") && repo_input.matches('/').count() == 1 {
            return repo_input.to_string();
        }

        // Try to parse as GitHub URL and extract owner/repo
        if let Ok(github_info) = super::parse_github_repository_url_internal(repo_input) {
            format!("{}/{}", github_info.repo_info.user, github_info.repo_info.repo)
        } else {
            // If parsing fails, return the input as-is (let GitHub API handle the error)
            repo_input.to_string()
        }
    }

    /// Convert octocrab repository search results to our format
    fn convert_repository_search_results(results: Page<OctocrabRepository>) -> RepositorySearchResults {
        let items = results
            .items
            .into_iter()
            .map(|repo| RepositoryItem {
                id: repo.id.to_string(),
                name: repo.name,
                full_name: repo.full_name.unwrap_or_default(),
                description: repo.description,
                html_url: repo.html_url.map(|u| u.to_string()).unwrap_or_default(),
                homepage: repo.homepage,
                language: repo.language.as_ref().and_then(|l| l.as_str()).map(|s| s.to_string()),
                stargazers_count: repo.stargazers_count.unwrap_or(0) as u64,
                watchers_count: repo.watchers_count.unwrap_or(0) as u64,
                forks_count: repo.forks_count.unwrap_or(0) as u64,
                open_issues_count: repo.open_issues_count.unwrap_or(0) as u64,
                topics: repo.topics.unwrap_or_default(),
                created_at: repo.created_at.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
                updated_at: repo.updated_at.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
                pushed_at: repo.pushed_at.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
                size: repo.size.unwrap_or(0) as u64,
                default_branch: repo.default_branch.unwrap_or_default(),
                archived: repo.archived.unwrap_or(false),
                fork: repo.fork.unwrap_or(false),
                private: repo.private.unwrap_or(false),
                score: Some(1.0), // octocrab doesn't expose score, use default
                owner: RepositoryOwner {
                    login: repo.owner.as_ref().map(|o| o.login.clone()).unwrap_or_default(),
                    id: repo.owner.as_ref().map(|o| o.id.0.to_string()).unwrap_or_default(),
                    type_field: repo.owner.as_ref().map(|o| format!("{:?}", o.r#type)).unwrap_or_default(),
                },
                license: repo.license.as_ref().map(|license| RepositoryLicense {
                    key: license.key.clone(),
                    name: license.name.clone(),
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
                    type_field: format!("{:?}", issue.user.r#type),
                    html_url: "".to_string(),
                },
                assignee: issue.assignee.map(|assignee| IssueUser {
                    login: assignee.login,
                    id: assignee.id.0.to_string(),
                    type_field: format!("{:?}", assignee.r#type),
                    html_url: "".to_string(),
                }),
                assignees: issue
                    .assignees
                    .into_iter()
                    .map(|assignee| IssueUser {
                        login: assignee.login,
                        id: assignee.id.0.to_string(),
                        type_field: format!("{:?}", assignee.r#type),
                        html_url: "".to_string(),
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
                milestone: issue.milestone.map(|milestone| IssueMilestone {
                    id: milestone.id.0.to_string(),
                    number: milestone.number as u64,
                    title: milestone.title,
                    description: milestone.description,
                    state: milestone.state.unwrap_or_default(),
                    created_at: milestone.created_at.to_rfc3339(),
                    updated_at: milestone.updated_at.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
                    due_on: milestone.due_on.map(|dt| dt.to_rfc3339()),
                    closed_at: milestone.closed_at.map(|dt| dt.to_rfc3339()),
                }),
                created_at: issue.created_at.to_rfc3339(),
                updated_at: issue.updated_at.to_rfc3339(),
                closed_at: issue.closed_at.map(|dt| dt.to_rfc3339()),
                comments: issue.comments as u64,
                score: None, // octocrab doesn't expose score
                repository: IssueRepository {
                    id: "".to_string(), // Not directly available
                    name: "".to_string(), // Not directly available
                    full_name: "".to_string(),
                    html_url: "".to_string(),
                    description: None, // Not available in issue search
                    private: false, // Not available in issue search
                    owner: RepositoryOwner {
                        login: issue.user.login.clone(), // Best guess
                        id: issue.user.id.0.to_string(),
                        type_field: format!("{:?}", issue.user.r#type),
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