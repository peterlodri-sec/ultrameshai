use mempalace::{MockMempalaceClient, UnitStats, StatsQueryBuilder};

#[tokio::test]
async fn test_mock_write_and_query_all() {
    let mock = MockMempalaceClient::new();
    let stats = UnitStats::new(
        "unit-001".into(),
        "slice-001".into(),
        "coder".into(),
        1719500000000,
        1719500005000,
    );
    mock.write_stats(stats.clone()).await.unwrap();
    let results = mock.query_all().await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].unit_id, "unit-001");
}

#[tokio::test]
async fn test_mock_query_by_status() {
    let mock = MockMempalaceClient::new();
    mock.write_stats(
        UnitStats::new("u1".into(), "s1".into(), "coder".into(), 1000, 2000)
            .with_status("completed"),
    )
    .await
    .unwrap();
    mock.write_stats(
        UnitStats::new("u2".into(), "s1".into(), "coder".into(), 1000, 2000)
            .with_status("killed"),
    )
    .await
    .unwrap();

    let query = StatsQueryBuilder::new().filter_status("completed").build();
    let results = mock.query_stats(query).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].status, "completed");
}

#[tokio::test]
async fn test_mock_query_by_slice() {
    let mock = MockMempalaceClient::new();
    mock.write_stats(
        UnitStats::new("u1".into(), "s1".into(), "coder".into(), 1000, 2000),
    )
    .await
    .unwrap();
    mock.write_stats(
        UnitStats::new("u2".into(), "s2".into(), "coder".into(), 1000, 2000),
    )
    .await
    .unwrap();

    let query = StatsQueryBuilder::new().filter_slice_id("s1").build();
    let results = mock.query_stats(query).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].slice_id, "s1");
}

#[tokio::test]
async fn test_mock_get_unit() {
    let mock = MockMempalaceClient::new();
    let stats = UnitStats::new(
        "u1".into(),
        "s1".into(),
        "coder".into(),
        1000,
        2000,
    );
    mock.write_stats(stats.clone()).await.unwrap();

    let retrieved = mock.get_unit("u1").await.unwrap().unwrap();
    assert_eq!(retrieved.unit_id, "u1");
    assert_eq!(retrieved.slice_id, "s1");

    let not_found = mock.get_unit("nonexistent").await.unwrap();
    assert!(not_found.is_none());
}

#[tokio::test]
async fn test_mock_clear() {
    let mock = MockMempalaceClient::new();
    for i in 0..5 {
        mock.write_stats(UnitStats::new(
            format!("u{}", i),
            "s1".into(),
            "coder".into(),
            1000,
            2000,
        ))
        .await
        .unwrap();
    }

    let all = mock.query_all().await.unwrap();
    assert_eq!(all.len(), 5);

    mock.clear().await.unwrap();
    let all_after = mock.query_all().await.unwrap();
    assert_eq!(all_after.len(), 0);
}
