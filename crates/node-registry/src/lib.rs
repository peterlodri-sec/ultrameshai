pub mod registry;
pub mod heartbeat;
pub mod error;

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/loop_engineering.rs"));
}

pub use error::{RegistryError, Result};