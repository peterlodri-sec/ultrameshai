// Integration tests - requires docker-compose up with milvus stack
// Run with: docker-compose -f docker-compose.milvus.yml up -d
// Then: cargo test --manifest-path crates/milvus-brain/Cargo.toml --test client_test -- --ignored

use milvus_brain::{MilvusClient, QueryBuilder, ResearchFinding};

const MILVUS_URI: &str = "http://localhost:19530";

#[tokio::test]
#[ignore]
async fn test_connect_to_milvus() {
    let client = MilvusClient::connect(MILVUS_URI).await.unwrap();
    assert_eq!(client.uri(), MILVUS_URI);
}

#[tokio::test]
#[ignore]
async fn test_ensure_collection() {
    let client = MilvusClient::connect(MILVUS_URI).await.unwrap();
    
    // Should create collection idempotently
    client.ensure_collection("research_findings").await.unwrap();
    client.ensure_collection("research_findings").await.unwrap();
}

#[tokio::test]
#[ignore]
async fn test_write_single_finding() {
    let client = MilvusClient::connect(MILVUS_URI).await.unwrap();
    client.ensure_collection("research_findings").await.unwrap();
    
    let finding = ResearchFinding::new(
        "integration-test-001",
        "deep-research",
        "tokio UDS",
        "Test finding",
        vec![0.1; 1536],
        vec!["test".into()],
    );
    
    client.write_finding(finding).await.unwrap();
}

#[tokio::test]
#[ignore]
async fn test_batch_write() {
    let client = MilvusClient::connect(MILVUS_URI).await.unwrap();
    client.ensure_collection("research_findings").await.unwrap();
    
    let findings: Vec<ResearchFinding> = (0..10)
        .map(|i| ResearchFinding::new(
            &format!("batch-{}", i),
            "junior-burst",
            "batch test",
            "batch summary",
            vec![0.5; 1536],
            vec!["batch".into()],
        ))
        .collect();
    
    client.batch_write(findings).await.unwrap();
}

#[tokio::test]
#[ignore]
async fn test_search_similarity() {
    let client = MilvusClient::connect(MILVUS_URI).await.unwrap();
    client.ensure_collection("research_findings").await.unwrap();
    
    // Write a finding
    let finding = ResearchFinding::new(
        "search-test-001",
        "deep-research",
        "tokio async",
        "Async programming in Rust",
        vec![0.1; 1536],
        vec!["rust".into(), "async".into()],
    );
    client.write_finding(finding).await.unwrap();
    
    // Search with similarity
    let query = QueryBuilder::new()
        .similarity("tokio async io", 5)
        .build();
    
    let results = client.search(query).await.unwrap();
    assert!(!results.is_empty());
}

#[tokio::test]
#[ignore]
async fn test_search_with_filters() {
    let client = MilvusClient::connect(MILVUS_URI).await.unwrap();
    client.ensure_collection("research_findings").await.unwrap();
    
    // Write findings with different agents
    let finding = ResearchFinding::new(
        "filter-test-001",
        "deep-research",
        "tokio",
        "Test",
        vec![0.1; 1536],
        vec!["rust".into()],
    );
    client.write_finding(finding).await.unwrap();
    
    // Search with agent filter
    let query = QueryBuilder::new()
        .filter_agent("deep-research")
        .build();
    
    let results = client.search(query).await.unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].source_agent, "deep-research");
}

#[tokio::test]
#[ignore]
async fn test_delete_finding() {
    let client = MilvusClient::connect(MILVUS_URI).await.unwrap();
    client.ensure_collection("research_findings").await.unwrap();
    
    let finding = ResearchFinding::new(
        "delete-test-001",
        "agent",
        "topic",
        "summary",
        vec![0.1; 1536],
        vec![],
    );
    
    client.write_finding(finding.clone()).await.unwrap();
    client.delete_finding(&finding.finding_id).await.unwrap();
}
