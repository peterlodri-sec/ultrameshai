use tempfile::NamedTempFile;
use loop_engineering_transport::proto::UnitSpawn;
use loop_engineering_transport::mmap::{write_to_mmap, read_from_mmap};

#[test]
fn test_mmap_roundtrip() {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path();

    let msg = UnitSpawn {
        unit_id: "test-unit-123".to_string(),
        slice_id: "test-slice".to_string(),
        loop_type: "coder".to_string(),
        sandbox_tier: "test".to_string(),
        nix_shell: "devShells.agent-unit".to_string(),
        memory_limit_mb: 128,
        assigned_node: "rpi4".to_string(),
    };

    // Write to mmap
    let len = write_to_mmap(path, &msg).unwrap();
    assert!(len > 0);

    // Read from mmap
    let decoded: UnitSpawn = read_from_mmap(path, len).unwrap();

    assert_eq!(decoded.unit_id, msg.unit_id);
    assert_eq!(decoded.slice_id, msg.slice_id);
    assert_eq!(decoded.loop_type, msg.loop_type);
    assert_eq!(decoded.sandbox_tier, msg.sandbox_tier);
    assert_eq!(decoded.nix_shell, msg.nix_shell);
    assert_eq!(decoded.memory_limit_mb, msg.memory_limit_mb);
    assert_eq!(decoded.assigned_node, msg.assigned_node);
}
