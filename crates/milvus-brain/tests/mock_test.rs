use milvus_brain::{MockMilvusClient, QueryBuilder, ResearchFinding};

#[tokio::test]
async fn test_mock_client_write_and_search() {
    let mock = MockMilvusClient::new();
    
    let finding = ResearchFinding::new(
        "test-finding-001",
        "deep-research",
        "tokio UDS pipelining",
        "Pipelined protobuf over UDS achieves 10k msg/s",
        vec![0.1; 1536],
        vec!["rust".into(), "uds".into(), "ipc".into()],
    );
    
    mock.write_finding(finding.clone()).await.unwrap();
    
    let results = mock.search(QueryBuilder::new()).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].finding_id, "test-finding-001");
}

#[tokio::test]
async fn test_mock_client_batch_write() {
    let mock = MockMilvusClient::new();
    
    let findings: Vec<ResearchFinding> = (0..50)
        .map(|i| ResearchFinding::new(
            &format!("batch-{}", i),
            "junior-burst",
            "test topic",
            "test summary",
            vec![0.5; 1536],
            vec!["test".into()],
        ))
        .collect();
    
    mock.batch_write(findings).await.unwrap();
    assert_eq!(mock.count().await, 50);
}

#[tokio::test]
async fn test_mock_client_filters() {
    let mock = MockMilvusClient::new();
    
    mock.write_finding(ResearchFinding::new(
        "f1", "deep-research", "tokio", "summary", vec![0.1; 1536], vec!["rust".into()]
    )).await.unwrap();
    
    mock.write_finding(ResearchFinding::new(
        "f2", "junior-burst", "tokio", "summary", vec![0.2; 1536], vec!["rust".into()]
    )).await.unwrap();
    
    mock.write_finding(ResearchFinding::new(
        "f3", "deep-research", "async", "summary", vec![0.3; 1536], vec!["async".into()]
    )).await.unwrap();
    
    // Filter by agent
    let query = QueryBuilder::new().filter_agent("deep-research");
    let results = mock.search(query).await.unwrap();
    assert_eq!(results.len(), 2);
    
    // Filter by topic
    let query = QueryBuilder::new().filter_topic("tokio");
    let results = mock.search(query).await.unwrap();
    assert_eq!(results.len(), 2);
    
    // Filter by tag
    let query = QueryBuilder::new().filter_tags(vec!["rust".into()]);
    let results = mock.search(query).await.unwrap();
    assert_eq!(results.len(), 2);
    
    // Combined filters
    let query = QueryBuilder::new()
        .filter_agent("deep-research")
        .filter_topic("tokio");
    let results = mock.search(query).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].finding_id, "f1");
}

#[tokio::test]
async fn test_mock_client_delete() {
    let mock = MockMilvusClient::new();
    
    mock.write_finding(ResearchFinding::new(
        "to-delete", "agent", "topic", "summary", vec![0.1; 1536], vec![]
    )).await.unwrap();
    
    assert_eq!(mock.count().await, 1);
    
    mock.delete_finding("to-delete").await.unwrap();
    assert_eq!(mock.count().await, 0);
}

#[tokio::test]
async fn test_mock_client_top_k() {
    let mock = MockMilvusClient::new();
    
    for i in 0..20 {
        mock.write_finding(ResearchFinding::new(
            &format!("f{}", i),
            "agent",
            "topic",
            "summary",
            vec![0.1; 1536],
            vec![],
        )).await.unwrap();
    }
    
    let query = QueryBuilder::new().similarity("test", 5);
    let results = mock.search(query).await.unwrap();
    assert_eq!(results.len(), 5);
}
