use loop_engineering_transport::proto::UnitStats as ProtoUnitStats;

/// UnitStats - represents unit lifecycle telemetry
#[derive(Debug, Clone)]
pub struct UnitStats {
    pub unit_id: String,
    pub slice_id: String,
    pub loop_type: String,
    pub spawned_at_ms: u64,
    pub died_at_ms: u64,
    pub peak_memory_mb: Option<u32>,
    pub status: String,
    pub snapshot_path: Option<String>,
}

impl UnitStats {
    pub fn new(
        unit_id: String,
        slice_id: String,
        loop_type: String,
        spawned_at_ms: u64,
        died_at_ms: u64,
    ) -> Self {
        Self {
            unit_id,
            slice_id,
            loop_type,
            spawned_at_ms,
            died_at_ms,
            peak_memory_mb: None,
            status: "completed".into(),
            snapshot_path: None,
        }
    }

    pub fn with_memory(mut self, peak_mb: u32) -> Self {
        self.peak_memory_mb = Some(peak_mb);
        self
    }

    pub fn with_status(mut self, status: &str) -> Self {
        self.status = status.to_string();
        self
    }

    pub fn with_snapshot(mut self, path: &str) -> Self {
        self.snapshot_path = Some(path.to_string());
        self
    }

    pub fn runtime_ms(&self) -> u64 {
        self.died_at_ms - self.spawned_at_ms
    }
}

impl From<ProtoUnitStats> for UnitStats {
    fn from(proto: ProtoUnitStats) -> Self {
        Self {
            unit_id: proto.unit_id,
            slice_id: proto.slice_id,
            loop_type: proto.loop_type,
            spawned_at_ms: proto.spawned_at_ms,
            died_at_ms: proto.died_at_ms,
            peak_memory_mb: if proto.peak_memory_mb > 0 { Some(proto.peak_memory_mb) } else { None },
            status: proto.status,
            snapshot_path: if proto.snapshot_path.is_empty() { None } else { Some(proto.snapshot_path) },
        }
    }
}

impl From<UnitStats> for ProtoUnitStats {
    fn from(stats: UnitStats) -> Self {
        Self {
            unit_id: stats.unit_id,
            slice_id: stats.slice_id,
            loop_type: stats.loop_type,
            spawned_at_ms: stats.spawned_at_ms,
            died_at_ms: stats.died_at_ms,
            peak_memory_mb: stats.peak_memory_mb.unwrap_or(0),
            status: stats.status,
            snapshot_path: stats.snapshot_path.unwrap_or_default(),
            stats_blob: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loop_engineering_transport::proto::UnitStats as ProtoUnitStats;

    #[test]
    fn test_unit_stats_builder() {
        let stats = UnitStats::new(
            "u1".into(),
            "s1".into(),
            "coder".into(),
            1000,
            2000,
        )
        .with_memory(120)
        .with_status("killed")
        .with_snapshot("/tmp/snapshot");

        assert_eq!(stats.unit_id, "u1");
        assert_eq!(stats.peak_memory_mb, Some(120));
        assert_eq!(stats.status, "killed");
        assert_eq!(stats.snapshot_path, Some("/tmp/snapshot".to_string()));
        assert_eq!(stats.runtime_ms(), 1000);
    }

    #[test]
    fn test_unit_stats_default_status() {
        let stats = UnitStats::new(
            "u1".into(),
            "s1".into(),
            "coder".into(),
            1000,
            2000,
        );

        assert_eq!(stats.status, "completed");
        assert_eq!(stats.peak_memory_mb, None);
        assert_eq!(stats.snapshot_path, None);
    }

    #[test]
    fn test_protobuf_conversion_from_proto() {
        let proto = ProtoUnitStats {
            unit_id: "u1".into(),
            slice_id: "s1".into(),
            loop_type: "coder".into(),
            spawned_at_ms: 1000,
            died_at_ms: 2000,
            peak_memory_mb: 120,
            status: "completed".into(),
            snapshot_path: "".into(),
            stats_blob: vec![],
        };

        let stats: UnitStats = proto.clone().into();

        assert_eq!(stats.unit_id, "u1");
        assert_eq!(stats.slice_id, "s1");
        assert_eq!(stats.loop_type, "coder");
        assert_eq!(stats.spawned_at_ms, 1000);
        assert_eq!(stats.died_at_ms, 2000);
        assert_eq!(stats.peak_memory_mb, Some(120));
        assert_eq!(stats.status, "completed");
        assert_eq!(stats.snapshot_path, None);
        assert_eq!(stats.runtime_ms(), 1000);
    }

    #[test]
    fn test_protobuf_conversion_to_proto() {
        let stats = UnitStats::new(
            "u1".into(),
            "s1".into(),
            "coder".into(),
            1000,
            2000,
        )
        .with_memory(120)
        .with_status("killed")
        .with_snapshot("/tmp/snapshot");

        let proto: ProtoUnitStats = stats.clone().into();

        assert_eq!(proto.unit_id, "u1");
        assert_eq!(proto.slice_id, "s1");
        assert_eq!(proto.loop_type, "coder");
        assert_eq!(proto.spawned_at_ms, 1000);
        assert_eq!(proto.died_at_ms, 2000);
        assert_eq!(proto.peak_memory_mb, 120);
        assert_eq!(proto.status, "killed");
        assert_eq!(proto.snapshot_path, "/tmp/snapshot");
        let empty: Vec<u8> = vec![];
        assert_eq!(proto.stats_blob, empty);
    }

    #[test]
    fn test_protobuf_roundtrip() {
        let proto = ProtoUnitStats {
            unit_id: "u1".into(),
            slice_id: "s1".into(),
            loop_type: "coder".into(),
            spawned_at_ms: 1000,
            died_at_ms: 2000,
            peak_memory_mb: 120,
            status: "completed".into(),
            snapshot_path: "/path".into(),
            stats_blob: vec![1, 2, 3],
        };

        let stats: UnitStats = proto.clone().into();
        let back: ProtoUnitStats = stats.into();

        assert_eq!(back.unit_id, proto.unit_id);
        assert_eq!(back.slice_id, proto.slice_id);
        assert_eq!(back.loop_type, proto.loop_type);
        assert_eq!(back.spawned_at_ms, proto.spawned_at_ms);
        assert_eq!(back.died_at_ms, proto.died_at_ms);
        assert_eq!(back.peak_memory_mb, proto.peak_memory_mb);
        assert_eq!(back.status, proto.status);
        assert_eq!(back.snapshot_path, proto.snapshot_path);
        let empty: Vec<u8> = vec![];
        assert_eq!(back.stats_blob, empty);
    }
}
