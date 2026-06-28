use mempalace::{StateStore, InMemoryStore, UnitStats};

#[tokio::test]
async fn test_in_memory_store_roundtrip() {
    let store = InMemoryStore::new();

    let stats = UnitStats {
        unit_id: "unit-1".to_string(),
        slice_id: "slice-a".to_string(),
        loop_type: "coder".to_string(),
        spawned_at_ms: 1000,
        died_at_ms: 2000,
        peak_memory_mb: Some(45),
        status: "completed".to_string(),
        snapshot_path: None,
    };

    // Write stats
    store.write_stats(stats.clone()).await.unwrap();

    // Get stats
    let retrieved = store.get_unit("unit-1").await.unwrap().unwrap();
    assert_eq!(retrieved.unit_id, stats.unit_id);
    assert_eq!(retrieved.peak_memory_mb, Some(45));

    // Aggregate by loop type
    let loop_aggs = store.aggregate_by_loop_type().await.unwrap();
    assert_eq!(loop_aggs.len(), 1);
    assert_eq!(loop_aggs[0].loop_type, "coder");
    assert_eq!(loop_aggs[0].unit_count, 1);
    assert_eq!(loop_aggs[0].avg_runtime_ms, 1000.0);
    assert_eq!(loop_aggs[0].avg_peak_memory_mb, Some(45.0));

    // Aggregate by status
    let status_aggs = store.aggregate_by_status().await.unwrap();
    assert_eq!(status_aggs.len(), 1);
    assert_eq!(status_aggs[0].status, "completed");
    assert_eq!(status_aggs[0].unit_count, 1);

    // Memory distribution
    let mem_dist = store.memory_distribution().await.unwrap();
    assert_eq!(mem_dist[0].unit_count, 1); // 0-50MB bucket
    assert_eq!(mem_dist[1].unit_count, 0);

    // Clear
    store.clear().await.unwrap();
    let all = store.query_all().await.unwrap();
    assert!(all.is_empty());
}
