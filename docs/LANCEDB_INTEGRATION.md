# LanceDB Integration Guide

This document explains the LanceDB integration approach for gitdb and provides guidance on setting up and using LanceDB for full-text and hybrid search functionality.

## Overview

LanceDB is a serverless vector database that provides:
- Native full-text search using BM25 algorithm
- Vector search capabilities for semantic search
- Hybrid search combining both approaches
- Efficient columnar storage using Apache Arrow format

## Current Implementation Status

### Completed
1. **LanceDB Storage Module** (`src/storage/lancedb_store.rs`)
   - Complete data model implementation for all GitHub entity types
   - Full-text search index creation on relevant fields
   - Search functionality using LanceDB's native FTS

2. **Enhanced Search Module** (`src/storage/enhanced_search.rs`)
   - Enhanced tantivy-based search with LanceDB-like features
   - Fuzzy matching, phrase search, and field boosting
   - Preparation for future hybrid search implementation

### Prerequisites for LanceDB

**IMPORTANT**: LanceDB requires `protoc` (Protocol Buffers compiler) to be installed on your system.

#### Installing protoc

**On Ubuntu/Debian:**
```bash
sudo apt-get update
sudo apt-get install protobuf-compiler
```

**On macOS:**
```bash
brew install protobuf
```

**On Windows:**
Download from https://github.com/protocolbuffers/protobuf/releases

**Using Nix:**
```bash
nix-env -iA nixpkgs.protobuf
```

## Architecture Design

### 1. Dual Storage Approach

The implementation provides two storage backends:
- **Tantivy** (current): Lightweight, no external dependencies
- **LanceDB** (optional): Advanced features, requires protoc

### 2. Unified Search Interface

Both backends implement a common search interface:
```rust
pub trait SearchBackend {
    async fn search(&self, query: SearchQuery, limit: usize) -> Result<Vec<SearchResult>>;
    async fn index_document(&self, doc: Document) -> Result<()>;
}
```

### 3. Feature Flags

To enable LanceDB support, add to `Cargo.toml`:
```toml
[features]
default = ["tantivy-search"]
tantivy-search = []
lancedb-search = ["lancedb", "arrow", "arrow-array", "arrow-schema"]
```

## LanceDB Advantages

1. **Native Full-Text Search**
   - BM25 algorithm built-in
   - No need for separate search index
   - Immediate searchability of new records

2. **Hybrid Search Ready**
   - Combine keyword and semantic search
   - Reranking strategies (RRF, linear combination)
   - Better relevance for complex queries

3. **Efficient Storage**
   - Columnar format optimized for analytics
   - Better compression than row-based storage
   - Fast aggregations and filtering

4. **Scalability**
   - Handles datasets up to hundreds of terabytes
   - Supports billion-scale vector operations
   - Serverless architecture

## Migration Path

### Phase 1: Enhanced Tantivy (Current)
- Improve existing tantivy search with fuzzy matching
- Add field boosting and advanced query features
- Prepare for hybrid search architecture

### Phase 2: Optional LanceDB Backend
- Add feature flag for LanceDB
- Implement storage abstraction layer
- Allow users to choose backend

### Phase 3: Hybrid Search
- Add vector embedding generation
- Implement semantic search
- Combine with keyword search using reranking

## Usage Examples

### Using Enhanced Tantivy Search
```rust
use gitdb::storage::{EnhancedSearch, SearchQueryBuilder, SearchConfig};

// Configure search
let config = SearchConfig {
    fuzzy_matching: true,
    fuzzy_distance: 2,
    phrase_search: true,
    ..Default::default()
};

// Build query
let query = SearchQueryBuilder::new("rust async tokio".to_string())
    .with_fields(vec!["title".to_string(), "body".to_string()])
    .with_config(config)
    .with_filter(FilterCondition::Equals {
        field: "state".to_string(),
        value: "open".to_string(),
    });

// Execute search
let results = search.search(query, 10).await?;
```

### Using LanceDB (when enabled)
```rust
use gitdb::storage::LanceDbStore;

// Initialize store
let store = LanceDbStore::new(data_dir).await?;

// Search repositories
let repos = store.search_repositories("rust http client", 10).await?;

// Search with filters (future implementation)
let query = HybridSearchQuery {
    text_query: Some("async runtime".to_string()),
    vector_query: None, // Will be added with embeddings
    rerank_strategy: RerankStrategy::RRF { k: 60.0 },
};
```

## Performance Considerations

### Tantivy
- Lower memory footprint
- Faster for pure text search
- No external dependencies
- Good for smaller datasets (<1GB)

### LanceDB
- Better for large datasets (>1GB)
- Efficient for complex queries
- Native support for analytics
- Future-proof for vector search

## Recommendations

1. **For Development**: Use tantivy (no setup required)
2. **For Production with Simple Search**: Use enhanced tantivy
3. **For Production with Advanced Search**: Use LanceDB
4. **For Future Semantic Search**: Prepare for LanceDB migration

## Next Steps

1. Implement feature flags for backend selection
2. Add vector embedding generation using local models
3. Implement hybrid search with reranking
4. Add benchmarks comparing both backends
5. Create migration tool from tantivy to LanceDB

## References

- [LanceDB Documentation](https://lancedb.github.io/lancedb/)
- [LanceDB Rust SDK](https://docs.rs/lancedb/latest/lancedb/)
- [Full-Text Search in LanceDB](https://lancedb.github.io/lancedb/fts/)
- [Hybrid Search Guide](https://lancedb.github.io/lancedb/hybrid_search/hybrid_search/)