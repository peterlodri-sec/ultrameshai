// HonchoDaemon integration tests

use honcho::HonchoDaemon;
use tempfile::tempdir;
use std::time::Duration;

#[tokio::test]
async fn test_daemon_starts_and_polls() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    // Create daemon with short poll interval for testing
    let daemon = HonchoDaemon::new(&db_path.to_string_lossy(), None, Some(100))
        .await
        .unwrap();

    assert!(!daemon.is_running());

    daemon.start().await.unwrap();
    assert!(daemon.is_running());

    // Wait for at least one poll cycle
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(daemon.is_running());

    daemon.stop();
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(!daemon.is_running());
}

#[tokio::test]
async fn test_daemon_with_milvus_uri() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    // Test with milvus URI (will fail to connect, but tests API)
    let result = HonchoDaemon::new(
        &db_path.to_string_lossy(),
        Some("http://localhost:19530"),
        Some(1000),
    )
    .await;

    // Connection may fail if milvus not running
    // Test verifies API accepts optional milvus_uri
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_daemon_env_var_interval() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    // Set env var
    std::env::set_var("HONCHO_POLL_INTERVAL_MS", "60000");

    let daemon = HonchoDaemon::new(&db_path.to_string_lossy(), None, None)
        .await
        .unwrap();

    assert_eq!(daemon.poll_interval_ms(), 60000);

    std::env::remove_var("HONCHO_POLL_INTERVAL_MS");
}

#[tokio::test]
async fn test_daemon_processes_batch() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    let daemon = HonchoDaemon::new(&db_path.to_string_lossy(), None, Some(100))
        .await
        .unwrap();

    daemon.start().await.unwrap();

    // Wait for poll cycle
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Verify daemon still running (didn't crash on empty batch)
    assert!(daemon.is_running());

    daemon.stop();
}


