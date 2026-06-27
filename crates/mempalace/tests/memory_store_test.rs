// Test MemoryStore trait implementation for milvus compatibility

#[cfg(feature = "milvus-compat")]
#[tokio::test]
async fn test_memory_store_noop() {
    use mempalace::MempalaceClient;
    use milvus_brain::{MemoryStore, ResearchFinding, QueryBuilder as MilvusQueryBuilder};
    
    let db_path = "/tmp/mempalace_test_memory_store.db";
    let client = MempalaceClient::connect(db_path).await.unwrap();
    
    // write_finding should return Ok without side effects
    let finding = ResearchFinding::new(
        "test-finding",
        "test-agent",
        "test-topic",
        "test summary",
        vec![0.1; 1536],
        vec!["test".into()],
    );
    let result = client.write_finding(finding).await;
    assert!(result.is_ok());
    
    // search should return empty vec without side effects
    let query = MilvusQueryBuilder::new().similarity("test", 10);
    let result = client.search(query).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
    
    // delete_finding should return Ok without side effects
    let result = client.delete_finding("test-finding").await;
    assert!(result.is_ok());
    
    std::fs::remove_file(db_path).ok();
}

#[cfg(feature = "milvus-compat")]
#[tokio::test]
async fn test_memory_store_trait_object() {
    use mempalace::MempalaceClient;
    use milvus_brain::{MemoryStore, ResearchFinding};
    use std::sync::Arc;
    
    let db_path = "/tmp/mempalace_test_trait_obj.db";
    let client = MempalaceClient::connect(db_path).await.unwrap();
    
    // Verify MempalaceClient can be used as trait object
    let store: Arc<dyn MemoryStore + Send + Sync> = Arc::new(client);
    
    // Call trait methods through trait object
    let finding = ResearchFinding::new(
        "test",
        "agent",
        "topic",
        "summary",
        vec![0.1; 1536],
        vec![],
    );
    let result = store.write_finding(finding).await;
    assert!(result.is_ok());
    
    std::fs::remove_file(db_path).ok();
}
