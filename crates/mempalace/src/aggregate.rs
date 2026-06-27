use sqlx::FromRow;

/// Aggregation result by loop type
#[derive(FromRow, Debug)]
pub struct LoopTypeAgg {
    pub loop_type: String,
    pub unit_count: i64,
    pub avg_runtime_ms: f64,
    pub avg_peak_memory_mb: Option<f64>,
}

/// Aggregation result by status
#[derive(FromRow, Debug)]
pub struct StatusAgg {
    pub status: String,
    pub unit_count: i64,
}

/// Memory distribution bucket
#[derive(FromRow, Debug)]
pub struct MemoryBucket {
    pub memory_bucket: String,
    pub unit_count: i64,
}

/// Aggregation queries for mempalace
pub struct AggregationQueries;

impl AggregationQueries {
    /// Aggregate stats by loop type
    pub async fn aggregate_by_loop_type(
        pool: &sqlx::SqlitePool,
    ) -> Result<Vec<LoopTypeAgg>, crate::error::MempalaceError> {
        let results = sqlx::query_as::<_, LoopTypeAgg>(
            r#"
            SELECT 
              loop_type,
              COUNT(*) as unit_count,
              AVG(died_at_ms - spawned_at_ms) as avg_runtime_ms,
              AVG(peak_memory_mb) as avg_peak_memory_mb
            FROM unit_stats
            GROUP BY loop_type
            "#,
        )
        .fetch_all(pool)
        .await?;
        Ok(results)
    }

    /// Aggregate stats by status
    pub async fn aggregate_by_status(
        pool: &sqlx::SqlitePool,
    ) -> Result<Vec<StatusAgg>, crate::error::MempalaceError> {
        let results = sqlx::query_as::<_, StatusAgg>(
            "SELECT status, COUNT(*) as unit_count FROM unit_stats GROUP BY status",
        )
        .fetch_all(pool)
        .await?;
        Ok(results)
    }

    /// Memory distribution buckets (0-100MB, 100-150MB, 150MB+)
    pub async fn memory_distribution(
        pool: &sqlx::SqlitePool,
    ) -> Result<Vec<MemoryBucket>, crate::error::MempalaceError> {
        let results = sqlx::query_as::<_, MemoryBucket>(
            r#"
            SELECT 
              CASE 
                WHEN peak_memory_mb <= 100 THEN '0-100MB'
                WHEN peak_memory_mb <= 150 THEN '100-150MB'
                ELSE '150MB+'
              END as memory_bucket,
              COUNT(*) as unit_count
            FROM unit_stats
            GROUP BY memory_bucket
            "#,
        )
        .fetch_all(pool)
        .await?;
        Ok(results)
    }
}
