use criterion::{black_box, criterion_group, criterion_main, Criterion, BatchSize};
use gitdb::storage::{EnhancedSearch, SearchQueryBuilder, SearchConfig};
use tantivy::schema::*;
use tantivy::{doc, Index};
use tempfile::TempDir;
use std::collections::HashMap;

/// Create a test index with specified number of documents
fn create_benchmark_index(num_docs: usize) -> (Index, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    
    // Define schema
    let mut schema_builder = Schema::builder();
    schema_builder.add_text_field("id", STRING | STORED);
    schema_builder.add_text_field("title", TEXT | STORED);
    schema_builder.add_text_field("body", TEXT | STORED);
    schema_builder.add_text_field("author", TEXT | STORED);
    schema_builder.add_text_field("labels", TEXT | STORED);
    
    let schema = schema_builder.build();
    let index = Index::create_in_dir(&temp_dir, schema.clone()).unwrap();
    
    // Add documents
    let mut index_writer = index.writer(50_000_000).unwrap();
    
    let id_field = schema.get_field("id").unwrap();
    let title_field = schema.get_field("title").unwrap();
    let body_field = schema.get_field("body").unwrap();
    let author_field = schema.get_field("author").unwrap();
    let labels_field = schema.get_field("labels").unwrap();
    
    // Common words for generating content
    let words = vec![
        "async", "runtime", "tokio", "http", "client", "server", "request", "response",
        "error", "handler", "database", "connection", "query", "performance", "optimize",
        "memory", "allocation", "thread", "pool", "executor", "future", "stream", "channel",
        "mutex", "lock", "atomic", "concurrent", "parallel", "sync", "await", "poll"
    ];
    
    let authors = vec!["alice", "bob", "charlie", "dave", "eve", "frank"];
    let labels = vec!["bug", "enhancement", "documentation", "performance", "security"];
    
    for i in 0..num_docs {
        // Generate random content using word combinations
        let title_words: Vec<&str> = (0..5)
            .map(|_| words[i % words.len()])
            .collect();
        let body_words: Vec<&str> = (0..20)
            .map(|j| words[(i + j) % words.len()])
            .collect();
        
        index_writer.add_document(doc!(
            id_field => format!("doc-{}", i),
            title_field => title_words.join(" "),
            body_field => body_words.join(" "),
            author_field => authors[i % authors.len()],
            labels_field => labels[i % labels.len()]
        )).unwrap();
    }
    
    index_writer.commit().unwrap();
    (index, temp_dir)
}

fn benchmark_basic_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("basic_search");
    
    // Test with different index sizes
    for &num_docs in &[100, 1000, 10000] {
        group.bench_function(format!("search_{}_docs", num_docs), |b| {
            let (index, _temp_dir) = create_benchmark_index(num_docs);
            let search = EnhancedSearch::new(index).unwrap();
            
            b.iter(|| {
                let query = SearchQueryBuilder::new("async runtime".to_string());
                let results = search.search(query, 10).unwrap();
                black_box(results);
            });
        });
    }
    
    group.finish();
}

fn benchmark_field_specific_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("field_specific_search");
    
    let (index, _temp_dir) = create_benchmark_index(5000);
    let search = EnhancedSearch::new(index).unwrap();
    
    group.bench_function("search_all_fields", |b| {
        b.iter(|| {
            let query = SearchQueryBuilder::new("async".to_string());
            let results = search.search(query, 10).unwrap();
            black_box(results);
        });
    });
    
    group.bench_function("search_title_only", |b| {
        b.iter(|| {
            let query = SearchQueryBuilder::new("async".to_string())
                .with_fields(vec!["title".to_string()]);
            let results = search.search(query, 10).unwrap();
            black_box(results);
        });
    });
    
    group.bench_function("search_multiple_fields", |b| {
        b.iter(|| {
            let query = SearchQueryBuilder::new("async".to_string())
                .with_fields(vec!["title".to_string(), "body".to_string()]);
            let results = search.search(query, 10).unwrap();
            black_box(results);
        });
    });
    
    group.finish();
}

fn benchmark_search_with_config(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_with_config");
    
    let (index, _temp_dir) = create_benchmark_index(5000);
    let search = EnhancedSearch::new(index).unwrap();
    
    // Default config
    group.bench_function("default_config", |b| {
        b.iter(|| {
            let query = SearchQueryBuilder::new("http client".to_string());
            let results = search.search(query, 10).unwrap();
            black_box(results);
        });
    });
    
    // Config with field boosts
    group.bench_function("with_field_boosts", |b| {
        let mut field_boosts = HashMap::new();
        field_boosts.insert("title".to_string(), 5.0);
        field_boosts.insert("body".to_string(), 1.0);
        
        let config = SearchConfig {
            field_boosts,
            ..Default::default()
        };
        
        b.iter(|| {
            let query = SearchQueryBuilder::new("http client".to_string())
                .with_config(config.clone());
            let results = search.search(query, 10).unwrap();
            black_box(results);
        });
    });
    
    group.finish();
}

