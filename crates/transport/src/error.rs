use thiserror::Error;

#[derive(Error, Debug)]
pub enum TransportError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("protobuf decode error: {0}")]
    Decode(#[from] prost::DecodeError),
    #[error("protobuf encode error: {0}")]
    Encode(#[from] prost::EncodeError),
    #[error("message too large: {size} bytes (max {max})")]
    MessageTooLarge { size: usize, max: usize },
    #[error("connection closed")]
    ConnectionClosed,
}

pub type Result<T> = std::result::Result<T, TransportError>;