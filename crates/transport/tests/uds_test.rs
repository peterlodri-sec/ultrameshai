use loop_engineering_transport::uds::{UdsServer, UdsClient};
use loop_engineering_transport::proto::UnitSpawn;
use std::time::Duration;
use tempfile::tempdir;

#[tokio::test]
async fn test_uds_server_client_roundtrip() {
    let dir = tempdir().unwrap();
    let socket_path = dir.path().join("test.sock");

    let server = UdsServer::bind(&socket_path).await.unwrap();
    let server_handle = tokio::spawn(async move {
        server.accept(|mut conn| {
            Box::pin(async move {
                let msg: UnitSpawn = conn.read().await.unwrap();
                assert_eq!(msg.unit_id, "unit-test");
                // Echo back
                conn.write(&msg).await.unwrap();
            })
        }).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(50)).await;

    let mut client = UdsClient::connect(&socket_path).await.unwrap();
    let msg = UnitSpawn {
        unit_id: "unit-test".into(),
        slice_id: "slice-test".into(),
        loop_type: "coder".into(),
        sandbox_tier: "standard".into(),
        nix_shell: "flake#agent-unit".into(),
        memory_limit_mb: 100,
        assigned_node: "vm-01".into(),
    };
    client.write(&msg).await.unwrap();

    let echo: UnitSpawn = client.read().await.unwrap();
    assert_eq!(echo.unit_id, "unit-test");

    server_handle.abort();
}

#[tokio::test]
async fn test_uds_pipelined_100_messages() {
    let dir = tempdir().unwrap();
    let socket_path = dir.path().join("pipe.sock");

    let server = UdsServer::bind(&socket_path).await.unwrap();
    let server_handle = tokio::spawn(async move {
        server.accept(|mut conn| {
            Box::pin(async move {
                for _ in 0..100 {
                    let msg: UnitSpawn = conn.read().await.unwrap();
                    conn.write(&msg).await.unwrap();
                }
            })
        }).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let mut client = UdsClient::connect(&socket_path).await.unwrap();

    // Pipeline: write all 100, then read all 100
    for i in 0..100u32 {
        let msg = UnitSpawn {
            unit_id: format!("unit-{:03}", i),
            slice_id: format!("slice-{:03}", i),
            loop_type: "coder".into(),
            sandbox_tier: "standard".into(),
            nix_shell: "flake#agent-unit".into(),
            memory_limit_mb: 100,
            assigned_node: "vm-01".into(),
        };
        client.write(&msg).await.unwrap();
    }

    for i in 0..100u32 {
        let echo: UnitSpawn = client.read().await.unwrap();
        assert_eq!(echo.unit_id, format!("unit-{:03}", i));
    }

    server_handle.abort();
}