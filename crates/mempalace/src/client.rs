use sqlx::{SqlitePool, FromRow};
use sqlx::sqlite::SqliteConnectOptions;
use crate::error::Result;
use crate::stats::UnitStats;
use crate::mock::StatsQueryBuilder;
use crate::aggregate::{LoopTypeAgg, StatusAgg, MemoryBucket};

#[cfg(feature = "milvus-compat")]
use milvus_brain::{MemoryStore, ResearchFinding, QueryBuilder as MilvusQueryBuilder};

pub struct MempalaceClient {
    pool: SqlitePool,
}

impl MempalaceClient {
    pub async fn connect(database_path: &str) -> Result<Self> {
        let options = if database_path.starts_with("sqlite:") {
            use std::str::FromStr;
            SqliteConnectOptions::from_str(database_path)
                .map_err(|e| sqlx::Error::Configuration(e.into()))?
                .create_if_missing(true)
        } else {
            SqliteConnectOptions::new()
                .filename(database_path)
                .create_if_missing(true)
        };

        let pool = SqlitePool::connect_with(options).await?;
        
        // Run schema migration
        let schema = include_str!("schema.sql");
        sqlx::query(schema).execute(&pool).await?;
        
        Ok(Self { pool })
    }

    pub async fn write_stats(&self, stats: UnitStats) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO unit_stats (unit_id, slice_id, loop_type, spawned_at_ms, died_at_ms, peak_memory_mb, status, snapshot_path)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(unit_id) DO UPDATE SET
              slice_id = excluded.slice_id,
              loop_type = excluded.loop_type,
              spawned_at_ms = excluded.spawned_at_ms,
              died_at_ms = excluded.died_at_ms,
              peak_memory_mb = excluded.peak_memory_mb,
              status = excluded.status,
              snapshot_path = excluded.snapshot_path
            "#
        )
        .bind(&stats.unit_id)
        .bind(&stats.slice_id)
        .bind(&stats.loop_type)
        .bind(stats.spawned_at_ms as i64)
        .bind(stats.died_at_ms as i64)
        .bind(stats.peak_memory_mb.map(|m| m as i64))
        .bind(&stats.status)
        .bind(&stats.snapshot_path)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    pub async fn get_unit(&self, unit_id: &str) -> Result<Option<UnitStats>> {
        let row = sqlx::query_as::<_, UnitStatsRow>(
            "SELECT unit_id, slice_id, loop_type, spawned_at_ms, died_at_ms, peak_memory_mb, status, snapshot_path FROM unit_stats WHERE unit_id = ?"
        )
        .bind(unit_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into_stats()))
    }

    pub async fn query_stats(&self, query: StatsQueryBuilder) -> Result<Vec<UnitStats>> {
        let mut sql = String::from(
            "SELECT unit_id, slice_id, loop_type, spawned_at_ms, died_at_ms, peak_memory_mb, status, snapshot_path FROM unit_stats WHERE 1=1"
        );
        
        let mut binds: Vec<&str> = Vec::new();
        
        if let Some(slice_id) = &query.slice_id {
            sql.push_str(" AND slice_id = ?");
            binds.push(slice_id);
        }
        if let Some(loop_type) = &query.loop_type {
            sql.push_str(" AND loop_type = ?");
            binds.push(loop_type);
        }
        if let Some(status) = &query.status {
            sql.push_str(" AND status = ?");
            binds.push(status);
        }

        let mut q = sqlx::query_as::<_, UnitStatsRow>(&sql);
        for bind in binds {
            q = q.bind(bind);
        }

        let rows = q.fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| r.into_stats()).collect())
    }

    pub async fn query_all(&self) -> Result<Vec<UnitStats>> {
        let rows = sqlx::query_as::<_, UnitStatsRow>(
            "SELECT unit_id, slice_id, loop_type, spawned_at_ms, died_at_ms, peak_memory_mb, status, snapshot_path FROM unit_stats"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.into_stats()).collect())
    }

    pub async fn clear(&self) -> Result<()> {
        sqlx::query("DELETE FROM unit_stats").execute(&self.pool).await?;
        Ok(())
    }

    pub async fn aggregate_by_loop_type(&self) -> Result<Vec<LoopTypeAgg>> {
        crate::aggregate::AggregationQueries::aggregate_by_loop_type(&self.pool).await
    }

    pub async fn aggregate_by_status(&self) -> Result<Vec<StatusAgg>> {
        crate::aggregate::AggregationQueries::aggregate_by_status(&self.pool).await
    }

    pub async fn memory_distribution(&self) -> Result<Vec<MemoryBucket>> {
        crate::aggregate::AggregationQueries::memory_distribution(&self.pool).await
    }
}

/// Optional MemoryStore trait implementation for milvus compatibility
/// mempalace doesn't store research findings - these are no-ops
#[cfg(feature = "milvus-compat")]
#[async_trait::async_trait]
impl MemoryStore for MempalaceClient {
    async fn write_finding(&self, _finding: ResearchFinding) -> milvus_brain::Result<()> {
        // mempalace stores unit stats, not research findings
        Ok(())
    }

    async fn search(&self, _query: MilvusQueryBuilder) -> milvus_brain::Result<Vec<ResearchFinding>> {
        // mempalace doesn't support similarity search
        Ok(vec![])
    }

    async fn delete_finding(&self, _finding_id: &str) -> milvus_brain::Result<()> {
        // mempalace doesn't store research findings
        Ok(())
    }
}

#[derive(FromRow)]
struct UnitStatsRow {
    unit_id: String,
    slice_id: String,
    loop_type: String,
    spawned_at_ms: i64,
    died_at_ms: i64,
    peak_memory_mb: Option<i64>,
    status: String,
    snapshot_path: Option<String>,
}

impl UnitStatsRow {
    fn into_stats(self) -> UnitStats {
        UnitStats {
            unit_id: self.unit_id,
            slice_id: self.slice_id,
            loop_type: self.loop_type,
            spawned_at_ms: self.spawned_at_ms as u64,
            died_at_ms: self.died_at_ms as u64,
            peak_memory_mb: self.peak_memory_mb.map(|m| m as u32),
            status: self.status,
            snapshot_path: self.snapshot_path,
        }
    }
}
