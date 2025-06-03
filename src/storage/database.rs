use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sled::Db;
use std::sync::Arc;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy, doc};

use crate::ids::RepositoryId;
use crate::storage::models::*;
use crate::storage::paths::StoragePaths;
use crate::types::{ItemType, ResourceType, SyncStatusType};

pub struct GitDatabase {
    db: Db,
    search_index: Index,
    index_writer: Arc<std::sync::Mutex<IndexWriter>>,
    index_reader: IndexReader,
    paths: StoragePaths,
}

impl GitDatabase {
    /// Creates a new GitDatabase instance with search index and storage.
    ///
    /// # Returns
    ///
    /// Returns a Result containing the initialized GitDatabase.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Storage paths initialization fails
    /// - Sled database opening fails
    /// - Tantivy search index creation or opening fails
    pub async fn new() -> Result<Self> {
        let paths = StoragePaths::new()?;

        // Open sled database
        let db = sled::open(&paths.database_path()).context("Failed to open sled database")?;

        // Create tantivy search index
        let mut schema_builder = Schema::builder();

        // Define fields for search
        schema_builder.add_text_field("id", STRING | STORED);
        schema_builder.add_text_field("type", STRING | STORED);
        schema_builder.add_text_field("repository_id", STRING | STORED);
        schema_builder.add_text_field("title", TEXT | STORED);
        schema_builder.add_text_field("body", TEXT | STORED);
        schema_builder.add_text_field("author", TEXT | STORED);
        schema_builder.add_text_field("state", STRING | STORED);
        schema_builder.add_text_field("labels", TEXT | STORED);
        schema_builder.add_text_field("assignees", TEXT | STORED);

        let schema = schema_builder.build();

        let index_path = paths.data_dir.join("search_index");
        std::fs::create_dir_all(&index_path)?;

        let index = if index_path.exists() && index_path.read_dir()?.next().is_some() {
            Index::open_in_dir(&index_path)?
        } else {
            Index::create_in_dir(&index_path, schema)?
        };

        let index_writer = Arc::new(std::sync::Mutex::new(index.writer(50_000_000)?));
        let index_reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        Ok(Self {
            db,
            search_index: index,
            index_writer,
            index_reader,
            paths,
        })
    }

    // Repository operations
    /// Inserts or updates a repository in the database.
    ///
    /// # Arguments
    ///
    /// * `repo` - The Repository to insert or update
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail.
    pub async fn upsert_repository(&self, repo: &Repository) -> Result<()> {
        let tree = self.db.open_tree("repositories")?;

        let key = format!("repo:{}", repo.full_name);
        let value = serde_json::to_vec(repo)?;

        tree.insert(key, value)?;
        tree.flush()?;

        Ok(())
    }

    /// Retrieves a repository by its full name (owner/repo).
    ///
    /// # Arguments
    ///
    /// * `full_name` - The full repository name in format "owner/repo"
    ///
    /// # Returns
    ///
    /// Returns an Option containing the Repository if found.
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail.
    pub async fn get_repository_by_full_name(&self, full_name: &str) -> Result<Option<Repository>> {
        let tree = self.db.open_tree("repositories")?;

        let key = format!("repo:{}", full_name);

        match tree.get(key)? {
            Some(value) => {
                let repo: Repository = serde_json::from_slice(&value)?;
                Ok(Some(repo))
            }
            None => Ok(None),
        }
    }

    /// Retrieves a repository by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The repository ID
    ///
    /// # Returns
    ///
    /// Returns an Option containing the Repository if found.
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail.
    pub async fn get_repository_by_id(&self, id: RepositoryId) -> Result<Option<Repository>> {
        let tree = self.db.open_tree("repositories")?;

        // We need to iterate through all repositories to find by ID
        // In a production system, we might want to maintain a separate ID index
        for item in tree.iter() {
            let (_, value) = item?;
            let repo: Repository = serde_json::from_slice(&value)?;
            if repo.id == id {
                return Ok(Some(repo));
            }
        }

        Ok(None)
    }

    /// Lists all repositories in the database.
    ///
    /// # Returns
    ///
    /// Returns a Vec of all Repository entries.
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail.
    pub async fn list_repositories(&self) -> Result<Vec<Repository>> {
        let tree = self.db.open_tree("repositories")?;

        let mut repos = Vec::new();
        for item in tree.iter() {
            let (_, value) = item?;
            let repo: Repository = serde_json::from_slice(&value)?;
            repos.push(repo);
        }

        Ok(repos)
    }

