use mempalace::{MempalaceClient, UnitStats};

#[tokio::test]
async fn test_aggregate_by_loop_type() {
    let db_path = "/tmp/mempalace_test_agg_loop.db";
    let client = MempalaceClient::connect(db_path).await.unwrap();

    // Write test data with different loop types
    client
        .write_stats(UnitStats::new(
            "u1".into(),
            "s1".into(),
            "coder".into(),
            1000,
            2000,
        ))
        .await
        .unwrap();
    client
        .write_stats(UnitStats::new(
            "u2".into(),
            "s1".into(),
            "coder".into(),
            1000,
            3000,
        ))
        .await
        .unwrap();
    client
        .write_stats(UnitStats::new(
            "u3".into(),
            "s1".into(),
            "tester".into(),
            1000,
            2000,
        ))
        .await
        .unwrap();

    let results = client.aggregate_by_loop_type().await.unwrap();
    assert_eq!(results.len(), 2);

    let coder = results.iter().find(|r| r.loop_type == "coder").unwrap();
    assert_eq!(coder.unit_count, 2);
    assert_eq!(coder.avg_runtime_ms, 1500.0); // (1000 + 2000) / 2

    let tester = results.iter().find(|r| r.loop_type == "tester").unwrap();
    assert_eq!(tester.unit_count, 1);
    assert_eq!(tester.avg_runtime_ms, 1000.0);

    std::fs::remove_file(db_path).ok();
}

#[tokio::test]
async fn test_aggregate_by_status() {
    let db_path = "/tmp/mempalace_test_agg_status.db";
    let client = MempalaceClient::connect(db_path).await.unwrap();

    client
        .write_stats(
            UnitStats::new("u1".into(), "s1".into(), "coder".into(), 1000, 2000)
                .with_status("completed"),
        )
        .await
        .unwrap();
    client
        .write_stats(
            UnitStats::new("u2".into(), "s1".into(), "coder".into(), 1000, 2000)
                .with_status("completed"),
        )
        .await
        .unwrap();
    client
        .write_stats(
            UnitStats::new("u3".into(), "s1".into(), "coder".into(), 1000, 2000)
                .with_status("killed"),
        )
        .await
        .unwrap();
    client
        .write_stats(
            UnitStats::new("u4".into(), "s1".into(), "coder".into(), 1000, 2000)
                .with_status("failed"),
        )
        .await
        .unwrap();

    let results = client.aggregate_by_status().await.unwrap();
    assert_eq!(results.len(), 3);

    let completed = results.iter().find(|r| r.status == "completed").unwrap();
    assert_eq!(completed.unit_count, 2);

    let killed = results.iter().find(|r| r.status == "killed").unwrap();
    assert_eq!(killed.unit_count, 1);

    let failed = results.iter().find(|r| r.status == "failed").unwrap();
    assert_eq!(failed.unit_count, 1);

    std::fs::remove_file(db_path).ok();
}

#[tokio::test]
async fn test_memory_distribution() {
    let db_path = "/tmp/mempalace_test_mem_dist.db";
    let client = MempalaceClient::connect(db_path).await.unwrap();

    // Write units with different memory usage
    client
        .write_stats(
            UnitStats::new("u1".into(), "s1".into(), "coder".into(), 1000, 2000)
                .with_memory(50),
        )
        .await
        .unwrap();
    client
        .write_stats(
            UnitStats::new("u2".into(), "s1".into(), "coder".into(), 1000, 2000)
                .with_memory(80),
        )
        .await
        .unwrap();
    client
        .write_stats(
            UnitStats::new("u3".into(), "s1".into(), "coder".into(), 1000, 2000)
                .with_memory(120),
        )
        .await
        .unwrap();
    client
        .write_stats(
            UnitStats::new("u4".into(), "s1".into(), "coder".into(), 1000, 2000)
                .with_memory(140),
        )
        .await
        .unwrap();
    client
        .write_stats(
            UnitStats::new("u5".into(), "s1".into(), "coder".into(), 1000, 2000)
                .with_memory(180),
        )
        .await
        .unwrap();
    client
        .write_stats(
            UnitStats::new("u6".into(), "s1".into(), "coder".into(), 1000, 2000)
                .with_memory(200),
        )
        .await
        .unwrap();

    let results = client.memory_distribution().await.unwrap();
    assert_eq!(results.len(), 3);

    let bucket_0_100 = results
        .iter()
        .find(|r| r.memory_bucket == "0-100MB")
        .unwrap();
    assert_eq!(bucket_0_100.unit_count, 2); // u1, u2

    let bucket_100_150 = results
        .iter()
        .find(|r| r.memory_bucket == "100-150MB")
        .unwrap();
    assert_eq!(bucket_100_150.unit_count, 2); // u3, u4

    let bucket_150_plus = results
        .iter()
        .find(|r| r.memory_bucket == "150MB+")
        .unwrap();
    assert_eq!(bucket_150_plus.unit_count, 2); // u5, u6

    std::fs::remove_file(db_path).ok();
}

#[tokio::test]
async fn test_aggregate_with_empty_table() {
    let db_path = "/tmp/mempalace_test_agg_empty.db";
    let client = MempalaceClient::connect(db_path).await.unwrap();

    let loop_results = client.aggregate_by_loop_type().await.unwrap();
    assert_eq!(loop_results.len(), 0);

    let status_results = client.aggregate_by_status().await.unwrap();
    assert_eq!(status_results.len(), 0);

    let mem_results = client.memory_distribution().await.unwrap();
    assert_eq!(mem_results.len(), 0);

    std::fs::remove_file(db_path).ok();
}
