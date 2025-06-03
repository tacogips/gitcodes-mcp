#![cfg(feature = "lancedb-backend")]

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use arrow_array::{
    ArrayRef, Float32Array, Int64Array, RecordBatch, RecordBatchIterator, StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use lancedb::index::scalar::FtsIndexBuilder;
use lancedb::index::Index;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::{connect, Connection, Table};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ids::FullId;
use crate::types::{
    GitHubComment, GitHubIssue, GitHubPullRequest, GitHubPullRequestFile, GitHubRepository,
    GitHubUser,
};

const REPOSITORIES_TABLE: &str = "repositories";
const ISSUES_TABLE: &str = "issues";
const PULL_REQUESTS_TABLE: &str = "pull_requests";
const COMMENTS_TABLE: &str = "comments";
const USERS_TABLE: &str = "users";
const FILES_TABLE: &str = "pull_request_files";

pub struct LanceDbStore {
    connection: Connection,
    data_dir: PathBuf,
}

impl LanceDbStore {
    pub async fn new(data_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&data_dir)?;
        let connection = connect(data_dir.to_str().unwrap()).execute().await?;

        let store = Self {
            connection,
            data_dir,
        };

        store.initialize_tables().await?;
        Ok(store)
    }

    async fn initialize_tables(&self) -> Result<()> {
        // Create repositories table if it doesn't exist
        if !self.table_exists(REPOSITORIES_TABLE).await? {
            self.create_repositories_table().await?;
        }

        // Create issues table if it doesn't exist
        if !self.table_exists(ISSUES_TABLE).await? {
            self.create_issues_table().await?;
        }

        // Create pull requests table if it doesn't exist
        if !self.table_exists(PULL_REQUESTS_TABLE).await? {
            self.create_pull_requests_table().await?;
        }

        // Create comments table if it doesn't exist
        if !self.table_exists(COMMENTS_TABLE).await? {
            self.create_comments_table().await?;
        }

        // Create users table if it doesn't exist
        if !self.table_exists(USERS_TABLE).await? {
            self.create_users_table().await?;
        }

        // Create files table if it doesn't exist
        if !self.table_exists(FILES_TABLE).await? {
            self.create_files_table().await?;
        }

        Ok(())
    }

    async fn table_exists(&self, table_name: &str) -> Result<bool> {
        let tables = self.connection.table_names().execute().await?;
        Ok(tables.contains(&table_name.to_string()))
    }

    async fn create_repositories_table(&self) -> Result<()> {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("owner", DataType::Utf8, false),
            Field::new("name", DataType::Utf8, false),
            Field::new("full_name", DataType::Utf8, false),
            Field::new("description", DataType::Utf8, true),
            Field::new("url", DataType::Utf8, false),
            Field::new("clone_url", DataType::Utf8, false),
            Field::new("created_at", DataType::Utf8, false),
            Field::new("updated_at", DataType::Utf8, false),
            Field::new("language", DataType::Utf8, true),
            Field::new("fork", DataType::Boolean, false),
            Field::new("forks_count", DataType::Int64, false),
            Field::new("stargazers_count", DataType::Int64, false),
            Field::new("open_issues_count", DataType::Int64, false),
            Field::new("is_template", DataType::Boolean, false),
            Field::new("topics", DataType::Utf8, true), // JSON array as string
            Field::new("visibility", DataType::Utf8, false),
            Field::new("default_branch", DataType::Utf8, false),
            Field::new("permissions", DataType::Utf8, true), // JSON object as string
            Field::new("license", DataType::Utf8, true),
            Field::new("archived", DataType::Boolean, false),
            Field::new("disabled", DataType::Boolean, false),
            Field::new("data", DataType::Utf8, false), // Full JSON data
        ]));

        // Create empty table
        let batch = RecordBatch::new_empty(schema.clone());
        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema.clone());

        let table = self
            .connection
            .create_table(REPOSITORIES_TABLE, Box::new(batches))
            .execute()
            .await?;

        // Create FTS index on searchable fields
        table
            .create_index(
                &["full_name", "description"],
                Index::FTS(FtsIndexBuilder::default()),
            )
            .execute()
            .await?;

        Ok(())
    }

    async fn create_issues_table(&self) -> Result<()> {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("repository_id", DataType::Utf8, false),
            Field::new("number", DataType::Int64, false),
            Field::new("title", DataType::Utf8, false),
            Field::new("body", DataType::Utf8, true),
            Field::new("state", DataType::Utf8, false),
            Field::new("user_login", DataType::Utf8, false),
            Field::new("assignees", DataType::Utf8, true), // JSON array as string
            Field::new("labels", DataType::Utf8, true),    // JSON array as string
            Field::new("milestone", DataType::Utf8, true),
            Field::new("created_at", DataType::Utf8, false),
            Field::new("updated_at", DataType::Utf8, false),
            Field::new("closed_at", DataType::Utf8, true),
            Field::new("data", DataType::Utf8, false), // Full JSON data
        ]));

        let batch = RecordBatch::new_empty(schema.clone());
        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema.clone());

        let table = self
            .connection
            .create_table(ISSUES_TABLE, Box::new(batches))
            .execute()
            .await?;

        // Create FTS index
        table
            .create_index(
                &["title", "body", "labels"],
                Index::FTS(FtsIndexBuilder::default()),
            )
            .execute()
            .await?;

        Ok(())
    }

    async fn create_pull_requests_table(&self) -> Result<()> {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("repository_id", DataType::Utf8, false),
            Field::new("number", DataType::Int64, false),
            Field::new("title", DataType::Utf8, false),
            Field::new("body", DataType::Utf8, true),
            Field::new("state", DataType::Utf8, false),
            Field::new("user_login", DataType::Utf8, false),
            Field::new("assignees", DataType::Utf8, true), // JSON array as string
            Field::new("labels", DataType::Utf8, true),    // JSON array as string
            Field::new("milestone", DataType::Utf8, true),
            Field::new("created_at", DataType::Utf8, false),
            Field::new("updated_at", DataType::Utf8, false),
            Field::new("closed_at", DataType::Utf8, true),
            Field::new("merged_at", DataType::Utf8, true),
            Field::new("head_ref", DataType::Utf8, false),
            Field::new("base_ref", DataType::Utf8, false),
            Field::new("draft", DataType::Boolean, false),
            Field::new("data", DataType::Utf8, false), // Full JSON data
        ]));

        let batch = RecordBatch::new_empty(schema.clone());
        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema.clone());

        let table = self
            .connection
            .create_table(PULL_REQUESTS_TABLE, Box::new(batches))
            .execute()
            .await?;

        // Create FTS index
        table
            .create_index(
                &["title", "body", "labels"],
                Index::FTS(FtsIndexBuilder::default()),
            )
            .execute()
            .await?;

        Ok(())
    }

    async fn create_comments_table(&self) -> Result<()> {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("issue_id", DataType::Utf8, false),
            Field::new("user_login", DataType::Utf8, false),
            Field::new("body", DataType::Utf8, false),
            Field::new("created_at", DataType::Utf8, false),
            Field::new("updated_at", DataType::Utf8, false),
            Field::new("data", DataType::Utf8, false), // Full JSON data
        ]));

        let batch = RecordBatch::new_empty(schema.clone());
        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema.clone());

        let table = self
            .connection
            .create_table(COMMENTS_TABLE, Box::new(batches))
            .execute()
            .await?;

        // Create FTS index
        table
            .create_index(&["body"], Index::FTS(FtsIndexBuilder::default()))
            .execute()
            .await?;

        Ok(())
    }

    async fn create_users_table(&self) -> Result<()> {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("login", DataType::Utf8, false),
            Field::new("name", DataType::Utf8, true),
            Field::new("email", DataType::Utf8, true),
            Field::new("avatar_url", DataType::Utf8, true),
            Field::new("html_url", DataType::Utf8, false),
            Field::new("bio", DataType::Utf8, true),
            Field::new("company", DataType::Utf8, true),
            Field::new("location", DataType::Utf8, true),
            Field::new("created_at", DataType::Utf8, false),
            Field::new("updated_at", DataType::Utf8, false),
            Field::new("public_repos", DataType::Int64, false),
            Field::new("followers", DataType::Int64, false),
            Field::new("following", DataType::Int64, false),
            Field::new("data", DataType::Utf8, false), // Full JSON data
        ]));

        let batch = RecordBatch::new_empty(schema.clone());
        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema.clone());

        let table = self
            .connection
            .create_table(USERS_TABLE, Box::new(batches))
            .execute()
            .await?;

        // Create FTS index
        table
            .create_index(
                &["login", "name", "bio", "company"],
                Index::FTS(FtsIndexBuilder::default()),
            )
            .execute()
            .await?;

        Ok(())
    }

    async fn create_files_table(&self) -> Result<()> {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("pull_request_id", DataType::Utf8, false),
            Field::new("filename", DataType::Utf8, false),
            Field::new("status", DataType::Utf8, false),
            Field::new("additions", DataType::Int64, false),
            Field::new("deletions", DataType::Int64, false),
            Field::new("changes", DataType::Int64, false),
            Field::new("patch", DataType::Utf8, true),
            Field::new("data", DataType::Utf8, false), // Full JSON data
        ]));

        let batch = RecordBatch::new_empty(schema.clone());
        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema.clone());

        let table = self
            .connection
            .create_table(FILES_TABLE, Box::new(batches))
            .execute()
            .await?;

        // Create FTS index
        table
            .create_index(
                &["filename", "patch"],
                Index::FTS(FtsIndexBuilder::default()),
            )
            .execute()
            .await?;

        Ok(())
    }

    // Repository operations
    pub async fn save_repository(&self, repo: &GitHubRepository) -> Result<()> {
        let table = self.connection.open_table(REPOSITORIES_TABLE).execute().await?;
        
        let id = vec![repo.full_id().to_string()];
        let owner = vec![repo.owner.clone()];
        let name = vec![repo.name.clone()];
        let full_name = vec![repo.full_name.clone()];
        let description = vec![repo.description.clone()];
        let url = vec![repo.url.clone()];
        let clone_url = vec![repo.clone_url.clone()];
        let created_at = vec![repo.created_at.to_rfc3339()];
        let updated_at = vec![repo.updated_at.to_rfc3339()];
        let language = vec![repo.language.clone()];
        let fork = vec![repo.fork];
        let forks_count = vec![repo.forks_count as i64];
        let stargazers_count = vec![repo.stargazers_count as i64];
        let open_issues_count = vec![repo.open_issues_count as i64];
        let is_template = vec![repo.is_template.unwrap_or(false)];
        let topics = vec![serde_json::to_string(&repo.topics)?];
        let visibility = vec![repo.visibility.clone()];
        let default_branch = vec![repo.default_branch.clone()];
        let permissions = vec![repo
            .permissions
            .as_ref()
            .map(|p| serde_json::to_string(p).unwrap())];
        let license = vec![repo.license.as_ref().map(|l| l.name.clone())];
        let archived = vec![repo.archived];
        let disabled = vec![repo.disabled];
        let data = vec![serde_json::to_string(&repo)?];

        let batch = RecordBatch::try_new(
            table.schema().clone(),
            vec![
                Arc::new(StringArray::from(id)) as ArrayRef,
                Arc::new(StringArray::from(owner)),
                Arc::new(StringArray::from(name)),
                Arc::new(StringArray::from(full_name)),
                Arc::new(StringArray::from(description)),
                Arc::new(StringArray::from(url)),
                Arc::new(StringArray::from(clone_url)),
                Arc::new(StringArray::from(created_at)),
                Arc::new(StringArray::from(updated_at)),
                Arc::new(StringArray::from(language)),
                Arc::new(arrow_array::BooleanArray::from(fork)),
                Arc::new(Int64Array::from(forks_count)),
                Arc::new(Int64Array::from(stargazers_count)),
                Arc::new(Int64Array::from(open_issues_count)),
                Arc::new(arrow_array::BooleanArray::from(is_template)),
                Arc::new(StringArray::from(topics)),
                Arc::new(StringArray::from(visibility)),
                Arc::new(StringArray::from(default_branch)),
                Arc::new(StringArray::from(permissions)),
                Arc::new(StringArray::from(license)),
                Arc::new(arrow_array::BooleanArray::from(archived)),
                Arc::new(arrow_array::BooleanArray::from(disabled)),
                Arc::new(StringArray::from(data)),
            ],
        )?;

        table.add(vec![batch]).execute().await?;
        Ok(())
    }

    pub async fn get_repository(&self, full_id: &FullId) -> Result<Option<GitHubRepository>> {
        let table = self.connection.open_table(REPOSITORIES_TABLE).execute().await?;
        
        let filter = format!("id = '{}'", full_id.to_string());
        let mut results = table
            .query()
            .filter(filter.as_str())
            .limit(1)
            .execute()
            .await?;

        if let Some(batch) = results.next().await? {
            if batch.num_rows() > 0 {
                let data_array = batch
                    .column_by_name("data")
                    .ok_or_else(|| anyhow!("Missing data column"))?
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .ok_or_else(|| anyhow!("Invalid data column type"))?;

                let json_str = data_array.value(0);
                let repo: GitHubRepository = serde_json::from_str(json_str)?;
                return Ok(Some(repo));
            }
        }

        Ok(None)
    }

    // Search operations using LanceDB's native FTS
    pub async fn search_repositories(&self, query: &str, limit: usize) -> Result<Vec<GitHubRepository>> {
        let table = self.connection.open_table(REPOSITORIES_TABLE).execute().await?;
        
        let mut results = table
            .query()
            .full_text_search(lance_index::scalar::FullTextSearchQuery::new(query.to_string()))
            .limit(limit)
            .execute()
            .await?;

        let mut repositories = Vec::new();
        while let Some(batch) = results.next().await? {
            let data_array = batch
                .column_by_name("data")
                .ok_or_else(|| anyhow!("Missing data column"))?
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or_else(|| anyhow!("Invalid data column type"))?;

            for i in 0..batch.num_rows() {
                let json_str = data_array.value(i);
                let repo: GitHubRepository = serde_json::from_str(json_str)?;
                repositories.push(repo);
            }
        }

        Ok(repositories)
    }

    // Similar implementations for issues, PRs, etc.
    pub async fn save_issue(&self, issue: &GitHubIssue) -> Result<()> {
        let table = self.connection.open_table(ISSUES_TABLE).execute().await?;
        
        let assignees_json = serde_json::to_string(
            &issue
                .assignees
                .iter()
                .map(|u| u.login.clone())
                .collect::<Vec<_>>(),
        )?;
        let labels_json = serde_json::to_string(
            &issue.labels.iter().map(|l| &l.name).collect::<Vec<_>>(),
        )?;

        let id = vec![issue.full_id().to_string()];
        let repository_id = vec![issue.repository_id.to_string()];
        let number = vec![issue.number as i64];
        let title = vec![issue.title.clone()];
        let body = vec![issue.body.clone()];
        let state = vec![issue.state.clone()];
        let user_login = vec![issue.user.login.clone()];
        let assignees = vec![Some(assignees_json)];
        let labels = vec![Some(labels_json)];
        let milestone = vec![issue.milestone.as_ref().map(|m| m.title.clone())];
        let created_at = vec![issue.created_at.to_rfc3339()];
        let updated_at = vec![issue.updated_at.to_rfc3339()];
        let closed_at = vec![issue.closed_at.as_ref().map(|dt| dt.to_rfc3339())];
        let data = vec![serde_json::to_string(&issue)?];

        let batch = RecordBatch::try_new(
            table.schema().clone(),
            vec![
                Arc::new(StringArray::from(id)) as ArrayRef,
                Arc::new(StringArray::from(repository_id)),
                Arc::new(Int64Array::from(number)),
                Arc::new(StringArray::from(title)),
                Arc::new(StringArray::from(body)),
                Arc::new(StringArray::from(state)),
                Arc::new(StringArray::from(user_login)),
                Arc::new(StringArray::from(assignees)),
                Arc::new(StringArray::from(labels)),
                Arc::new(StringArray::from(milestone)),
                Arc::new(StringArray::from(created_at)),
                Arc::new(StringArray::from(updated_at)),
                Arc::new(StringArray::from(closed_at)),
                Arc::new(StringArray::from(data)),
            ],
        )?;

        table.add(vec![batch]).execute().await?;
        Ok(())
    }

    pub async fn search_issues(&self, query: &str, limit: usize) -> Result<Vec<GitHubIssue>> {
        let table = self.connection.open_table(ISSUES_TABLE).execute().await?;
        
        let mut results = table
            .query()
            .full_text_search(lance_index::scalar::FullTextSearchQuery::new(query.to_string()))
            .limit(limit)
            .execute()
            .await?;

        let mut issues = Vec::new();
        while let Some(batch) = results.next().await? {
            let data_array = batch
                .column_by_name("data")
                .ok_or_else(|| anyhow!("Missing data column"))?
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or_else(|| anyhow!("Invalid data column type"))?;

            for i in 0..batch.num_rows() {
                let json_str = data_array.value(i);
                let issue: GitHubIssue = serde_json::from_str(json_str)?;
                issues.push(issue);
            }
        }

        Ok(issues)
    }

    // Combined search across all tables
    pub async fn search_all(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();

        // Search repositories
        let repos = self.search_repositories(query, limit).await?;
        for repo in repos {
            results.push(SearchResult::Repository(repo));
        }

        // Search issues
        let issues = self.search_issues(query, limit).await?;
        for issue in issues {
            results.push(SearchResult::Issue(issue));
        }

        // Limit total results
        results.truncate(limit);
        Ok(results)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchResult {
    Repository(GitHubRepository),
    Issue(GitHubIssue),
    PullRequest(GitHubPullRequest),
    Comment(GitHubComment),
    User(GitHubUser),
    File(GitHubPullRequestFile),
}

// Module for future hybrid search with embeddings
pub mod hybrid {
    use super::*;
    
    // This will be implemented when we add vector embeddings
    pub struct HybridSearchQuery {
        pub text_query: String,
        pub embedding: Option<Vec<f32>>,
        pub rerank: bool,
    }
    
    // Placeholder for future implementation
    pub async fn hybrid_search(
        _store: &LanceDbStore,
        _query: HybridSearchQuery,
        _limit: usize,
    ) -> Result<Vec<SearchResult>> {
        // TODO: Implement hybrid search combining text and vector search
        // 1. Perform text search using FTS
        // 2. Perform vector search if embedding provided
        // 3. Combine results using reciprocal rank fusion or other reranking
        // 4. Return reranked results
        
        unimplemented!("Hybrid search will be implemented with vector embeddings")
    }
}