    // Issue operations
    /// Inserts or updates an issue in the database and search index.
    ///
    /// # Arguments
    ///
    /// * `issue` - The Issue to insert or update
    ///
    /// # Errors
    ///
    /// Returns an error if database operations or search indexing fails.
    pub async fn upsert_issue(&self, issue: &Issue) -> Result<()> {
        let tree = self.db.open_tree("issues")?;

        let key = format!("issue:{}:{}", issue.repository_id, issue.number);
        let value = serde_json::to_vec(issue)?;

        tree.insert(key.as_bytes(), value)?;

        // Index for search
        self.index_issue_for_search(issue).await?;

        tree.flush()?;

        Ok(())
    }

    async fn index_issue_for_search(&self, issue: &Issue) -> Result<()> {
        let schema = self.search_index.schema();

        let id_field = schema.get_field("id").unwrap();
        let type_field = schema.get_field("type").unwrap();
        let repository_id_field = schema.get_field("repository_id").unwrap();
        let title_field = schema.get_field("title").unwrap();
        let body_field = schema.get_field("body").unwrap();
        let author_field = schema.get_field("author").unwrap();
        let state_field = schema.get_field("state").unwrap();
        let labels_field = schema.get_field("labels").unwrap();
        let assignees_field = schema.get_field("assignees").unwrap();

        let mut doc = doc!(
            id_field => issue.id.to_string(),
            type_field => ItemType::Issue.to_string(),
            repository_id_field => issue.repository_id.to_string(),
            title_field => issue.title.clone(),
            author_field => issue.author.clone(),
            state_field => issue.state.to_string(),
            labels_field => issue.labels.join(" "),
            assignees_field => issue.assignees.join(" ")
        );

        if let Some(body) = &issue.body {
            doc.add_text(body_field, body);
        }

        let mut writer = self.index_writer.lock().unwrap();
        writer.add_document(doc)?;
        writer.commit()?;

        Ok(())
    }

