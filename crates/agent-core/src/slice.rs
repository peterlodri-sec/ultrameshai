//! Slice protocol — task splitting via protobuf SliceAssign → UnitStats.


use loop_engineering_transport::proto;

/// Task slice specification decoded from protobuf.
pub struct SliceSpec {
    pub slice_id: String,
    pub task_id: String,
    pub loop_type: String,
    pub spec: String,
    pub dependencies: Vec<String>,
    pub execution_mode: String,
}

impl SliceSpec {
    /// Decode from protobuf SliceAssign message.
    pub fn from_proto(msg: &proto::SliceAssign) -> Self {
        Self {
            slice_id: msg.slice_id.clone(),
            task_id: msg.task_id.clone(),
            loop_type: msg.loop_type.clone(),
            spec: msg.spec.clone(),
            dependencies: msg.dependencies.clone(),
            execution_mode: msg.execution_mode.clone(),
        }
    }
}

/// Unit stats to emit after slice completion.
pub struct UnitStats {
    pub unit_id: String,
    pub slice_id: String,
    pub loop_type: String,
    pub spawned_at_ms: u64,
    pub died_at_ms: u64,
    pub peak_memory_mb: u32,
    pub status: String,
    pub snapshot_path: String,
    pub stats_blob: Vec<u8>,
}

impl UnitStats {
    /// Encode to protobuf UnitStats message.
    pub fn to_proto(&self) -> proto::UnitStats {
        proto::UnitStats {
            unit_id: self.unit_id.clone(),
            slice_id: self.slice_id.clone(),
            loop_type: self.loop_type.clone(),
            spawned_at_ms: self.spawned_at_ms,
            died_at_ms: self.died_at_ms,
            peak_memory_mb: self.peak_memory_mb,
            status: self.status.clone(),
            snapshot_path: self.snapshot_path.clone(),
            stats_blob: self.stats_blob.clone(),
        }
    }
}
