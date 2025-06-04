#[cfg(feature = "lancedb-backend")]
mod lancedb_query_tests {
    use gitdb::storage::lancedb_store::{LanceDbQuery, hybrid::{HybridSearchQuery, RerankStrategy}};

    #[test]
    fn test_lancedb_query_new() {
        let query = LanceDbQuery::new("test search");
        assert_eq!(query.text, "test search");
        assert_eq!(query.limit, None);
        assert_eq!(query.offset, None);
        assert_eq!(query.filter, None);
        assert_eq!(query.search_fields, None);
        assert_eq!(query.select_fields, None);
        assert!(!query.fast_search);
        assert!(!query.postfilter);
    }

    #[test]
    fn test_lancedb_query_builder() {
        let query = LanceDbQuery::new("rust async")
            .with_limit(20)
            .with_offset(10)
            .with_filter("state = 'open' AND stars > 100")
            .with_search_fields(vec!["title".to_string(), "body".to_string()])
            .with_select_fields(vec!["id".to_string(), "title".to_string(), "state".to_string()])
            .enable_fast_search()
            .enable_postfilter();

        assert_eq!(query.text, "rust async");
        assert_eq!(query.limit, Some(20));
        assert_eq!(query.offset, Some(10));
        assert_eq!(query.filter, Some("state = 'open' AND stars > 100".to_string()));
        assert_eq!(query.search_fields, Some(vec!["title".to_string(), "body".to_string()]));
        assert_eq!(query.select_fields, Some(vec!["id".to_string(), "title".to_string(), "state".to_string()]));
        assert!(query.fast_search);
        assert!(query.postfilter);
    }

    #[test]
    fn test_lancedb_query_serialization() {
        let query = LanceDbQuery::new("search term")
            .with_limit(10)
            .with_filter("language = 'Rust'");

        let json = serde_json::to_string(&query).unwrap();
        let deserialized: LanceDbQuery = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.text, query.text);
        assert_eq!(deserialized.limit, query.limit);
        assert_eq!(deserialized.filter, query.filter);
        assert_eq!(deserialized.fast_search, query.fast_search);
    }

    #[test]
    fn test_hybrid_search_query() {
        let query = HybridSearchQuery::new()
            .with_text("rust tokio")
            .with_vector(vec![0.1, 0.2, 0.3])
            .with_rerank_strategy(RerankStrategy::RRF { k: 30.0 });

        assert_eq!(query.text_query, Some("rust tokio".to_string()));
        assert_eq!(query.vector_query, Some(vec![0.1, 0.2, 0.3]));
        
        match query.rerank_strategy {
            RerankStrategy::RRF { k } => assert_eq!(k, 30.0),
            _ => panic!("Wrong rerank strategy"),
        }
    }

    #[test]
    fn test_rerank_strategies() {
        // Test RRF
        let rrf = RerankStrategy::RRF { k: 60.0 };
        let json = serde_json::to_string(&rrf).unwrap();
        let deserialized: RerankStrategy = serde_json::from_str(&json).unwrap();
        match deserialized {
            RerankStrategy::RRF { k } => assert_eq!(k, 60.0),
            _ => panic!("Wrong strategy"),
        }

        // Test Linear
        let linear = RerankStrategy::Linear { 
            text_weight: 0.7, 
            vector_weight: 0.3 
        };
        let json = serde_json::to_string(&linear).unwrap();
        let deserialized: RerankStrategy = serde_json::from_str(&json).unwrap();
        match deserialized {
            RerankStrategy::Linear { text_weight, vector_weight } => {
                assert_eq!(text_weight, 0.7);
                assert_eq!(vector_weight, 0.3);
            }
            _ => panic!("Wrong strategy"),
        }

        // Test TextOnly and VectorOnly
        let text_only = RerankStrategy::TextOnly;
        let json = serde_json::to_string(&text_only).unwrap();
        assert!(json.contains("TextOnly"));

        let vector_only = RerankStrategy::VectorOnly;
        let json = serde_json::to_string(&vector_only).unwrap();
        assert!(json.contains("VectorOnly"));
    }
}

// For non-lancedb builds, provide a dummy test
#[cfg(not(feature = "lancedb-backend"))]
#[test]
fn test_lancedb_feature_disabled() {
    // This test exists to prevent "no tests found" error when lancedb feature is disabled
    assert!(true, "LanceDB feature is not enabled");
}