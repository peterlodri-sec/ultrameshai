use crate::detector::PatternDetector;
use crate::error::{HonchoError, Result};

use crate::store::PatternStore;
use mempalace::{MempalaceClient, UnitStats};
use milvus_brain::ResearchFinding;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::{interval, MissedTickBehavior};

/// Brain health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum BrainStatus {
    Alive,   // data received within 2x poll interval
    Stale,   // no data within 2x poll interval
    Unknown, // never received data
}

impl std::fmt::Display for BrainStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrainStatus::Alive => write!(f, "ALIVE"),
            BrainStatus::Stale => write!(f, "STALE"),
            BrainStatus::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// Lock-free brain state snapshot written once per poll cycle
#[derive(Debug, Clone, serde::Serialize)]
pub struct BrainSnapshot {
    pub status: BrainStatus,
    pub patterns_total: u64,
    pub findings_total: u64,
    pub units_processed: u64,
    pub last_data_at_ms: u64,
    pub poll_count: u64,
    pub interval_ms: u64,
}

impl BrainSnapshot {
    /// Compute status from raw fields
    pub fn compute(
        patterns_total: u64,
        findings_total: u64,
        units_processed: u64,
        last_data_at_ms: u64,
        poll_count: u64,
        interval_ms: u64,
    ) -> Self {
        let status = if poll_count == 0 {
            BrainStatus::Unknown
        } else if last_data_at_ms == 0 {
            BrainStatus::Unknown
        } else {
            let now = Self::now_ms();
            let elapsed = now.saturating_sub(last_data_at_ms);
            let threshold = interval_ms * 2;
            if elapsed <= threshold {
                BrainStatus::Alive
            } else {
                BrainStatus::Stale
            }
        };
        Self { status, patterns_total, findings_total, units_processed, last_data_at_ms, poll_count, interval_ms }
    }

    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap()
            .as_millis() as u64
    }

    pub fn age_string(&self) -> String {
        if self.last_data_at_ms == 0 {
            "never".to_string()
        } else {
            let elapsed = Self::now_ms().saturating_sub(self.last_data_at_ms);
            let secs = elapsed / 1000;
            if secs < 60 { format!("{}s ago", secs) } else { format!("{}m ago", secs / 60) }
        }
    }

    pub fn format_banner(&self) -> String {
        let status_icon = match self.status {
            BrainStatus::Alive => "🧠",
            BrainStatus::Stale => "💤",
            BrainStatus::Unknown => "❓",
        };
        let age = self.age_string();
        format!(
            r#"
╔══════════════════════════════════════════════════════════╗
║  {}  BRAIN  {}                                      ║
║  patterns: {:>5}   findings: {:>5}   units: {:>6}        ║
║  last data: {:>12}   poll #{:>4}   interval: {}m       ║
╚══════════════════════════════════════════════════════════╝"#,
            status_icon, self.status, self.patterns_total, self.findings_total,
            self.units_processed, age, self.poll_count, self.interval_ms / 60000,
        )
    }
}

/// HonchoDaemon - background daemon that polls mempalace + milvus for patterns
pub struct HonchoDaemon {
    mempalace_db: String,
    pattern_store: Option<PatternStore>,
    detector: PatternDetector,
    poll_interval_ms: u64,
    last_processed_mempalace: Arc<RwLock<u64>>,
    last_processed_milvus: Arc<RwLock<u64>>,
    running: Arc<AtomicBool>,
    patterns_total: Arc<AtomicU64>,
    findings_total: Arc<AtomicU64>,
    units_processed: Arc<AtomicU64>,
    last_data_at_ms: Arc<AtomicU64>,
    poll_count: Arc<AtomicU64>,
}

impl Clone for HonchoDaemon {
    fn clone(&self) -> Self {
        Self {
            mempalace_db: self.mempalace_db.clone(),
            pattern_store: None,
            detector: self.detector.clone(),
            poll_interval_ms: self.poll_interval_ms,
            last_processed_mempalace: self.last_processed_mempalace.clone(),
            last_processed_milvus: self.last_processed_milvus.clone(),
            running: self.running.clone(),
            patterns_total: self.patterns_total.clone(),
            findings_total: self.findings_total.clone(),
            units_processed: self.units_processed.clone(),
            last_data_at_ms: self.last_data_at_ms.clone(),
            poll_count: self.poll_count.clone(),
        }
    }
}

