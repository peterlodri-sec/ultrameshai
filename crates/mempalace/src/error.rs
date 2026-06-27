use thiserror::Error;

#[derive(Error, Debug)]
pub enum MempalaceError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("query error: {0}")]
    Query(String),
}

pub type Result<T> = std::result::Result<T, MempalaceError>;
