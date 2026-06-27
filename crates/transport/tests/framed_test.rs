use loop_engineering_transport::framed::{write_message, read_message};
use loop_engineering_transport::proto::UnitSpawn;
use tokio::io::{duplex, AsyncWriteExt};

#[tokio::test]
async fn test_roundtrip_unit_spawn() {
    let (mut client, mut server) = duplex(4096);

    let msg = UnitSpawn {
        unit_id: "unit-001".into(),
        slice_id: "slice-001".into(),
        loop_type: "coder".into(),
        sandbox_tier: "standard".into(),
        nix_shell: "flake#agent-unit".into(),
        memory_limit_mb: 100,
        assigned_node: "vm-01".into(),
    };

    write_message(&mut client, &msg).await.unwrap();
    client.flush().await.unwrap();

    let received: UnitSpawn = read_message(&mut server).await.unwrap();
    assert_eq!(received.unit_id, "unit-001");
    assert_eq!(received.slice_id, "slice-001");
    assert_eq!(received.memory_limit_mb, 100);
}

#[tokio::test]
async fn test_pipelined_multiple_messages() {
    let (mut client, mut server) = duplex(8192);

    let msgs: Vec<UnitSpawn> = (0..10)
        .map(|i| UnitSpawn {
            unit_id: format!("unit-{:03}", i),
            slice_id: format!("slice-{:03}", i),
            loop_type: "coder".into(),
            sandbox_tier: "standard".into(),
            nix_shell: "flake#agent-unit".into(),
            memory_limit_mb: 100,
            assigned_node: "vm-01".into(),
        })
        .collect();

    // Pipelined: write all without waiting for responses
    for msg in &msgs {
        write_message(&mut client, msg).await.unwrap();
    }
    client.flush().await.unwrap();

    // Read all back in order
    for expected in &msgs {
        let received: UnitSpawn = read_message(&mut server).await.unwrap();
        assert_eq!(received.unit_id, expected.unit_id);
    }
}