pub mod registry;
pub mod heartbeat;
pub mod error;
pub mod types;
pub mod discovery;
pub mod handler;
pub mod crypto;
pub mod background;

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/loop_engineering.rs"));
}

pub use error::{RegistryError, Result};
pub use types::{NodeMetadata, NodeStatus, NodeEntry, HeartbeatRequest, HealthResponse};
pub use registry::NodeRegistry;
pub use discovery::TailscaleDiscovery;
pub use handler::create_router;
pub use background::spawn_background_tasks;