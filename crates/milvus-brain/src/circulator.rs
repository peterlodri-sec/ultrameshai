use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::client::MilvusClient;
use crate::error::Result;
use crate::write::ResearchFinding;

/// Classification of a pruned context entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Classification {
    Unspecified,
    Fact,
    Event,
    Instruction,
    Task,
}

/// A single graph triple extracted from context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphTriple {
    pub subject: String,
    pub predicate: String,
    pub object: String,
}

/// PrunedContextEntry — matches proto definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrunedContextEntry {
    pub session_id: String,
    pub agent_type: String,
    pub message_role: String,
    pub content_hash: String,
    pub classification: Classification,
    pub topic_key: String,
    pub superseded_by: Option<String>,
    pub triples: Vec<GraphTriple>,
    pub residual: String,
    pub timestamp_ms: u64,
}

/// Circulator — in-memory queue with async flush to Milvus or JSONL overflow.
pub struct Circulator {
    milvus_client: Option<MilvusClient>,
    queue: Vec<PrunedContextEntry>,
    overflow_path: Option<PathBuf>,
}

impl Circulator {
    const QUEUE_CAP: usize = 100;

    pub fn new(milvus_client: Option<MilvusClient>) -> Self {
        Self {
            milvus_client,
            queue: Vec::with_capacity(Self::QUEUE_CAP),
            overflow_path: None,
        }
    }

    pub fn set_overflow_path(&mut self, path: &Path) {
        self.overflow_path = Some(path.to_path_buf());
    }

    /// Enqueue entry. Drops oldest if at capacity. Tracks supersession by content_hash.
    pub fn enqueue(&mut self, entry: PrunedContextEntry) {
        let hash = entry.content_hash.clone();

        // Mark any existing entry with same topic_key as superseded
        for existing in &mut self.queue {
            if existing.topic_key == entry.topic_key && existing.content_hash != hash {
                existing.superseded_by = Some(hash.clone());
            }
        }

        // Drop oldest if at capacity
        if self.queue.len() >= Self::QUEUE_CAP {
            self.queue.drain(0..self.queue.len() - Self::QUEUE_CAP + 1);
        }

        self.queue.push(entry);
    }

    pub fn queue(&self) -> &[PrunedContextEntry] {
        &self.queue
    }

    /// Flush queued entries to Milvus or spill to JSONL.
    pub async fn flush(&mut self) -> Result<()> {
        if self.queue.is_empty() {
            return Ok(());
        }

        let entries: Vec<PrunedContextEntry> = self.queue.drain(..).collect();
        let count = entries.len();

        // Try Milvus write
        let milvus_ok = if let Some(ref client) = self.milvus_client {
            let findings: Vec<ResearchFinding> = entries
                .iter()
                .map(|e| ResearchFinding {
                    finding_id: format!("circ-{}", uuid::Uuid::new_v4()),
                    source_agent: e.agent_type.clone(),
                    topic: e.topic_key.clone(),
                    summary: e.residual.clone(),
                    embedding: Vec::new(),
                    tags: vec![format!("{}", e.classification as i32)],
                    timestamp_ms: e.timestamp_ms,
                })
                .collect();

            match client.batch_write(findings).await {
                Ok(()) => {
                    tracing::info!("flushed {} entries to milvus", count);
                    true
                }
                Err(e) => {
                    tracing::warn!("milvus flush failed: {}, spilling to JSONL", e);
                    false
                }
            }
        } else {
            false
        };

        if !milvus_ok {
            // Restore entries for spill
            self.queue = entries;
            self.spill_to_jsonl().await?;
            self.queue.clear();
            tracing::info!("spilled {} entries to JSONL overflow", count);
        }

        Ok(())
    }

    async fn spill_to_jsonl(&self) -> Result<()> {
        let path = self.get_overflow_path();
        for entry in &self.queue {
            let line = serde_json::to_string(entry)
                .map_err(|e| crate::error::MilvusError::Write(format!("JSONL serialize: {}", e)))?;
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .map_err(|e| crate::error::MilvusError::Write(format!("open overflow: {}", e)))?
                .write_all(format!("{}\n", line).as_bytes())
                .map_err(|e| crate::error::MilvusError::Write(format!("write overflow: {}", e)))?;
        }
        Ok(())
    }

    fn get_overflow_path(&self) -> PathBuf {
        if let Some(ref p) = self.overflow_path {
            return p.clone();
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(format!("{}/.cache/ultrameshai/overflow-circulator.jsonl", home))
    }

    /// Run flush loop on 30s interval. Consumes self. Spawn with tokio::spawn.
    pub async fn run_flush_loop(mut self) -> Result<()> {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            if let Err(e) = self.flush().await {
                tracing::error!("circulator flush loop error: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(id: &str, topic: &str) -> PrunedContextEntry {
        PrunedContextEntry {
            session_id: "s1".into(),
            agent_type: "test-agent".into(),
            message_role: "user".into(),
            content_hash: id.into(),
            classification: Classification::Fact,
            topic_key: topic.into(),
            superseded_by: None,
            triples: vec![],
            residual: format!("residual-{}", id),
            timestamp_ms: 1000,
        }
    }

    #[test]
    fn test_enqueue_and_queue() {
        let mut circ = Circulator::new(None);
        circ.enqueue(make_entry("h1", "topic-a"));
        assert_eq!(circ.queue().len(), 1);
        assert_eq!(circ.queue()[0].content_hash, "h1");
    }

    #[test]
    fn test_supersession_tracking() {
        let mut circ = Circulator::new(None);
        circ.enqueue(make_entry("h1", "topic-a"));
        circ.enqueue(make_entry("h2", "topic-a"));
        assert_eq!(circ.queue()[0].superseded_by, Some("h2".into()));
        assert_eq!(circ.queue()[1].superseded_by, None);
    }

    #[test]
    fn test_queue_capacity() {
        let mut circ = Circulator::new(None);
        for i in 0..110 {
            circ.enqueue(make_entry(&format!("h{}", i), "topic"));
        }
        assert!(circ.queue().len() <= Circulator::QUEUE_CAP);
    }

    #[tokio::test]
    async fn test_flush_empty() {
        let circ = Circulator::new(None);
        assert!(circ.queue().is_empty());
    }

    #[tokio::test]
    async fn test_spill_to_jsonl() {
        let tmp = std::env::temp_dir().join(format!("circ-test-{}.jsonl", uuid::Uuid::new_v4()));
        let mut circ = Circulator::new(None);
        circ.set_overflow_path(&tmp);
        circ.enqueue(make_entry("h1", "t1"));

        // Manually trigger spill via flush (no milvus client)
        circ.flush().await.unwrap();
        assert!(tmp.exists());
        let content = std::fs::read_to_string(&tmp).unwrap();
        assert!(content.contains("h1"));
        std::fs::remove_file(&tmp).ok();
    }
}
