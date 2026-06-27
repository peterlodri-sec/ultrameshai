use thiserror::Error;

#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("address parse error: {0}")]
    AddrParse(#[from] std::net::AddrParseError),
    #[error("node not found: {0}")]
    NodeNotFound(String),
    #[error("no node fits requirements (tier={tier}, need={need_mb}MB)")]
    NoFit { tier: String, need_mb: u64 },
}

pub type Result<T> = std::result::Result<T, RegistryError>;