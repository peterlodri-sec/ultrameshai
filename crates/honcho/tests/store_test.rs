// PatternStore integration tests
// These tests require a running milvus instance
// Run with: docker-compose -f docker-compose.milvus.yml up -d

use honcho::{PatternStore, LearningPattern};

#[tokio::test]
#[ignore]
async fn test_connect_creates_collection() {
    let store = PatternStore::connect("http://localhost:19530").await.unwrap();
    
    // Verify collection exists (would fail if not created)
    let patterns = store.query_all().await.unwrap();
    assert!(true); // Collection exists if query succeeds
}

#[tokio::test]
#[ignore]
async fn test_write_pattern() {
    let store = PatternStore::connect("http://localhost:19530").await.unwrap();
    
    let pattern = LearningPattern::new(
        "performance",
        0.85,
        "Coder loops show performance degradation after 100 units",
        vec!["coder".into(), "tester".into()],
    )
    .with_evidence_count(50)
    .with_metadata(serde_json::json!({
        "mean_runtime_ms": 1500.0,
        "std_dev_ms": 200.0
    }));
    
    let result = store.write_pattern(pattern).await;
    assert!(result.is_ok());
}

#[tokio::test]
#[ignore]
async fn test_query_by_type() {
    let store = PatternStore::connect("http://localhost:19530").await.unwrap();
    
    // Write a pattern first
    let pattern = LearningPattern::new(
        "failure",
        0.75,
        "High failure rate in red-team loops",
        vec!["red-team".into()],
    )
    .with_evidence_count(25);
    
    store.write_pattern(pattern).await.unwrap();
    
    // Query by type
    let patterns = store.query_by_type("failure").await.unwrap();
    assert!(!patterns.is_empty());
    
    let failure_pattern = patterns.iter().find(|p| p.pattern_type == "failure");
    assert!(failure_pattern.is_some());
}

#[tokio::test]
#[ignore]
async fn test_query_similar() {
    let store = PatternStore::connect("http://localhost:19530").await.unwrap();
    
    // Write a pattern
    let pattern = LearningPattern::new(
        "success",
        0.9,
        "Deep research loops have high success rate with async patterns",
        vec!["deep-research".into()],
    );
    
    store.write_pattern(pattern).await.unwrap();
    
    // Query similar patterns
    let patterns = store.query_similar("async research success", 5).await.unwrap();
    assert!(!patterns.is_empty());
}

#[tokio::test]
#[ignore]
async fn test_write_and_query_roundtrip() {
    let store = PatternStore::connect("http://localhost:19530").await.unwrap();
    
    let original = LearningPattern::new(
        "cross-loop",
        0.8,
        "Cross-loop correlation between coder and tester on rust async",
        vec!["coder".into(), "tester".into()],
    )
    .with_evidence_count(15)
    .with_metadata(serde_json::json!({
        "jaccard": 0.5
    }));
    
    store.write_pattern(original.clone()).await.unwrap();
    
    // Query by type
    let patterns = store.query_by_type("cross-loop").await.unwrap();
    assert!(!patterns.is_empty());
    
    // Verify pattern was stored
    let found = patterns.iter().find(|p| p.pattern_type == "cross-loop");
    assert!(found.is_some());
}
