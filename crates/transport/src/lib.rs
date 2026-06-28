pub mod framed;
pub mod error;
pub mod uds;
pub mod mmap;

// Re-export generated protobuf types
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/loop_engineering.rs"));
}

pub use error::{TransportError, Result};