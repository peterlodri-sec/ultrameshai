use thiserror::Error;

#[derive(Error, Debug)]
pub enum HonchoError {
    #[error("database error: {0}")]
    Database(#[from] mempalace::MempalaceError),

    #[error("milvus error: {0}")]
    Milvus(#[from] milvus_brain::MilvusError),

    #[error("pattern detection error: {0}")]
    Detection(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, HonchoError>;
