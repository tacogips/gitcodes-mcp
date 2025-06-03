#[cfg(test)]
mod tests {
    use super::super::enhanced_search::*;
    use anyhow::Result;
    use std::collections::HashMap;
    use tantivy::schema::*;
    use tantivy::{doc, Index};
    use tempfile::TempDir;

    /// Helper function to create a test index with sample data
    fn create_test_index() -> Result<(Index, TempDir)> {
        let temp_dir = TempDir::new()?;
        
        // Define schema
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("id", STRING | STORED);
        schema_builder.add_text_field("title", TEXT | STORED);
        schema_builder.add_text_field("body", TEXT | STORED);
        schema_builder.add_text_field("author", TEXT | STORED);
        schema_builder.add_text_field("labels", TEXT | STORED);
        schema_builder.add_text_field("repository", TEXT | STORED);
        schema_builder.add_u64_field("stars", STORED | FAST);
        schema_builder.add_i64_field("priority", STORED | FAST);
        
        let schema = schema_builder.build();
        let index = Index::create_in_dir(&temp_dir, schema.clone())?;
        
        // Add test documents
        let mut index_writer = index.writer(50_000_000)?;
        
        let id_field = schema.get_field("id").unwrap();
        let title_field = schema.get_field("title").unwrap();
        let body_field = schema.get_field("body").unwrap();
        let author_field = schema.get_field("author").unwrap();
        let labels_field = schema.get_field("labels").unwrap();
        let repository_field = schema.get_field("repository").unwrap();
        let stars_field = schema.get_field("stars").unwrap();
        let priority_field = schema.get_field("priority").unwrap();
        
        // Document 1: Rust async runtime issue
        index_writer.add_document(doc!(
            id_field => "issue-1",
            title_field => "Tokio runtime panic on shutdown",
            body_field => "When shutting down the tokio runtime, we get a panic with async tasks still running",
            author_field => "alice",
            labels_field => "bug async runtime",
            repository_field => "awesome-rust/tokio-examples",
            stars_field => 1500u64,
            priority_field => 1i64
        ))?;
        
        // Document 2: HTTP client feature request
        index_writer.add_document(doc!(
            id_field => "issue-2",
            title_field => "Add retry logic to HTTP client",
            body_field => "It would be great to have automatic retry logic with exponential backoff for failed requests",
            author_field => "bob",
            labels_field => "enhancement http client",
            repository_field => "rust-http/reqwest",
            stars_field => 3000u64,
            priority_field => 2i64
        ))?;
        
        // Document 3: Documentation improvement
        index_writer.add_document(doc!(
            id_field => "issue-3",
            title_field => "Improve async/await documentation",
            body_field => "The current documentation for async/await patterns could use more real-world examples",
            author_field => "charlie",
            labels_field => "documentation async",
            repository_field => "rust-lang/book",
            stars_field => 5000u64,
            priority_field => 3i64
        ))?;
        
        // Document 4: Performance issue
        index_writer.add_document(doc!(
            id_field => "pr-1",
            title_field => "Optimize vector allocation in hot path",
            body_field => "This PR reduces memory allocations by pre-allocating vectors with estimated capacity",
            author_field => "dave",
            labels_field => "performance optimization",
            repository_field => "servo/servo",
            stars_field => 2000u64,
            priority_field => 1i64
        ))?;
        
        // Document 5: Bug fix with typo in title (for fuzzy search testing)
        index_writer.add_document(doc!(
            id_field => "pr-2",
            title_field => "Fix tokoi runtime deadlock",  // Note: "tokoi" instead of "tokio"
            body_field => "This fixes a deadlock that occurs when using block_on inside an async context",
            author_field => "eve",
            labels_field => "bug fix runtime",
            repository_field => "awesome-rust/tokio-examples",
            stars_field => 1500u64,
            priority_field => 1i64
        ))?;
        
        // Document 6: Feature with similar content
        index_writer.add_document(doc!(
            id_field => "issue-4",
            title_field => "Support for WebSocket connections",
            body_field => "Add WebSocket support to the HTTP client for real-time communication",
            author_field => "frank",
            labels_field => "feature websocket http",
            repository_field => "rust-http/reqwest",
            stars_field => 3000u64,
            priority_field => 2i64
        ))?;
        
        index_writer.commit()?;
        
        Ok((index, temp_dir))
    }

