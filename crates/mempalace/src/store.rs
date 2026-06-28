use std::collections::HashMap;
use std::sync::RwLock;
use crate::error::Result;
use crate::stats::UnitStats;
use crate::mock::StatsQueryBuilder;
use crate::aggregate::{LoopTypeAgg, StatusAgg, MemoryBucket};

/// Unified storage trait for agent state and execution metrics.
#[async_trait::async_trait]
pub trait StateStore: Send + Sync {
    async fn write_stats(&self, stats: UnitStats) -> Result<()>;
    async fn get_unit(&self, unit_id: &str) -> Result<Option<UnitStats>>;
    async fn query_stats(&self, query: StatsQueryBuilder) -> Result<Vec<UnitStats>>;
    async fn query_all(&self) -> Result<Vec<UnitStats>>;
    async fn clear(&self) -> Result<()>;
    async fn aggregate_by_loop_type(&self) -> Result<Vec<LoopTypeAgg>>;
    async fn aggregate_by_status(&self) -> Result<Vec<StatusAgg>>;
    async fn memory_distribution(&self) -> Result<Vec<MemoryBucket>>;
}

/// A thread-safe, in-memory implementation of `StateStore`.
pub struct InMemoryStore {
    stats: RwLock<HashMap<String, UnitStats>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            stats: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl StateStore for InMemoryStore {
    async fn write_stats(&self, stats: UnitStats) -> Result<()> {
        let mut map = self.stats.write().unwrap();
        map.insert(stats.unit_id.clone(), stats);
        Ok(())
    }

    async fn get_unit(&self, unit_id: &str) -> Result<Option<UnitStats>> {
        let map = self.stats.read().unwrap();
        Ok(map.get(unit_id).cloned())
    }

    async fn query_stats(&self, query: StatsQueryBuilder) -> Result<Vec<UnitStats>> {
        let map = self.stats.read().unwrap();
        let results: Vec<UnitStats> = map.values()
            .filter(|s| {
                if let Some(slice_id) = &query.slice_id {
                    if s.slice_id != *slice_id { return false; }
                }
                if let Some(loop_type) = &query.loop_type {
                    if s.loop_type != *loop_type { return false; }
                }
                if let Some(status) = &query.status {
                    if s.status != *status { return false; }
                }
                true
            })
            .cloned()
            .collect();
        Ok(results)
    }

    async fn query_all(&self) -> Result<Vec<UnitStats>> {
        let map = self.stats.read().unwrap();
        Ok(map.values().cloned().collect())
    }

    async fn clear(&self) -> Result<()> {
        let mut map = self.stats.write().unwrap();
        map.clear();
        Ok(())
    }

    async fn aggregate_by_loop_type(&self) -> Result<Vec<LoopTypeAgg>> {
        let map = self.stats.read().unwrap();
        let mut counts = HashMap::new();
        let mut runtimes = HashMap::new();
        let mut memories = HashMap::new();
        for s in map.values() {
            *counts.entry(s.loop_type.clone()).or_insert(0) += 1;
            let runtime = s.died_at_ms.saturating_sub(s.spawned_at_ms);
            runtimes.entry(s.loop_type.clone()).or_insert(vec![]).push(runtime);
            if let Some(mem) = s.peak_memory_mb {
                memories.entry(s.loop_type.clone()).or_insert(vec![]).push(mem);
            }
        }
        Ok(counts.into_iter().map(|(k, v)| {
            let avg_runtime = runtimes.get(&k).map(|r| r.iter().sum::<u64>() as f64 / r.len() as f64).unwrap_or(0.0);
            let avg_mem = memories.get(&k).map(|m| m.iter().sum::<u32>() as f64 / m.len() as f64);
            LoopTypeAgg {
                loop_type: k,
                unit_count: v,
                avg_runtime_ms: avg_runtime,
                avg_peak_memory_mb: avg_mem,
            }
        }).collect())
    }

    async fn aggregate_by_status(&self) -> Result<Vec<StatusAgg>> {
        let map = self.stats.read().unwrap();
        let mut counts = HashMap::new();
        for s in map.values() {
            *counts.entry(s.status.clone()).or_insert(0) += 1;
        }
        Ok(counts.into_iter().map(|(k, v)| StatusAgg { status: k, unit_count: v }).collect())
    }

    async fn memory_distribution(&self) -> Result<Vec<MemoryBucket>> {
        let map = self.stats.read().unwrap();
        let mut buckets = vec![
            MemoryBucket { memory_bucket: "0-50MB".into(), unit_count: 0 },
            MemoryBucket { memory_bucket: "51-100MB".into(), unit_count: 0 },
            MemoryBucket { memory_bucket: "101-150MB".into(), unit_count: 0 },
            MemoryBucket { memory_bucket: ">150MB".into(), unit_count: 0 },
        ];
        for s in map.values() {
            if let Some(mem) = s.peak_memory_mb {
                if mem <= 50 { buckets[0].unit_count += 1; }
                else if mem <= 100 { buckets[1].unit_count += 1; }
                else if mem <= 150 { buckets[2].unit_count += 1; }
                else { buckets[3].unit_count += 1; }
            }
        }
        Ok(buckets)
    }
}
