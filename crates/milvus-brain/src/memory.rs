use crate::error::Result;
use crate::query::QueryBuilder;
use crate::write::ResearchFinding;

/// MemoryStore - abstraction for research memory backends
/// Implement this trait to swap between Milvus, mempalace, honcho, or in-memory stores
#[async_trait::async_trait]
pub trait MemoryStore: Send + Sync {
    /// Write a single research finding
    async fn write_finding(&self, finding: ResearchFinding) -> Result<()>;

    /// Search findings with similarity + metadata filters
    async fn search(&self, query: QueryBuilder) -> Result<Vec<ResearchFinding>>;

    /// Delete a finding by ID
    async fn delete_finding(&self, finding_id: &str) -> Result<()>;
}