fn benchmark_result_limits(c: &mut Criterion) {
    let mut group = c.benchmark_group("result_limits");
    
    let (index, _temp_dir) = create_benchmark_index(10000);
    let search = EnhancedSearch::new(index).unwrap();
    
    for &limit in &[1, 10, 50, 100] {
        group.bench_function(format!("limit_{}", limit), |b| {
            b.iter(|| {
                let query = SearchQueryBuilder::new("async".to_string());
                let results = search.search(query, limit).unwrap();
                black_box(results);
            });
        });
    }
    
    group.finish();
}

fn benchmark_query_complexity(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_complexity");
    
    let (index, _temp_dir) = create_benchmark_index(5000);
    let search = EnhancedSearch::new(index).unwrap();
    
    // Single term
    group.bench_function("single_term", |b| {
        b.iter(|| {
            let query = SearchQueryBuilder::new("async".to_string());
            let results = search.search(query, 10).unwrap();
            black_box(results);
        });
    });
    
    // Multiple terms
    group.bench_function("two_terms", |b| {
        b.iter(|| {
            let query = SearchQueryBuilder::new("async runtime".to_string());
            let results = search.search(query, 10).unwrap();
            black_box(results);
        });
    });
    
    group.bench_function("five_terms", |b| {
        b.iter(|| {
            let query = SearchQueryBuilder::new("async runtime http client server".to_string());
            let results = search.search(query, 10).unwrap();
            black_box(results);
        });
    });
    
    // Phrase query
    group.bench_function("phrase_query", |b| {
        b.iter(|| {
            let query = SearchQueryBuilder::new("\"async runtime\"".to_string());
            let results = search.search(query, 10).unwrap();
            black_box(results);
        });
    });
    
    group.finish();
}

fn benchmark_concurrent_searches(c: &mut Criterion) {
    use std::sync::Arc;
    
    let mut group = c.benchmark_group("concurrent_searches");
    group.sample_size(10); // Reduce sample size for concurrent benchmarks
    
    let (index, _temp_dir) = create_benchmark_index(5000);
    let search = Arc::new(EnhancedSearch::new(index).unwrap());
    
    // Sequential baseline
    group.bench_function("sequential_10_searches", |b| {
        b.iter(|| {
            for i in 0..10 {
                let query = SearchQueryBuilder::new(format!("query{}", i % 5));
                let results = search.search(query, 10).unwrap();
                black_box(results);
            }
        });
    });
    
    // Concurrent searches using threads
    group.bench_function("concurrent_10_searches", |b| {
        b.iter(|| {
            let handles: Vec<_> = (0..10)
                .map(|i| {
                    let search_clone = Arc::clone(&search);
                    std::thread::spawn(move || {
                        let query = SearchQueryBuilder::new(format!("query{}", i % 5));
                        let results = search_clone.search(query, 10).unwrap();
                        black_box(results);
                    })
                })
                .collect();
            
            for handle in handles {
                handle.join().unwrap();
            }
        });
    });
    
    group.finish();
}

fn benchmark_highlighting_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("highlighting");
    
    let (index, _temp_dir) = create_benchmark_index(1000);
    let search = EnhancedSearch::new(index).unwrap();
    
    // Measure the overhead of highlight extraction
    group.bench_function("search_with_highlights", |b| {
        b.iter(|| {
            let query = SearchQueryBuilder::new("async runtime http".to_string());
            let results = search.search(query, 20).unwrap();
            
            // Force evaluation of highlights
            for result in &results {
                black_box(&result.highlights);
            }
            black_box(results);
        });
    });
    
    group.finish();
}

fn benchmark_index_size_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("index_size_scaling");
    group.sample_size(10); // Reduce sample size for large indices
    
    // Test how search performance scales with index size
    for &size in &[1000, 5000, 10000, 25000] {
        group.bench_function(format!("index_size_{}", size), |b| {
            // Create index once per benchmark iteration to avoid keeping large indices in memory
            b.iter_batched(
                || {
                    let (index, temp_dir) = create_benchmark_index(size);
                    let search = EnhancedSearch::new(index).unwrap();
                    (search, temp_dir)
                },
                |(search, _temp_dir)| {
                    let query = SearchQueryBuilder::new("async runtime".to_string());
                    let results = search.search(query, 10).unwrap();
                    black_box(results);
                },
                BatchSize::LargeInput,
            );
        });
    }
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_basic_search,
    benchmark_field_specific_search,
    benchmark_search_with_config,
    benchmark_result_limits,
    benchmark_query_complexity,
    benchmark_concurrent_searches,
    benchmark_highlighting_overhead,
    benchmark_index_size_scaling
);

criterion_main!(benches);