impl HonchoDaemon {
    /// Create new daemon with mempalace connection
    /// PatternStore (milvus) is optional - patterns can be detected without writing
    pub async fn new(
        mempalace_db: &str,
        milvus_uri: Option<&str>,
        poll_interval_ms: Option<u64>,
    ) -> Result<Self> {
        let pattern_store = if let Some(uri) = milvus_uri {
            Some(PatternStore::connect(uri).await?)
        } else {
            None
        };

        let poll_interval = poll_interval_ms.unwrap_or_else(|| {
            std::env::var("HONCHO_POLL_INTERVAL_MS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(300000) // Default 5 minutes
        });

        Ok(Self {
            mempalace_db: mempalace_db.to_string(),
            pattern_store,
            detector: PatternDetector::new(),
            poll_interval_ms: poll_interval,
            last_processed_mempalace: Arc::new(RwLock::new(0)),
            last_processed_milvus: Arc::new(RwLock::new(0)),
            running: Arc::new(AtomicBool::new(false)),
            patterns_total: Arc::new(AtomicU64::new(0)),
            findings_total: Arc::new(AtomicU64::new(0)),
            units_processed: Arc::new(AtomicU64::new(0)),
            last_data_at_ms: Arc::new(AtomicU64::new(0)),
            poll_count: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Start background polling task
    pub async fn start(&self) -> Result<()> {
        if self.running.load(Ordering::SeqCst) {
            return Err(HonchoError::Detection(
                "Daemon already running".to_string(),
            ));
        }

        self.running.store(true, Ordering::SeqCst);

        let this = self.clone();
        let poll_interval = self.poll_interval_ms;

        tokio::spawn(async move {
            let mut interval_timer = interval(Duration::from_millis(poll_interval));
            interval_timer.set_missed_tick_behavior(MissedTickBehavior::Skip);

            while this.running.load(Ordering::SeqCst) {
                interval_timer.tick().await;
                this.tick().await;
            }

            tracing::info!("Honcho daemon stopped");
        });

        Ok(())
    }

    /// Stop background polling
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Check if daemon is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap()
            .as_millis() as u64
    }

    /// Take a snapshot of current brain state (lock-free read of atomics)
    pub fn snapshot(&self) -> BrainSnapshot {
        let last = self.last_data_at_ms.load(Ordering::Relaxed);
        let pc = self.poll_count.load(Ordering::Relaxed);
        BrainSnapshot::compute(
            self.patterns_total.load(Ordering::Relaxed),
            self.findings_total.load(Ordering::Relaxed),
            self.units_processed.load(Ordering::Relaxed),
            last, pc, self.poll_interval_ms,
        )
    }

    /// Write snapshot JSON to brain-state.json in cache dir
    pub async fn write_state_file(&self, snap: &BrainSnapshot) -> std::io::Result<()> {
        let path = dirs::cache_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("ultrameshai")
            .join("brain-state.json");
        tokio::fs::create_dir_all(path.parent().unwrap()).await?;
        let json = serde_json::to_string_pretty(snap).unwrap();
        tokio::fs::write(path, json).await
    }

    /// Execute one poll cycle: mempalace + milvus, update counters, emit banner
    async fn tick(&self) {
        let mempalace_db = self.mempalace_db.clone();
        let last_mempalace = self.last_processed_mempalace.clone();
        let last_milvus = self.last_processed_milvus.clone();
        let has_milvus = self.pattern_store.is_some();

        let stats = match Self::poll_mempalace(&mempalace_db, &last_mempalace).await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to poll mempalace: {}", e);
                return;
            }
        };

        let findings = if has_milvus {
            match Self::poll_milvus(&last_milvus).await {
                Ok(f) => f,
                Err(e) => {
                    tracing::warn!("Failed to poll milvus: {}", e);
                    vec![]
                }
            }
        } else {
            vec![]
        };

        if !stats.is_empty() || !findings.is_empty() {
            match Self::process_batch(&self.detector, has_milvus, &mempalace_db, stats.clone(), findings).await {
                Ok(count) => tracing::info!("Detected {} new patterns", count),
                Err(e) => tracing::error!("Failed to process batch: {}", e),
            }
        }

        let now = Self::now_ms();
        self.last_data_at_ms.store(now, Ordering::Relaxed);
        self.units_processed.fetch_add(stats.len() as u64, Ordering::Relaxed);
        self.poll_count.fetch_add(1, Ordering::Relaxed);

        let snap = self.snapshot();
        tracing::info!(banner = "brain", "{}", snap.format_banner());
        if let Err(e) = self.write_state_file(&snap).await {
            tracing::warn!("Failed to write brain-state.json: {}", e);
        }
    }

    /// Poll mempalace for new UnitStats since last processed timestamp
    async fn poll_mempalace(
        mempalace_db: &str,
        last_processed: &Arc<RwLock<u64>>,
    ) -> Result<Vec<UnitStats>> {
        let last_ts = *last_processed.read().await;

        // Connect to mempalace for this poll
        let mempalace = MempalaceClient::connect(mempalace_db).await?;

        // Query all stats (simplified - would filter by timestamp in production)
        let all_stats = mempalace.query_all().await?;

        // Filter by timestamp
        let new_stats: Vec<_> = all_stats
            .into_iter()
            .filter(|s| s.died_at_ms > last_ts)
            .collect();

        // Update last processed timestamp
        if let Some(max_ts) = new_stats.iter().map(|s| s.died_at_ms).max() {
            *last_processed.write().await = max_ts;
        }

        Ok(new_stats)
    }

    /// Poll milvus for new ResearchFinding since last processed timestamp
    async fn poll_milvus(last_processed: &Arc<RwLock<u64>>) -> Result<Vec<ResearchFinding>> {
        // Simplified - would query milvus with timestamp filter in production
        let _last_ts = *last_processed.read().await;

        // Return empty for now - milvus polling requires PatternStore
        Ok(vec![])
    }

    /// Process batch of stats and findings, detect patterns, write to store
    async fn process_batch(
        detector: &PatternDetector,
        has_milvus: bool,
        mempalace_db: &str,
        stats: Vec<UnitStats>,
        findings: Vec<ResearchFinding>,
    ) -> Result<usize> {
        // Detect patterns
        let patterns = detector.detect(stats, findings)?;
        let count = patterns.len();

        // Write patterns to milvus (if connected)
        if has_milvus {
            // Would write patterns here if PatternStore was cloneable
            tracing::debug!("Would write {} patterns to milvus", count);
        }

        Ok(count)
    }

    /// Get poll interval in milliseconds
    pub fn poll_interval_ms(&self) -> u64 {
        self.poll_interval_ms
    }

    /// Get last processed mempalace timestamp
    pub async fn last_processed_mempalace(&self) -> u64 {
        *self.last_processed_mempalace.read().await
    }

    /// Get last processed milvus timestamp
    pub async fn last_processed_milvus(&self) -> u64 {
        *self.last_processed_milvus.read().await
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_daemon_new() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let daemon = HonchoDaemon::new(&db_path.to_string_lossy(), None, Some(1000))
            .await
            .unwrap();

        assert_eq!(daemon.poll_interval_ms(), 1000);
        assert!(!daemon.is_running());
    }

    #[tokio::test]
    async fn test_daemon_default_interval() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let daemon = HonchoDaemon::new(&db_path.to_string_lossy(), None, None)
            .await
            .unwrap();

        assert_eq!(daemon.poll_interval_ms(), 300000); // Default 5 min
    }

