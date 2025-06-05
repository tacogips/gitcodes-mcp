use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use arrow_array::{
    ArrayRef, FixedSizeListArray, Float32Array, Int64Array, RecordBatch, RecordBatchIterator, StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use futures::StreamExt;
use lancedb::index::scalar::{FtsIndexBuilder, FullTextSearchQuery};
use lancedb::index::vector::IvfPqIndexBuilder;
use lancedb::index::Index;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::{connect, Connection};
use serde::{Deserialize, Serialize};

use crate::types::FullId;
use crate::types::{
    GitHubComment, GitHubIssue, GitHubPullRequest, GitHubPullRequestFile, 
    GitHubRepository, GitHubUser,
};

const REPOSITORIES_TABLE: &str = "repositories";
const ISSUES_TABLE: &str = "issues";
const PULL_REQUESTS_TABLE: &str = "pull_requests";
const COMMENTS_TABLE: &str = "comments";
const USERS_TABLE: &str = "users";
const FILES_TABLE: &str = "pull_request_files";

// Vector embedding dimension - common sizes are 384 (all-MiniLM-L6-v2), 768 (BERT), 1536 (OpenAI)
const EMBEDDING_DIMENSION: i32 = 384;

pub struct SearchStore {
    connection: Connection,
    data_dir: PathBuf,
}

impl SearchStore {
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
            Field::new("searchable_content", DataType::Utf8, false), // Combined searchable text
            Field::new("embedding", DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                EMBEDDING_DIMENSION
            ), true), // Vector embedding of searchable_content
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

        // Don't create FTS index immediately - will create it after data is added

        // Create vector index on embedding field
        // Skip vector index creation in test environment to avoid empty table issues
        if std::env::var("GITDB_SKIP_VECTOR_INDEX").is_err() {
            // Only create vector index if not in test mode
            // LanceDB requires data to create IvfPq index with many partitions
            match table
                .create_index(
                    &["embedding"],
                    Index::IvfPq(IvfPqIndexBuilder::default()
                        .num_partitions(10)  // Reduced from 100 for smaller datasets
                        .num_sub_vectors(4)), // Reduced from 16 for smaller datasets
                )
                .execute()
                .await
            {
                Ok(_) => {},
                Err(e) => {
                    // Log but don't fail if vector index creation fails
                    eprintln!("Warning: Failed to create vector index on repositories: {}. Vector search may be slower.", e);
                }
            }
        }

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
            Field::new("searchable_content", DataType::Utf8, false), // Combined searchable text
            Field::new("embedding", DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                EMBEDDING_DIMENSION
            ), true), // Vector embedding of searchable_content
            Field::new("data", DataType::Utf8, false), // Full JSON data
        ]));

        let batch = RecordBatch::new_empty(schema.clone());
        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema.clone());

        let table = self
            .connection
            .create_table(ISSUES_TABLE, Box::new(batches))
            .execute()
            .await?;

        // Don't create FTS index immediately - will create it after data is added

        // Create vector index on embedding field
        // Skip vector index creation in test environment to avoid empty table issues
        if std::env::var("GITDB_SKIP_VECTOR_INDEX").is_err() {
            // Only create vector index if not in test mode
            // LanceDB requires data to create IvfPq index with many partitions
            match table
                .create_index(
                    &["embedding"],
                    Index::IvfPq(IvfPqIndexBuilder::default()
                        .num_partitions(10)  // Reduced from 100 for smaller datasets
                        .num_sub_vectors(4)), // Reduced from 16 for smaller datasets
                )
                .execute()
                .await
            {
                Ok(_) => {},
                Err(e) => {
                    // Log but don't fail if vector index creation fails
                    eprintln!("Warning: Failed to create vector index on repositories: {}. Vector search may be slower.", e);
                }
            }
        }

        Ok(())
    }

    /// Create or update the FTS index for repositories table
    pub async fn create_fts_index_repositories(&self) -> Result<()> {
        let table = self.connection.open_table(REPOSITORIES_TABLE).execute().await?;
        
        match table
            .create_index(
                &["searchable_content"],
                Index::FTS(FtsIndexBuilder::default()),
            )
            .execute()
            .await
        {
            Ok(_) => {},
            Err(e) => return Err(anyhow!("Failed to create FTS index on repositories: {}", e)),
        }
        
        Ok(())
    }

    /// Create or update the FTS index for issues table
    pub async fn create_fts_index_issues(&self) -> Result<()> {
        let table = self.connection.open_table(ISSUES_TABLE).execute().await?;
        
        match table
            .create_index(
                &["searchable_content"],
                Index::FTS(FtsIndexBuilder::default()),
            )
            .execute()
            .await
        {
            Ok(_) => {},
            Err(e) => return Err(anyhow!("Failed to create FTS index on issues: {}", e)),
        }
        
        Ok(())
    }

    pub async fn save_repository(&self, repo: &GitHubRepository) -> Result<()> {
        let table = self.connection.open_table(REPOSITORIES_TABLE).execute().await?;
        
        // Build searchable_content field combining all searchable content
        let mut searchable_content_parts = vec![
            repo.full_name.clone(),
            repo.name.clone(),
            repo.owner.clone(),
        ];
        
        if let Some(desc) = &repo.description {
            searchable_content_parts.push(desc.clone());
        }
        
        if let Some(lang) = &repo.language {
            searchable_content_parts.push(lang.clone());
        }
        
        // Add topics
        for topic in &repo.topics {
            searchable_content_parts.push(topic.clone());
        }
        
        let searchable_content = vec![searchable_content_parts.join(" ")];
        
        // TODO: Generate real embeddings from searchable_content using an embedding model
        // For now, create a placeholder embedding vector
        let embedding_vec: Vec<f32> = vec![0.0; EMBEDDING_DIMENSION as usize];
        
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

        // Get the schema from the table
        let schema = table.schema().await?;

        // Create the embedding array
        let embedding_array = Float32Array::from(embedding_vec);
        let field = Arc::new(Field::new("item", DataType::Float32, true));
        let embedding_list = FixedSizeListArray::try_new(field, EMBEDDING_DIMENSION, Arc::new(embedding_array), None)?;

        let batch = RecordBatch::try_new(
            schema,
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
                Arc::new(StringArray::from(searchable_content)),
                Arc::new(embedding_list) as ArrayRef,
                Arc::new(StringArray::from(data)),
            ],
        )?;

        let batches = RecordBatchIterator::new(vec![Ok(batch)], table.schema().await?);
        table.add(batches).execute().await?;
        Ok(())
    }

    pub async fn get_repository(&self, full_id: &FullId) -> Result<Option<GitHubRepository>> {
        let table = self.connection.open_table(REPOSITORIES_TABLE).execute().await?;
        
        let filter = format!("id = '{}'", full_id.to_string());
        let mut results = table
            .query()
            .only_if(filter.as_str())
            .limit(1)
            .execute()
            .await?;

        if let Some(batch_result) = results.next().await {
            let batch = batch_result?;
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

    pub async fn search_repositories(&self, query: &LanceDbQuery) -> Result<Vec<GitHubRepository>> {
        let table = match self.connection.open_table(REPOSITORIES_TABLE).execute().await {
            Ok(table) => table,
            Err(_) => return Ok(Vec::new()), // Table doesn't exist yet
        };
        
        let mut table_query = table.query();
        
        // Apply full-text search
        table_query = table_query.full_text_search(
            FullTextSearchQuery::new(query.text.clone())
                .with_columns(&["searchable_content".to_string()])
                .unwrap()
        );
        
        // Apply filters if specified
        if let Some(filter) = &query.filter {
            table_query = table_query.only_if(filter.as_str());
        }
        
        // Set limit and offset
        let limit = query.limit.unwrap_or(10);
        table_query = table_query.limit(limit);
        
        if let Some(offset) = query.offset {
            table_query = table_query.offset(offset);
        }
        
        // Apply fast search if enabled
        if query.fast_search {
            table_query = table_query.fast_search();
        }
        
        // Apply postfilter if enabled
        if query.postfilter {
            table_query = table_query.postfilter();
        }
        
        let mut results = match table_query.execute().await {
            Ok(results) => results,
            Err(e) => {
                // If FTS index doesn't exist, return empty results
                if e.to_string().contains("no inverted index") {
                    return Ok(Vec::new());
                }
                return Err(e.into());
            }
        };

        let mut repositories = Vec::new();
        while let Some(batch_result) = results.next().await {
            let batch = batch_result?;
            
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

    pub async fn save_issue(&self, issue: &GitHubIssue) -> Result<()> {
        let table = self.connection.open_table(ISSUES_TABLE).execute().await?;
        
        // Build searchable_content field combining all searchable content
        let mut searchable_content_parts = vec![
            issue.title.clone(),
            format!("#{}", issue.number),
        ];
        
        if let Some(body) = &issue.body {
            searchable_content_parts.push(body.clone());
        }
        
        // Add labels
        for label in &issue.labels {
            searchable_content_parts.push(label.name.clone());
        }
        
        // Add assignees
        for assignee in &issue.assignees {
            searchable_content_parts.push(assignee.login.clone());
        }
        
        // Add milestone
        if let Some(milestone) = &issue.milestone {
            searchable_content_parts.push(milestone.title.clone());
        }
        
        let searchable_content = vec![searchable_content_parts.join(" ")];
        
        // TODO: Generate real embeddings from searchable_content using an embedding model
        // For now, create a placeholder embedding vector
        let embedding_vec: Vec<f32> = vec![0.0; EMBEDDING_DIMENSION as usize];
        
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

        // Get the schema from the table
        let schema = table.schema().await?;

        // Create the embedding array
        let embedding_array = Float32Array::from(embedding_vec);
        let field = Arc::new(Field::new("item", DataType::Float32, true));
        let embedding_list = FixedSizeListArray::try_new(field, EMBEDDING_DIMENSION, Arc::new(embedding_array), None)?;

        let batch = RecordBatch::try_new(
            schema,
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
                Arc::new(StringArray::from(searchable_content)),
                Arc::new(embedding_list) as ArrayRef,
                Arc::new(StringArray::from(data)),
            ],
        )?;

        let batches = RecordBatchIterator::new(vec![Ok(batch)], table.schema().await?);
        table.add(batches).execute().await?;
        Ok(())
    }

    pub async fn search_issues(&self, query: &LanceDbQuery) -> Result<Vec<GitHubIssue>> {
        let table = self.connection.open_table(ISSUES_TABLE).execute().await?;
        
        let mut table_query = table.query();
        
        // Apply full-text search
        table_query = table_query.full_text_search(
            FullTextSearchQuery::new(query.text.clone())
                .with_columns(&["searchable_content".to_string()])
                .unwrap()
        );
        
        // Apply filters if specified
        if let Some(filter) = &query.filter {
            table_query = table_query.only_if(filter.as_str());
        }
        
        // Set limit and offset
        let limit = query.limit.unwrap_or(10);
        table_query = table_query.limit(limit);
        
        if let Some(offset) = query.offset {
            table_query = table_query.offset(offset);
        }
        
        // Apply fast search if enabled
        if query.fast_search {
            table_query = table_query.fast_search();
        }
        
        // Apply postfilter if enabled
        if query.postfilter {
            table_query = table_query.postfilter();
        }
        
        let mut results = match table_query.execute().await {
            Ok(results) => results,
            Err(e) => {
                // If FTS index doesn't exist, return empty results
                if e.to_string().contains("no inverted index") {
                    return Ok(Vec::new());
                }
                return Err(e.into());
            }
        };

        let mut issues = Vec::new();
        while let Some(batch_result) = results.next().await {
            let batch = batch_result?;
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

    pub async fn search_all(&self, query: &LanceDbQuery) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();
        let limit = query.limit.unwrap_or(10);

        // Search repositories
        let repos = self.search_repositories(query).await?;
        for repo in repos {
            results.push(SearchResult::Repository(repo));
        }

        // Search issues
        let issues = self.search_issues(query).await?;
        for issue in issues {
            results.push(SearchResult::Issue(issue));
        }

        // Limit total results
        results.truncate(limit);
        Ok(results)
    }

    pub async fn search(&self, query: SearchQuery) -> Result<Vec<SearchResult>> {
        // Build filter clauses
        let mut filters = Vec::new();
        
        if let Some(repo) = &query.repository {
            filters.push(format!("repository_id = '{}'", repo));
        }
        
        if let Some(state) = &query.state {
            filters.push(format!("state = '{}'", state));
        }
        
        if let Some(label) = &query.label {
            // For labels, we might need to search in a labels field or use contains
            filters.push(format!("labels LIKE '%{}%'", label));
        }
        
        // Combine custom filter if provided
        let filter = if !filters.is_empty() {
            let base_filter = filters.join(" AND ");
            if let Some(custom_filter) = &query.filter {
                Some(format!("{} AND {}", base_filter, custom_filter))
            } else {
                Some(base_filter)
            }
        } else {
            query.filter.clone()
        };
        
        // Use the new unified search method with FullText mode by default
        let unified_query = UnifiedSearchQuery {
            text: Some(query.text),
            vector: None,
            search_mode: SearchMode::FullText,
            limit: query.limit,
            offset: query.offset,
            filter,
        };
        
        self.unified_search(unified_query).await
    }

    /// Unified search method supporting full text, semantic, and hybrid search modes
    pub async fn unified_search(&self, query: UnifiedSearchQuery) -> Result<Vec<SearchResult>> {
        let limit = query.limit.unwrap_or(10);
        
        match query.search_mode {
            SearchMode::FullText => {
                // Full text search
                if let Some(text) = query.text {
                    let lance_query = LanceDbQuery {
                        text,
                        limit: query.limit,
                        offset: query.offset,
                        filter: query.filter,
                        search_fields: None,
                        select_fields: None,
                        fast_search: false,
                        postfilter: false,
                    };
                    self.search_all(&lance_query).await
                } else {
                    Err(anyhow!("Text query is required for full text search"))
                }
            }
            SearchMode::Semantic => {
                // Semantic/vector search
                let vector = if let Some(vec) = query.vector {
                    vec
                } else if let Some(text) = &query.text {
                    // Generate embedding from text
                    self.generate_embedding(text).await?
                } else {
                    return Err(anyhow!("Either text or vector is required for semantic search"));
                };
                
                let mut results = Vec::new();
                
                // Search repositories
                let repos = self.vector_search_repositories(
                    vector.clone(),
                    limit,
                    query.filter.clone()
                ).await?;
                for repo in repos {
                    results.push(SearchResult::Repository(repo));
                }
                
                // Search issues
                let issues = self.vector_search_issues(
                    vector,
                    limit,
                    query.filter
                ).await?;
                for issue in issues {
                    results.push(SearchResult::Issue(issue));
                }
                
                // Limit total results
                results.truncate(limit);
                Ok(results)
            }
            SearchMode::Hybrid { rerank_strategy } => {
                // Hybrid search
                let vector = if let Some(vec) = query.vector {
                    Some(vec)
                } else if let Some(text) = &query.text {
                    // Generate embedding from text
                    Some(self.generate_embedding(text).await?)
                } else {
                    None
                };
                
                let hybrid_query = hybrid::HybridSearchQuery {
                    text_query: query.text,
                    vector_query: vector,
                    base_params: LanceDbQuery {
                        text: String::new(), // Will be set by with_text if needed
                        limit: query.limit,
                        offset: query.offset,
                        filter: query.filter,
                        search_fields: None,
                        select_fields: None,
                        fast_search: false,
                        postfilter: false,
                    },
                    rerank_strategy,
                };
                
                hybrid::hybrid_search(self, hybrid_query).await
            }
        }
    }

    /// Perform vector search on repositories using embedding similarity
    pub async fn vector_search_repositories(
        &self, 
        query_vector: Vec<f32>, 
        limit: usize,
        filter: Option<String>
    ) -> Result<Vec<GitHubRepository>> {
        let table = self.connection.open_table(REPOSITORIES_TABLE).execute().await?;
        
        let mut table_query = table
            .vector_search(query_vector)?
            .limit(limit)
            .column("embedding");
        
        // Apply filters if specified
        if let Some(filter) = filter {
            table_query = table_query.only_if(filter.as_str());
        }
        
        let mut results = match table_query.execute().await {
            Ok(results) => results,
            Err(e) => {
                // If FTS index doesn't exist, return empty results
                if e.to_string().contains("no inverted index") {
                    return Ok(Vec::new());
                }
                return Err(e.into());
            }
        };
        
        let mut repositories = Vec::new();
        while let Some(batch_result) = results.next().await {
            let batch = batch_result?;
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

    /// Perform vector search on issues using embedding similarity
    pub async fn vector_search_issues(
        &self, 
        query_vector: Vec<f32>, 
        limit: usize,
        filter: Option<String>
    ) -> Result<Vec<GitHubIssue>> {
        let table = self.connection.open_table(ISSUES_TABLE).execute().await?;
        
        let mut table_query = table
            .vector_search(query_vector)?
            .limit(limit)
            .column("embedding");
        
        // Apply filters if specified
        if let Some(filter) = filter {
            table_query = table_query.only_if(filter.as_str());
        }
        
        let mut results = match table_query.execute().await {
            Ok(results) => results,
            Err(e) => {
                // If FTS index doesn't exist, return empty results
                if e.to_string().contains("no inverted index") {
                    return Ok(Vec::new());
                }
                return Err(e.into());
            }
        };
        
        let mut issues = Vec::new();
        while let Some(batch_result) = results.next().await {
            let batch = batch_result?;
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

    /// Generate embedding for text (placeholder - needs real implementation)
    pub async fn generate_embedding(&self, _text: &str) -> Result<Vec<f32>> {
        // TODO: Integrate with a real embedding model
        // For now, return a placeholder embedding
        Ok(vec![0.0; EMBEDDING_DIMENSION as usize])
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchMode {
    /// Full text search using FTS index
    FullText,
    /// Semantic search using vector embeddings
    Semantic,
    /// Hybrid search combining text and vector with reranking
    Hybrid { rerank_strategy: hybrid::RerankStrategy },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub text: String,
    pub repository: Option<String>,
    pub state: Option<String>,
    pub label: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub filter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedSearchQuery {
    /// Text query for full-text search (required for FullText and Hybrid modes)
    pub text: Option<String>,
    /// Pre-computed vector embedding for semantic search (auto-generated from text if not provided)
    pub vector: Option<Vec<f32>>,
    /// Search mode: FullText, Semantic, or Hybrid
    pub search_mode: SearchMode,
    /// Maximum number of results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
    /// SQL-style filter expression
    pub filter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanceDbQuery {
    pub text: String,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub filter: Option<String>,
    pub search_fields: Option<Vec<String>>,
    pub select_fields: Option<Vec<String>>,
    pub fast_search: bool,
    pub postfilter: bool,
}

impl LanceDbQuery {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            limit: None,
            offset: None,
            filter: None,
            search_fields: None,
            select_fields: None,
            fast_search: false,
            postfilter: false,
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn with_filter(mut self, filter: impl Into<String>) -> Self {
        self.filter = Some(filter.into());
        self
    }

    pub fn with_search_fields(mut self, fields: Vec<String>) -> Self {
        self.search_fields = Some(fields);
        self
    }

    pub fn with_select_fields(mut self, fields: Vec<String>) -> Self {
        self.select_fields = Some(fields);
        self
    }

    pub fn enable_fast_search(mut self) -> Self {
        self.fast_search = true;
        self
    }

    pub fn enable_postfilter(mut self) -> Self {
        self.postfilter = true;
        self
    }
}

impl UnifiedSearchQuery {
    /// Create a new unified search query
    pub fn new(search_mode: SearchMode) -> Self {
        Self {
            text: None,
            vector: None,
            search_mode,
            limit: None,
            offset: None,
            filter: None,
        }
    }

    /// Create a full text search query
    pub fn full_text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            vector: None,
            search_mode: SearchMode::FullText,
            limit: None,
            offset: None,
            filter: None,
        }
    }

    /// Create a semantic search query from text (embedding will be generated)
    pub fn semantic_from_text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            vector: None,
            search_mode: SearchMode::Semantic,
            limit: None,
            offset: None,
            filter: None,
        }
    }

    /// Create a semantic search query from vector
    pub fn semantic_from_vector(vector: Vec<f32>) -> Self {
        Self {
            text: None,
            vector: Some(vector),
            search_mode: SearchMode::Semantic,
            limit: None,
            offset: None,
            filter: None,
        }
    }

    /// Create a hybrid search query
    pub fn hybrid(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            vector: None,
            search_mode: SearchMode::Hybrid { 
                rerank_strategy: hybrid::RerankStrategy::RRF { k: 60.0 } 
            },
            limit: None,
            offset: None,
            filter: None,
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn with_filter(mut self, filter: impl Into<String>) -> Self {
        self.filter = Some(filter.into());
        self
    }

    pub fn with_rerank_strategy(mut self, strategy: hybrid::RerankStrategy) -> Self {
        if let SearchMode::Hybrid { .. } = self.search_mode {
            self.search_mode = SearchMode::Hybrid { rerank_strategy: strategy };
        }
        self
    }
}

pub mod hybrid {
    use super::*;
    use std::collections::HashMap;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum RerankStrategy {
        RRF { k: f32 },
        Linear { text_weight: f32, vector_weight: f32 },
        TextOnly,
        VectorOnly,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct HybridSearchQuery {
        pub text_query: Option<String>,
        pub vector_query: Option<Vec<f32>>,
        pub base_params: LanceDbQuery,
        pub rerank_strategy: RerankStrategy,
    }

    impl HybridSearchQuery {
        pub fn new() -> Self {
            Self {
                text_query: None,
                vector_query: None,
                base_params: LanceDbQuery::new(""),
                rerank_strategy: RerankStrategy::RRF { k: 60.0 },
            }
        }

        pub fn with_text(mut self, text: impl Into<String>) -> Self {
            let text_str = text.into();
            self.text_query = Some(text_str.clone());
            self.base_params.text = text_str;
            self
        }

        pub fn with_vector(mut self, vector: Vec<f32>) -> Self {
            self.vector_query = Some(vector);
            self
        }

        pub fn with_rerank_strategy(mut self, strategy: RerankStrategy) -> Self {
            self.rerank_strategy = strategy;
            self
        }

        pub fn with_filter(mut self, filter: impl Into<String>) -> Self {
            self.base_params = self.base_params.with_filter(filter);
            self
        }

        pub fn with_limit(mut self, limit: usize) -> Self {
            self.base_params = self.base_params.with_limit(limit);
            self
        }
    }

    /// Perform hybrid search combining text and vector search
    pub async fn hybrid_search(
        store: &SearchStore,
        query: HybridSearchQuery,
    ) -> Result<Vec<SearchResult>> {
        let limit = query.base_params.limit.unwrap_or(10);
        
        match query.rerank_strategy {
            RerankStrategy::TextOnly => {
                // Only perform text search
                if let Some(text) = query.text_query {
                    let mut text_query = LanceDbQuery::new(text);
                    text_query.limit = query.base_params.limit;
                    text_query.filter = query.base_params.filter;
                    store.search_all(&text_query).await
                } else {
                    Ok(Vec::new())
                }
            }
            RerankStrategy::VectorOnly => {
                // Only perform vector search
                if let Some(vector) = query.vector_query {
                    let mut results = Vec::new();
                    
                    // Search repositories
                    let repos = store.vector_search_repositories(
                        vector.clone(), 
                        limit, 
                        query.base_params.filter.clone()
                    ).await?;
                    for repo in repos {
                        results.push(SearchResult::Repository(repo));
                    }
                    
                    // Search issues
                    let issues = store.vector_search_issues(
                        vector, 
                        limit, 
                        query.base_params.filter
                    ).await?;
                    for issue in issues {
                        results.push(SearchResult::Issue(issue));
                    }
                    
                    results.truncate(limit);
                    Ok(results)
                } else {
                    Ok(Vec::new())
                }
            }
            RerankStrategy::RRF { k } => {
                // Reciprocal Rank Fusion
                let mut text_results = Vec::new();
                let mut vector_results = Vec::new();
                
                // Get text search results
                if let Some(text) = query.text_query {
                    let mut text_query = LanceDbQuery::new(text);
                    text_query.limit = Some(limit * 2); // Get more results for fusion
                    text_query.filter = query.base_params.filter.clone();
                    text_results = store.search_all(&text_query).await?;
                }
                
                // Get vector search results
                if let Some(vector) = query.vector_query {
                    // Search repositories
                    let repos = store.vector_search_repositories(
                        vector.clone(), 
                        limit * 2, 
                        query.base_params.filter.clone()
                    ).await?;
                    for repo in repos {
                        vector_results.push(SearchResult::Repository(repo));
                    }
                    
                    // Search issues
                    let issues = store.vector_search_issues(
                        vector, 
                        limit * 2, 
                        query.base_params.filter
                    ).await?;
                    for issue in issues {
                        vector_results.push(SearchResult::Issue(issue));
                    }
                }
                
                // Apply RRF scoring
                let mut scores: HashMap<String, f32> = HashMap::new();
                
                // Score text results
                for (rank, result) in text_results.iter().enumerate() {
                    let id = result_to_id(result);
                    let score = 1.0 / (k + rank as f32 + 1.0);
                    *scores.entry(id).or_insert(0.0) += score;
                }
                
                // Score vector results
                for (rank, result) in vector_results.iter().enumerate() {
                    let id = result_to_id(result);
                    let score = 1.0 / (k + rank as f32 + 1.0);
                    *scores.entry(id).or_insert(0.0) += score;
                }
                
                // Combine and sort by score
                let mut all_results: Vec<SearchResult> = text_results;
                for vr in vector_results {
                    if !result_exists(&all_results, &vr) {
                        all_results.push(vr);
                    }
                }
                
                all_results.sort_by(|a, b| {
                    let score_a = scores.get(&result_to_id(a)).unwrap_or(&0.0);
                    let score_b = scores.get(&result_to_id(b)).unwrap_or(&0.0);
                    score_b.partial_cmp(score_a).unwrap()
                });
                
                all_results.truncate(limit);
                Ok(all_results)
            }
            RerankStrategy::Linear { text_weight: _, vector_weight: _ } => {
                // Linear combination of scores
                // Similar to RRF but with weighted combination
                // Implementation would be similar to RRF but with different scoring
                unimplemented!("Linear reranking strategy not yet implemented")
            }
        }
    }

    fn result_to_id(result: &SearchResult) -> String {
        match result {
            SearchResult::Repository(repo) => repo.full_id().to_string(),
            SearchResult::Issue(issue) => issue.full_id().to_string(),
            SearchResult::PullRequest(pr) => pr.full_id().to_string(),
            SearchResult::Comment(comment) => comment.full_id().to_string(),
            SearchResult::User(user) => user.id.to_string(),
            SearchResult::File(file) => format!("{}:{}", file.sha, file.filename),
        }
    }

    fn result_exists(results: &[SearchResult], target: &SearchResult) -> bool {
        let target_id = result_to_id(target);
        results.iter().any(|r| result_to_id(r) == target_id)
    }
}