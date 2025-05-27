use gitcodes_mcp::gitcodes::CodeSearchResult;
use gitcodes_mcp::tools::responses::CompactCodeSearchResponse;
use lumin::search::SearchResultLine;
use std::path::PathBuf;

#[test]
fn test_compact_code_search_response_conversion() {
    // Create sample search result lines
    let search_lines = vec![
        SearchResultLine {
            file_path: PathBuf::from("src/main.rs"),
            line_number: 10,
            line_content: "fn main() {".to_string(),
            content_omitted: false,
            is_context: false,
        },
        SearchResultLine {
            file_path: PathBuf::from("src/main.rs"),
            line_number: 11,
            line_content: "    println!(\"Hello, world!\");".to_string(),
            content_omitted: false,
            is_context: true,
        },
        SearchResultLine {
            file_path: PathBuf::from("src/lib.rs"),
            line_number: 25,
            line_content: "pub fn main_function() -> Result<(), Error> {".to_string(),
            content_omitted: false,
            is_context: false,
        },
        SearchResultLine {
            file_path: PathBuf::from("tests/integration_test.rs"),
            line_number: 15,
            line_content: "    let result = main_module::main_function();".to_string(),
            content_omitted: true,
            is_context: false,
        },
    ];

    // Create CodeSearchResult directly
    let search_result = CodeSearchResult {
        total_match_line_number: 4,
        matches: search_lines,
        pattern: "main".to_string(),
        repository: "/tmp/test_repo".to_string(),
        case_sensitive: false,
        file_extensions: None,
        include_globs: Some(vec!["**/*.rs".to_string(), "**/*.md".to_string()]),
        exclude_globs: Some(vec!["**/target/**".to_string(), "**/.git/**".to_string()]),
        before_context: Some(0),
        after_context: Some(1),
    };

    // Convert to compact format
    let compact = CompactCodeSearchResponse::from_search_result(search_result);

    // Verify the conversion
    assert_eq!(compact.total_match_line_number, 4);
    assert_eq!(compact.pattern, "main");
    assert_eq!(compact.case_sensitive, false);
    assert_eq!(compact.file_extensions, None);
    assert_eq!(compact.include_globs, Some(vec!["**/*.rs".to_string(), "**/*.md".to_string()]));
    assert_eq!(compact.exclude_globs, Some(vec!["**/target/**".to_string(), "**/.git/**".to_string()]));
    assert_eq!(compact.before_context, Some(0));
    assert_eq!(compact.after_context, Some(1));

    // Verify matches are grouped by file
    assert_eq!(compact.matches.len(), 3); // 3 different files

    // Find the main.rs match
    let main_rs_match = compact.matches.iter()
        .find(|m| m.file_path == "src/main.rs")
        .expect("Should find src/main.rs match");

    // Verify the lines are concatenated correctly
    let expected_lines = "10:fn main() {\n11:    println!(\"Hello, world!\");";
    assert_eq!(main_rs_match.lines, expected_lines);

    // Find the lib.rs match
    let lib_rs_match = compact.matches.iter()
        .find(|m| m.file_path == "src/lib.rs")
        .expect("Should find src/lib.rs match");

    let expected_lib_lines = "25:pub fn main_function() -> Result<(), Error> {";
    assert_eq!(lib_rs_match.lines, expected_lib_lines);

    // Find the integration test match
    let test_match = compact.matches.iter()
        .find(|m| m.file_path == "tests/integration_test.rs")
        .expect("Should find tests/integration_test.rs match");

    let expected_test_lines = "15:    let result = main_module::main_function();";
    assert_eq!(test_match.lines, expected_test_lines);
}

#[test]
fn test_compact_code_search_response_serialization() {
    // Create a simple search result
    let search_lines = vec![
        SearchResultLine {
            file_path: PathBuf::from("example.rs"),
            line_number: 1,
            line_content: "// Example file".to_string(),
            content_omitted: false,
            is_context: false,
        },
    ];

    let search_result = CodeSearchResult {
        total_match_line_number: 1,
        matches: search_lines,
        pattern: "Example".to_string(),
        repository: "/tmp/repo".to_string(),
        case_sensitive: true,
        file_extensions: Some(vec!["rs".to_string()]),
        include_globs: None,
        exclude_globs: None,
        before_context: None,
        after_context: None,
    };

    let compact = CompactCodeSearchResponse::from_search_result(search_result);

    // Test JSON serialization
    let json = serde_json::to_string(&compact).expect("Should serialize to JSON");
    
    // Verify it contains expected fields
    assert!(json.contains("\"total_match_line_number\":1"));
    assert!(json.contains("\"pattern\":\"Example\""));
    assert!(json.contains("\"case_sensitive\":true"));
    assert!(json.contains("\"file_extensions\":[\"rs\"]"));
    assert!(json.contains("\"file_path\":\"example.rs\""));
    assert!(json.contains("\"lines\":\"1:// Example file\""));

    // Test deserialization
    let deserialized: CompactCodeSearchResponse = serde_json::from_str(&json)
        .expect("Should deserialize from JSON");
    
    assert_eq!(deserialized.total_match_line_number, 1);
    assert_eq!(deserialized.pattern, "Example");
    assert_eq!(deserialized.case_sensitive, true);
    assert_eq!(deserialized.matches.len(), 1);
    assert_eq!(deserialized.matches[0].file_path, "example.rs");
    assert_eq!(deserialized.matches[0].lines, "1:// Example file");
}

#[test]
fn test_compact_code_search_response_json_format() {
    // Create a realistic example to show the JSON format
    let search_lines = vec![
        SearchResultLine {
            file_path: PathBuf::from("src/main.rs"),
            line_number: 10,
            line_content: "fn main() {".to_string(),
            content_omitted: false,
            is_context: false,
        },
        SearchResultLine {
            file_path: PathBuf::from("src/main.rs"),
            line_number: 11,
            line_content: "    println!(\"Hello, world!\");".to_string(),
            content_omitted: false,
            is_context: true,
        },
        SearchResultLine {
            file_path: PathBuf::from("src/lib.rs"),
            line_number: 25,
            line_content: "pub fn main_function() -> Result<(), Error> {".to_string(),
            content_omitted: false,
            is_context: false,
        },
    ];

    let search_result = CodeSearchResult {
        total_match_line_number: 3,
        matches: search_lines,
        pattern: "main".to_string(),
        repository: "/tmp/example_repo".to_string(),
        case_sensitive: false,
        file_extensions: None,
        include_globs: Some(vec!["**/*.rs".to_string()]),
        exclude_globs: Some(vec!["**/target/**".to_string()]),
        before_context: Some(0),
        after_context: Some(1),
    };

    let compact = CompactCodeSearchResponse::from_search_result(search_result);
    let json = serde_json::to_string_pretty(&compact).expect("Should serialize to JSON");
    
    println!("Sample JSON output:");
    println!("{}", json);
    
    // Verify the structure
    assert_eq!(compact.total_match_line_number, 3);
    assert_eq!(compact.matches.len(), 2); // 2 files
}