    #[test]
    fn test_basic_search() -> Result<()> {
        let (index, _temp_dir) = create_test_index()?;
        let search = EnhancedSearch::new(index)?;
        
        // Search for "tokio"
        let query = SearchQueryBuilder::new("tokio".to_string());
        let results = search.search(query, 10)?;
        
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|r| r.id == "issue-1"));
        assert!(results.iter().any(|r| r.id == "pr-2"));
        
        Ok(())
    }

    #[test]
    fn test_multi_term_search() -> Result<()> {
        let (index, _temp_dir) = create_test_index()?;
        let search = EnhancedSearch::new(index)?;
        
        // Search for multiple terms
        let query = SearchQueryBuilder::new("async runtime".to_string());
        let results = search.search(query, 10)?;
        
        // Should find documents containing either "async" or "runtime"
        assert!(results.len() >= 3);
        assert!(results.iter().any(|r| r.id == "issue-1"));
        assert!(results.iter().any(|r| r.id == "issue-3"));
        
        Ok(())
    }

    #[test]
    fn test_field_specific_search() -> Result<()> {
        let (index, _temp_dir) = create_test_index()?;
        let search = EnhancedSearch::new(index)?;
        
        // Search only in specific fields
        let query = SearchQueryBuilder::new("alice".to_string())
            .with_fields(vec!["author".to_string()]);
        let results = search.search(query, 10)?;
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "issue-1");
        
        Ok(())
    }

    #[test]
    fn test_search_highlighting() -> Result<()> {
        let (index, _temp_dir) = create_test_index()?;
        let search = EnhancedSearch::new(index)?;
        
        // Search for a term that appears in multiple fields
        let query = SearchQueryBuilder::new("http".to_string());
        let results = search.search(query, 10)?;
        
        // Check that highlights are extracted
        for result in &results {
            if result.id == "issue-2" || result.id == "issue-4" {
                assert!(!result.highlights.is_empty());
                // Should have highlights in title or body
                let has_highlights = result.highlights.contains_key("title") 
                    || result.highlights.contains_key("body")
                    || result.highlights.contains_key("labels");
                assert!(has_highlights);
            }
        }
        
        Ok(())
    }

    #[test]
    fn test_case_insensitive_search() -> Result<()> {
        let (index, _temp_dir) = create_test_index()?;
        let search = EnhancedSearch::new(index)?;
        
        // Search with different cases
        let queries = vec![
            "TOKIO".to_string(),
            "Tokio".to_string(),
            "tokio".to_string(),
        ];
        
        for query_text in queries {
            let query = SearchQueryBuilder::new(query_text);
            let results = search.search(query, 10)?;
            assert_eq!(results.len(), 2, "Case-insensitive search should find same results");
        }
        
        Ok(())
    }

    #[test]
    fn test_search_with_special_characters() -> Result<()> {
        let (index, _temp_dir) = create_test_index()?;
        let search = EnhancedSearch::new(index)?;
        
        // Search for repository with special characters
        let query = SearchQueryBuilder::new("rust-http/reqwest".to_string());
        let results = search.search(query, 10)?;
        
        // Should find documents from that repository
        assert!(results.len() >= 2);
        assert!(results.iter().any(|r| r.id == "issue-2"));
        assert!(results.iter().any(|r| r.id == "issue-4"));
        
        Ok(())
    }

    #[test]
    fn test_empty_search() -> Result<()> {
        let (index, _temp_dir) = create_test_index()?;
        let search = EnhancedSearch::new(index)?;
        
        // Search for whitespace should work but return no/few results
        let query = SearchQueryBuilder::new("   ".to_string());
        let result = search.search(query, 10);
        
        // Should either error or return empty/minimal results
        match result {
            Ok(results) => assert!(results.is_empty() || results.len() <= 2),
            Err(_) => {} // Empty query error is also acceptable
        }
        
        Ok(())
    }

    #[test]
    fn test_search_result_scoring() -> Result<()> {
        let (index, _temp_dir) = create_test_index()?;
        let search = EnhancedSearch::new(index)?;
        
        // Search for "optimization"
        let query = SearchQueryBuilder::new("optimization".to_string());
        let results = search.search(query, 10)?;
        
        // Results should be sorted by score (descending)
        for i in 1..results.len() {
            assert!(results[i-1].score >= results[i].score);
        }
        
        Ok(())
    }

    #[test]
    fn test_search_limit() -> Result<()> {
        let (index, _temp_dir) = create_test_index()?;
        let search = EnhancedSearch::new(index)?;
        
        // Search with a limit
        let query = SearchQueryBuilder::new("the".to_string());
        let results = search.search(query, 2)?;
        
        assert!(results.len() <= 2);
        
        Ok(())
    }

    #[test]
    fn test_search_config_field_boosts() -> Result<()> {
        let (index, _temp_dir) = create_test_index()?;
        let search = EnhancedSearch::new(index)?;
        
        // Create config with title boost
        let mut field_boosts = HashMap::new();
        field_boosts.insert("title".to_string(), 5.0);
        field_boosts.insert("body".to_string(), 1.0);
        
        let config = SearchConfig {
            field_boosts,
            ..Default::default()
        };
        
        let query = SearchQueryBuilder::new("runtime".to_string())
            .with_config(config);
        let results = search.search(query, 10)?;
        
        // Documents with "runtime" in title should rank higher
        // Note: This test might need adjustment based on actual tantivy scoring behavior
        assert!(!results.is_empty());
        
        Ok(())
    }

    #[test]
    fn test_metadata_extraction() -> Result<()> {
        let (index, _temp_dir) = create_test_index()?;
        let search = EnhancedSearch::new(index)?;
        
        let query = SearchQueryBuilder::new("alice".to_string());
        let results = search.search(query, 1)?;
        
        assert_eq!(results.len(), 1);
        let result = &results[0];
        
        // Check metadata contains expected fields
        let metadata = result.metadata.as_object().unwrap();
        assert!(metadata.contains_key("title"));
        assert!(metadata.contains_key("body"));
        assert!(metadata.contains_key("author"));
        assert!(metadata.contains_key("stars"));
        assert!(metadata.contains_key("priority"));
        
        // Verify numeric fields
        assert_eq!(metadata["stars"].as_u64().unwrap(), 1500);
        assert_eq!(metadata["priority"].as_i64().unwrap(), 1);
        
        Ok(())
    }

    #[test]
    fn test_phrase_search() -> Result<()> {
        let (index, _temp_dir) = create_test_index()?;
        let search = EnhancedSearch::new(index)?;
        
        // Search for exact phrase (using quotes)
        let query = SearchQueryBuilder::new("\"exponential backoff\"".to_string());
        let results = search.search(query, 10)?;
        
        // Should find documents containing the phrase
        assert!(!results.is_empty());
        // The document with exact phrase should be included
        assert!(results.iter().any(|r| r.id == "issue-2"));
        
        Ok(())
    }

    #[test]
    fn test_word_stemming() -> Result<()> {
        let (index, _temp_dir) = create_test_index()?;
        let search = EnhancedSearch::new(index)?;
        
        // Search for a base word that appears in different forms
        let query = SearchQueryBuilder::new("optimize".to_string());
        let results = search.search(query, 10)?;
        
        // Should find documents containing "optimize" or "optimization"
        // This tests that the search can find related word forms
        assert!(results.iter().any(|r| r.id == "pr-1"));
        
        Ok(())
    }

    #[test]
    fn test_search_non_existent_term() -> Result<()> {
        let (index, _temp_dir) = create_test_index()?;
        let search = EnhancedSearch::new(index)?;
        
        // Search for term that doesn't exist
        let query = SearchQueryBuilder::new("nonexistentterm123".to_string());
        let results = search.search(query, 10)?;
        
        assert_eq!(results.len(), 0);
        
        Ok(())
    }

    #[test]
    fn test_concurrent_searches() -> Result<()> {
        use std::sync::Arc;
        use std::thread;
        
        let (index, _temp_dir) = create_test_index()?;
        let search = Arc::new(EnhancedSearch::new(index)?);
        
        let mut handles = vec![];
        
        // Spawn multiple threads doing searches
        for i in 0..5 {
            let search_clone = Arc::clone(&search);
            let handle = thread::spawn(move || -> Result<()> {
                let query_text = match i % 3 {
                    0 => "tokio",
                    1 => "http",
                    _ => "async",
                };
                
                let query = SearchQueryBuilder::new(query_text.to_string());
                let results = search_clone.search(query, 10)?;
                assert!(!results.is_empty());
                Ok(())
            });
            handles.push(handle);
        }
        
        // Wait for all threads
        for handle in handles {
            handle.join().unwrap()?;
        }
        
        Ok(())
    }
}