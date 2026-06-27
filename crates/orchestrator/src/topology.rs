use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use arc_swap::ArcSwap;
use notify::{Watcher, RecommendedWatcher, RecursiveMode, Event};
use tokio::sync::mpsc;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum NodeRole {
    Compute,
    Worker,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeType {
    Bun,
    RustNative,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct NodeProfile {
    pub ip: IpAddr,
    pub role: NodeRole,
    pub capabilities: Vec<String>,
    pub runtime: RuntimeType,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ClusterTopology {
    pub coordinator_ip: IpAddr,
    pub coordinator_port: u16,
    pub nodes: Vec<NodeProfile>,
}

pub struct ClusterTopologyRouter {
    file_path: PathBuf,
    topology: ArcSwap<ClusterTopology>,
}

impl ClusterTopologyRouter {
    /// Load the initial topology from path
    pub fn new(path: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        let file_path = path.as_ref().to_path_buf();
        let content = std::fs::read_to_string(&file_path)?;
        let initial_topo: ClusterTopology = toml::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        Ok(Self {
            file_path,
            topology: ArcSwap::new(Arc::new(initial_topo)),
        })
    }

    /// Read path: lock-free lookup
    pub fn find_optimal_node(&self, capability: &str) -> Option<NodeProfile> {
        let current = self.topology.load();
        
        current.nodes.iter()
            .filter(|node| node.capabilities.iter().any(|c| c == capability))
            .min_by_key(|node| match node.role {
                NodeRole::Compute => 0, // Prefer Compute
                NodeRole::Worker => 1,  // Over Worker
            })
            .cloned()
    }

    /// Get current snapshot of global topology
    pub fn get_topology(&self) -> Arc<ClusterTopology> {
        self.topology.load_full()
    }

    /// Start file watcher task to reload on file modifications
    pub fn start_watcher(self: Arc<Self>) -> Result<(), std::io::Error> {
        let (tx, mut rx) = mpsc::channel::<Event>(10);
        let path = self.file_path.clone();

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = tx.blocking_send(event);
                }
            },
            notify::Config::default(),
        ).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        if let Some(parent) = path.parent() {
            watcher.watch(parent, RecursiveMode::NonRecursive)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        } else {
            watcher.watch(Path::new("."), RecursiveMode::NonRecursive)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        }

        // Spawn background task to monitor events
        tokio::spawn(async move {
            let _watcher = watcher; // keep watcher alive
            while let Some(event) = rx.recv().await {
                // Reload on modify, create, or rename events targeting our file
                if event.kind.is_modify() || event.kind.is_create() {
                    let matches_file = event.paths.iter().any(|p| {
                        p.file_name() == path.file_name()
                    });
                    if matches_file {
                        if let Ok(content) = tokio::fs::read_to_string(&path).await {
                            if let Ok(new_topo) = toml::from_str::<ClusterTopology>(&content) {
                                self.topology.store(Arc::new(new_topo));
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }
}

// Optional Rig Tool Integration
#[cfg(feature = "rig")]
pub mod rig_tool {
    use super::*;
    use rig_core::tool::Tool;
    use schemars::JsonSchema;

    #[derive(Deserialize, JsonSchema)]
    pub struct RouteArgs {
        /// The capability required for the node (e.g. "cuda", "sensors")
        pub capability: String,
    }

    #[derive(Serialize, JsonSchema)]
    pub struct RouteOutput {
        /// Whether a suitable target node was found
        pub found: bool,
        /// The matching node profile if found
        pub node: Option<NodeProfile>,
    }

    pub struct ClusterRouterTool {
        router: Arc<ClusterTopologyRouter>,
    }

    impl ClusterRouterTool {
        pub fn new(router: Arc<ClusterTopologyRouter>) -> Self {
            Self { router }
        }
    }

    #[derive(Debug, thiserror::Error)]
    #[error("Routing tool error: {0}")]
    pub struct RouterToolError(String);

    impl Tool for ClusterRouterTool {
        const NAME: &'static str = "cluster_router";
        type Error = RouterToolError;
        type Args = RouteArgs;
        type Output = RouteOutput;

        async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
            let node = self.router.find_optimal_node(&args.capability);
            Ok(RouteOutput {
                found: node.is_some(),
                node,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn make_test_toml() -> &'static str {
        r#"
        coordinator_ip = "100.64.0.1"
        coordinator_port = 8080

        [[nodes]]
        ip = "100.64.0.10"
        role = "Compute"
        capabilities = ["cuda", "llm-inference"]
        runtime = "RustNative"

        [[nodes]]
        ip = "100.64.0.20"
        role = "Worker"
        capabilities = ["sensors", "arm64"]
        runtime = "Bun"
        "#
    }

    #[test]
    fn test_topology_parsing_and_routing() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", make_test_toml()).unwrap();

        let router = ClusterTopologyRouter::new(file.path()).unwrap();
        
        // Find compute node
        let node = router.find_optimal_node("cuda").unwrap();
        assert_eq!(node.ip, "100.64.0.10".parse::<IpAddr>().unwrap());
        assert_eq!(node.role, NodeRole::Compute);

        // Find worker node
        let node = router.find_optimal_node("sensors").unwrap();
        assert_eq!(node.ip, "100.64.0.20".parse::<IpAddr>().unwrap());
        assert_eq!(node.role, NodeRole::Worker);

        // Capability not present
        assert!(router.find_optimal_node("non-existent").is_none());
    }

    #[tokio::test]
    async fn test_topology_hot_reloading() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", make_test_toml()).unwrap();

        let router = Arc::new(ClusterTopologyRouter::new(file.path()).unwrap());
        router.clone().start_watcher().unwrap();

        // Check initial state
        assert_eq!(
            router.find_optimal_node("cuda").unwrap().ip,
            "100.64.0.10".parse::<IpAddr>().unwrap()
        );

        // Modify TOML: change IP of compute node from 100.64.0.10 to 100.64.0.99
        let modified_toml = r#"
        coordinator_ip = "100.64.0.1"
        coordinator_port = 8080

        [[nodes]]
        ip = "100.64.0.99"
        role = "Compute"
        capabilities = ["cuda", "llm-inference"]
        runtime = "RustNative"

        [[nodes]]
        ip = "100.64.0.20"
        role = "Worker"
        capabilities = ["sensors", "arm64"]
        runtime = "Bun"
        "#;

        // Overwrite file
        let mut file_handle = std::fs::File::create(file.path()).unwrap();
        write!(file_handle, "{}", modified_toml).unwrap();
        file_handle.sync_all().unwrap();

        // Wait up to 2 seconds for watcher to propagate change
        let mut success = false;
        for _ in 0..20 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if let Some(node) = router.find_optimal_node("cuda") {
                if node.ip == "100.64.0.99".parse::<IpAddr>().unwrap() {
                    success = true;
                    break;
                }
            }
        }

        assert!(success, "Topology did not hot-reload dynamically!");
    }
}