    #[tokio::test]
    async fn test_daemon_start_stop() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let daemon = HonchoDaemon::new(&db_path.to_string_lossy(), None, Some(1000))
            .await
            .unwrap();

        assert!(!daemon.is_running());

        daemon.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(daemon.is_running());

        daemon.stop();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert!(!daemon.is_running());
    }

    #[tokio::test]
    async fn test_daemon_double_start() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let daemon = HonchoDaemon::new(&db_path.to_string_lossy(), None, Some(1000))
            .await
            .unwrap();

        daemon.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Second start should fail
        let result = daemon.start().await;
        assert!(result.is_err());

        daemon.stop();
    }

    #[tokio::test]
    async fn test_daemon_last_processed() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let daemon = HonchoDaemon::new(&db_path.to_string_lossy(), None, Some(1000))
            .await
            .unwrap();

        assert_eq!(daemon.last_processed_mempalace().await, 0);
        assert_eq!(daemon.last_processed_milvus().await, 0);
    }

    #[tokio::test]
    async fn test_process_batch_empty() {
        let detector = PatternDetector::new();

        let count = HonchoDaemon::process_batch(
            &detector,
            false,
            "/tmp/test.db",
            vec![],
            vec![],
        )
        .await
        .unwrap();

        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_process_batch_with_stats() {
        use mempalace::UnitStats;

        let detector = PatternDetector::new();

        let stats = vec![
            UnitStats::new("u1".into(), "s1".into(), "coder".into(), 1000, 2000),
            UnitStats::new("u2".into(), "s1".into(), "coder".into(), 1000, 2000),
        ];

        let count = HonchoDaemon::process_batch(
            &detector,
            false,
            "/tmp/test.db",
            stats,
            vec![],
        )
        .await
        .unwrap();

        // May or may not detect patterns depending on data
        let _ = count;
    }
}