    /// Retrieves issues for a specific repository, optionally filtered by update time.
    ///
    /// # Arguments
    ///
    /// * `repository_id` - The ID of the repository
    /// * `since` - Optional DateTime to filter issues updated after this time
    ///
    /// # Returns
    ///
    /// Returns a Vec of Issue entries matching the criteria.
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail.
    pub async fn get_issues_by_repository(
        &self,
        repository_id: RepositoryId,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<Issue>> {
        let tree = self.db.open_tree("issues")?;

        let prefix = format!("issue:{}:", repository_id);
        let mut issues = Vec::new();

        for item in tree.scan_prefix(prefix.as_bytes()) {
            let (_, value) = item?;
            let issue: Issue = serde_json::from_slice(&value)?;

            if let Some(since_date) = since {
                if issue.updated_at > since_date {
                    issues.push(issue);
                }
            } else {
                issues.push(issue);
            }
        }

        Ok(issues)
    }

    // Pull request operations
    /// Inserts or updates a pull request in the database and search index.
    ///
    /// # Arguments
    ///
    /// * `pr` - The PullRequest to insert or update
    ///
    /// # Errors
    ///
    /// Returns an error if database operations or search indexing fails.
    pub async fn upsert_pull_request(&self, pr: &PullRequest) -> Result<()> {
        let tree = self.db.open_tree("pull_requests")?;

        let key = format!("pr:{}:{}", pr.repository_id, pr.number);
        let value = serde_json::to_vec(pr)?;

        tree.insert(key.as_bytes(), value)?;

        // Index for search
        self.index_pull_request_for_search(pr).await?;

        tree.flush()?;

        Ok(())
    }

    async fn index_pull_request_for_search(&self, pr: &PullRequest) -> Result<()> {
        let schema = self.search_index.schema();

        let id_field = schema.get_field("id").unwrap();
        let type_field = schema.get_field("type").unwrap();
        let repository_id_field = schema.get_field("repository_id").unwrap();
        let title_field = schema.get_field("title").unwrap();
        let body_field = schema.get_field("body").unwrap();
        let author_field = schema.get_field("author").unwrap();
        let state_field = schema.get_field("state").unwrap();
        let labels_field = schema.get_field("labels").unwrap();
        let assignees_field = schema.get_field("assignees").unwrap();

        let mut doc = doc!(
            id_field => pr.id.to_string(),
            type_field => ItemType::PullRequest.to_string(),
            repository_id_field => pr.repository_id.to_string(),
            title_field => pr.title.clone(),
            author_field => pr.author.clone(),
            state_field => pr.state.to_string(),
            labels_field => pr.labels.join(" "),
            assignees_field => pr.assignees.join(" ")
        );

        if let Some(body) = &pr.body {
            doc.add_text(body_field, body);
        }

        let mut writer = self.index_writer.lock().unwrap();
        writer.add_document(doc)?;
        writer.commit()?;

        Ok(())
    }

    /// Retrieves pull requests for a specific repository, optionally filtered by update time.
    ///
    /// # Arguments
    ///
    /// * `repository_id` - The ID of the repository
    /// * `since` - Optional DateTime to filter pull requests updated after this time
    ///
    /// # Returns
    ///
    /// Returns a Vec of PullRequest entries matching the criteria.
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail.
    pub async fn get_pull_requests_by_repository(
        &self,
        repository_id: RepositoryId,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<PullRequest>> {
        let tree = self.db.open_tree("pull_requests")?;

        let prefix = format!("pr:{}:", repository_id);
        let mut prs = Vec::new();

        for item in tree.scan_prefix(prefix.as_bytes()) {
            let (_, value) = item?;
            let pr: PullRequest = serde_json::from_slice(&value)?;

            if let Some(since_date) = since {
                if pr.updated_at > since_date {
                    prs.push(pr);
                }
            } else {
                prs.push(pr);
            }
        }

        Ok(prs)
    }

    // Comment operations
    /// Inserts or updates an issue comment in the database.
    ///
    /// # Arguments
    ///
    /// * `comment` - The IssueComment to insert or update
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail.
    pub async fn upsert_issue_comment(&self, comment: &IssueComment) -> Result<()> {
        let tree = self.db.open_tree("issue_comments")?;

        let key = format!("issue_comment:{}:{}", comment.issue_id, comment.comment_id);
        let value = serde_json::to_vec(comment)?;

        tree.insert(key.as_bytes(), value)?;
        tree.flush()?;

        Ok(())
    }

    /// Inserts or updates a pull request comment in the database.
    ///
    /// # Arguments
    ///
    /// * `comment` - The PullRequestComment to insert or update
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail.
    pub async fn upsert_pull_request_comment(&self, comment: &PullRequestComment) -> Result<()> {
        let tree = self.db.open_tree("pr_comments")?;

        let key = format!(
            "pr_comment:{}:{}",
            comment.pull_request_id, comment.comment_id
        );
        let value = serde_json::to_vec(comment)?;

        tree.insert(key.as_bytes(), value)?;
        tree.flush()?;

        Ok(())
    }

    // Sync status operations
    /// Retrieves the last successful sync status for a repository and resource type.
    ///
    /// # Arguments
    ///
    /// * `repository_id` - The ID of the repository
    /// * `resource_type` - The type of resource (Issues or PullRequests)
    ///
    /// # Returns
    ///
    /// Returns an Option containing the most recent successful SyncStatus.
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail.
    pub async fn get_last_sync_status(
        &self,
        repository_id: RepositoryId,
        resource_type: ResourceType,
    ) -> Result<Option<SyncStatus>> {
        let tree = self.db.open_tree("sync_status")?;

        let prefix = format!("sync:{}:{}:", repository_id, resource_type);
        let mut latest: Option<SyncStatus> = None;

        for item in tree.scan_prefix(prefix.as_bytes()) {
            let (_, value) = item?;
            let status: SyncStatus = serde_json::from_slice(&value)?;

            if status.status == SyncStatusType::Success {
                match &latest {
                    None => latest = Some(status),
                    Some(current) => {
                        if status.last_synced_at > current.last_synced_at {
                            latest = Some(status);
                        }
                    }
                }
            }
        }

        Ok(latest)
    }

    /// Updates the sync status for a repository and resource type.
    ///
    /// # Arguments
    ///
    /// * `status` - The SyncStatus to store
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail.
    pub async fn update_sync_status(&self, status: &SyncStatus) -> Result<()> {
        let tree = self.db.open_tree("sync_status")?;

        let timestamp = status.last_synced_at.timestamp_millis();
        let key = format!(
            "sync:{}:{}:{}",
            status.repository_id, status.resource_type, timestamp
        );
        let value = serde_json::to_vec(status)?;

        tree.insert(key.as_bytes(), value)?;
        tree.flush()?;

        Ok(())
    }

    // Cross-reference operations
    /// Adds a cross-reference between issues or pull requests.
    ///
    /// # Arguments
    ///
    /// * `cross_ref` - The CrossReference to add
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail.
    pub fn add_cross_reference(&self, cross_ref: &CrossReference) -> Result<()> {
        let tree = self.db.open_tree("cross_references")?;

        // Store with source key
        let source_key = format!(
            "xref_source:{}:{}:{}",
            cross_ref.source_repository_id, cross_ref.source_type, cross_ref.source_id
        );

        // Store with target key for bidirectional lookup
        let target_key = format!(
            "xref_target:{}:{}:{}",
            cross_ref.target_repository_id, cross_ref.target_type, cross_ref.target_number
        );

        let value = serde_json::to_vec(cross_ref)?;

        tree.insert(source_key.as_bytes(), value.clone())?;
        tree.insert(target_key.as_bytes(), value)?;
        tree.flush()?;

        Ok(())
    }

    /// Retrieves cross-references originating from a specific source item.
    ///
    /// # Arguments
    ///
    /// * `repository_id` - The repository ID of the source item
    /// * `item_type` - The type of the source item (Issue or PullRequest)
    /// * `item_id` - The ID of the source item
    ///
    /// # Returns
    ///
    /// Returns a Vec of CrossReference entries from this source.
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail.
    pub fn get_cross_references_by_source(
        &self,
        repository_id: RepositoryId,
        item_type: ItemType,
        item_id: i64,
    ) -> Result<Vec<CrossReference>> {
        let tree = self.db.open_tree("cross_references")?;

        let prefix = format!("xref_source:{}:{}:{}", repository_id, item_type, item_id);
        let mut refs = Vec::new();

        for item in tree.scan_prefix(prefix.as_bytes()) {
            let (_, value) = item?;
            let cross_ref: CrossReference = serde_json::from_slice(&value)?;
            refs.push(cross_ref);
        }

        Ok(refs)
    }

    /// Retrieves cross-references pointing to a specific target item.
    ///
    /// # Arguments
    ///
    /// * `repository_id` - The repository ID of the target item
    /// * `item_type` - The type of the target item (Issue or PullRequest)
    /// * `item_number` - The number of the target item
    ///
    /// # Returns
    ///
    /// Returns a Vec of CrossReference entries pointing to this target.
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail.
    pub fn get_cross_references_by_target(
        &self,
        repository_id: RepositoryId,
        item_type: ItemType,
        item_number: i64,
    ) -> Result<Vec<CrossReference>> {
        let tree = self.db.open_tree("cross_references")?;

        let prefix = format!(
            "xref_target:{}:{}:{}",
            repository_id, item_type, item_number
        );
        let mut refs = Vec::new();

        for item in tree.scan_prefix(prefix.as_bytes()) {
            let (_, value) = item?;
            let cross_ref: CrossReference = serde_json::from_slice(&value)?;
            refs.push(cross_ref);
        }

        Ok(refs)
    }

    // Search operations
    /// Searches for issues and pull requests using full-text search.
    ///
    /// # Arguments
    ///
    /// * `query` - The search query string
    /// * `repository_id` - Optional repository ID to filter results
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    ///
    /// Returns a Vec of SearchResult entries matching the query.
    ///
    /// # Errors
    ///
    /// Returns an error if search operations fail.
    pub async fn search(
        &self,
        query: &str,
        repository_id: Option<RepositoryId>,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let searcher = self.index_reader.searcher();
        let schema = self.search_index.schema();

        let title_field = schema.get_field("title").unwrap();
        let body_field = schema.get_field("body").unwrap();

        let query_parser =
            QueryParser::for_index(&self.search_index, vec![title_field, body_field]);
        let query = query_parser.parse_query(query)?;

        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let mut results = Vec::new();

        for (_score, doc_address) in top_docs {
            let retrieved_doc = searcher.doc::<tantivy::TantivyDocument>(doc_address)?;

            let doc_type = retrieved_doc
                .get_first(schema.get_field("type").unwrap())
                .and_then(|v| v.as_str())
                .unwrap_or_default();

            let doc_id = retrieved_doc
                .get_first(schema.get_field("id").unwrap())
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);

            let repo_id = retrieved_doc
                .get_first(schema.get_field("repository_id").unwrap())
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<i64>().ok())
                .map(RepositoryId::new)
                .unwrap_or_else(|| RepositoryId::new(0));

            // Filter by repository if specified
            if let Some(filter_repo_id) = repository_id {
                if repo_id != filter_repo_id {
                    continue;
                }
            }

            let title = retrieved_doc
                .get_first(schema.get_field("title").unwrap())
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            let body = retrieved_doc
                .get_first(schema.get_field("body").unwrap())
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            results.push(SearchResult {
                id: doc_id,
                result_type: doc_type.to_string(),
                repository_id: repo_id,
                title,
                body,
            });
        }

        Ok(results)
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: i64,
    pub result_type: String,
    pub repository_id: RepositoryId,
    pub title: String,
    pub body: Option<String>,
}
