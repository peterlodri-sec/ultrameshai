use mempalace::{MempalaceClient, UnitStats, StatsQueryBuilder};

#[tokio::test]
async fn test_connect_creates_schema() {
    let db_path = "/tmp/mempalace_test_schema.db";
    let client = MempalaceClient::connect(db_path).await.unwrap();
    
    // Verify table exists by querying via client
    let all = client.query_all().await.unwrap();
    assert_eq!(all.len(), 0); // Empty after schema creation
    
    std::fs::remove_file(db_path).ok();
}

#[tokio::test]
async fn test_write_and_get_unit() {
    let db_path = "/tmp/mempalace_test_write.db";
    let client = MempalaceClient::connect(db_path).await.unwrap();
    
    let stats = UnitStats::new("u1".into(), "s1".into(), "coder".into(), 1000, 2000);
    client.write_stats(stats.clone()).await.unwrap();
    
    let retrieved = client.get_unit("u1").await.unwrap().unwrap();
    assert_eq!(retrieved.unit_id, "u1");
    assert_eq!(retrieved.slice_id, "s1");
    
    std::fs::remove_file(db_path).ok();
}

#[tokio::test]
async fn test_query_by_status() {
    let db_path = "/tmp/mempalace_test_status.db";
    let client = MempalaceClient::connect(db_path).await.unwrap();
    
    client.write_stats(
        UnitStats::new("u1".into(), "s1".into(), "coder".into(), 1000, 2000)
            .with_status("completed"),
    ).await.unwrap();
    
    client.write_stats(
        UnitStats::new("u2".into(), "s1".into(), "coder".into(), 1000, 2000)
            .with_status("killed"),
    ).await.unwrap();
    
    let query = StatsQueryBuilder::new().filter_status("completed").build();
    let results = client.query_stats(query).await.unwrap();
    
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].status, "completed");
    
    std::fs::remove_file(db_path).ok();
}
