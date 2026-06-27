use thiserror::Error;

#[derive(Error, Debug)]
pub enum MilvusError {
    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Schema error: {0}")]
    Schema(String),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Write error: {0}")]
    Write(String),

    #[error("Delete error: {0}")]
    Delete(String),

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Mock error: {0}")]
    Mock(String),
}

pub type Result<T> = std::result::Result<T, MilvusError>;
