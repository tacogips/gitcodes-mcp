use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tantivy::{
    collector::TopDocs,
    query::QueryParser,
    schema::*,
    doc, Index, IndexReader, IndexWriter, Score, TantivyDocument,
};

/// Enhanced search functionality that provides improved features
/// using tantivy as the underlying engine
pub struct EnhancedSearch {
    index: Index,
    reader: IndexReader,
    schema: Schema,
    field_mapping: HashMap<String, Field>,
}

/// Configuration for enhanced search features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Enable fuzzy matching for typo tolerance
    pub fuzzy_matching: bool,
    /// Maximum edit distance for fuzzy matching (Levenshtein distance)
    pub fuzzy_distance: u8,
    /// Enable phrase search
    pub phrase_search: bool,
    /// Enable wildcard search
    pub wildcard_search: bool,
    /// Boost scores for specific fields
    pub field_boosts: HashMap<String, f32>,
}

impl Default for SearchConfig {
    fn default() -> Self {
        let mut field_boosts = HashMap::new();
        field_boosts.insert("title".to_string(), 2.0);
        field_boosts.insert("name".to_string(), 2.0);
        field_boosts.insert("full_name".to_string(), 1.5);
        
        Self {
            fuzzy_matching: true,
            fuzzy_distance: 2,
            phrase_search: true,
            wildcard_search: true,
            field_boosts,
        }
    }
}

/// Search query builder that supports various search modes
#[derive(Debug, Clone)]
pub struct SearchQueryBuilder {
    query_text: String,
    fields: Vec<String>,
    config: SearchConfig,
}

impl SearchQueryBuilder {
    pub fn new(query_text: String) -> Self {
        Self {
            query_text,
            fields: vec![],
            config: SearchConfig::default(),
        }
    }
    
    pub fn with_fields(mut self, fields: Vec<String>) -> Self {
        self.fields = fields;
        self
    }
    
    pub fn with_config(mut self, config: SearchConfig) -> Self {
        self.config = config;
        self
    }
}

/// Result of a search operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub score: f32,
    pub highlights: HashMap<String, Vec<String>>,
    pub metadata: serde_json::Value,
}

impl EnhancedSearch {
    pub fn new(index: Index) -> Result<Self> {
        let reader = index
            .reader_builder()
            .try_into()?;
        
        let schema = index.schema();
        let mut field_mapping = HashMap::new();
        
        // Map field names to field handles
        for (field, field_entry) in schema.fields() {
            field_mapping.insert(field_entry.name().to_string(), field);
        }
        
        Ok(Self {
            index,
            reader,
            schema,
            field_mapping,
        })
    }
    
    /// Perform a search with enhanced features
    pub fn search(&self, query_builder: SearchQueryBuilder, limit: usize) -> Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();
        
        // Build query parser with all indexed fields
        let indexed_fields: Vec<Field> = self.field_mapping.values().copied().collect();
        let query_parser = QueryParser::for_index(&self.index, indexed_fields);
        
        // Parse the query
        let query = query_parser
            .parse_query(&query_builder.query_text)
            .map_err(|e| anyhow!("Failed to parse query: {}", e))?;
        
        // Execute search
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;
        
        // Process results
        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher.doc(doc_address)?;
            let result = self.process_document(&doc, score, &query_builder)?;
            results.push(result);
        }
        
        Ok(results)
    }
    
    /// Process a document into a search result
    fn process_document(
        &self,
        doc: &TantivyDocument,
        score: Score,
        query_builder: &SearchQueryBuilder,
    ) -> Result<SearchResult> {
        let mut metadata = serde_json::Map::new();
        let mut highlights = HashMap::new();
        let mut id = String::new();
        
        // Extract fields from document
        for (field, field_entry) in self.schema.fields() {
            let field_name = field_entry.name();
            
            // Get field value
            if let Some(field_value) = doc.get_first(field) {
                if field_name == "id" {
                    if let Some(text) = field_value.as_str() {
                        id = text.to_string();
                    }
                }
                
                // Convert field value to JSON based on type
                if let Some(text) = field_value.as_str() {
                    metadata.insert(field_name.to_string(), serde_json::Value::String(text.to_string()));
                    
                    // Simple highlight extraction for matching fields
                    if query_builder.query_text.split_whitespace().any(|term| {
                        text.to_lowercase().contains(&term.to_lowercase())
                    }) {
                        highlights.entry(field_name.to_string())
                            .or_insert_with(Vec::new)
                            .push(text.to_string());
                    }
                } else if let Some(n) = field_value.as_u64() {
                    metadata.insert(field_name.to_string(), serde_json::Value::Number(n.into()));
                } else if let Some(n) = field_value.as_i64() {
                    metadata.insert(field_name.to_string(), serde_json::Value::Number(n.into()));
                } else if let Some(b) = field_value.as_bool() {
                    metadata.insert(field_name.to_string(), serde_json::Value::Bool(b));
                }
            }
        }
        
        Ok(SearchResult {
            id,
            score,
            highlights,
            metadata: serde_json::Value::Object(metadata),
        })
    }
    
    /// Create a writer for indexing operations
    pub fn writer(&self) -> Result<IndexWriter> {
        Ok(self.index.writer(100_000_000)?)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_search_config_default() {
        let config = SearchConfig::default();
        assert!(config.fuzzy_matching);
        assert_eq!(config.fuzzy_distance, 2);
        assert!(config.field_boosts.contains_key("title"));
    }
    
    #[test]
    fn test_search_query_builder() {
        let builder = SearchQueryBuilder::new("test query".to_string())
            .with_fields(vec!["title".to_string(), "body".to_string()]);
        
        assert_eq!(builder.query_text, "test query");
        assert_eq!(builder.fields.len(), 2);
    }
}