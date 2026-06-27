use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use arc_swap::ArcSwap;
use notify::{Watcher, RecommendedWatcher, RecursiveMode, Event};
use tokio::sync::mpsc;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "rig", derive(schemars::JsonSchema))]
pub enum NodeRole {
    Compute,
    Worker,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "rig", derive(schemars::JsonSchema))]
pub enum RuntimeType {
    Bun,
    RustNative,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "rig", derive(schemars::JsonSchema))]
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
    registry: ArcSwap<Option<Arc<std::sync::Mutex<loop_engineering_node_registry::registry::NodeRegistry>>>>,
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
            registry: ArcSwap::new(Arc::new(None)),
        })
    }

    /// Associate a live heartbeat node registry to enable dynamic load balancing
    pub fn with_registry(self: Arc<Self>, registry: Arc<std::sync::Mutex<loop_engineering_node_registry::registry::NodeRegistry>>) -> Arc<Self> {
        self.registry.store(Arc::new(Some(registry)));
        self
    }

    /// Read path: lock-free lookup with dynamic load-balancing fallback
    pub fn find_optimal_node(&self, capability: &str) -> Option<NodeProfile> {
        // 1. Try to load-balance using active registry if available
        if let Some(ref reg) = **self.registry.load() {
            if let Ok(registry) = reg.lock() {
                let live_nodes = registry.get_all_nodes();
                if !live_nodes.is_empty() {
                    let best_live = live_nodes.into_iter()
                        .filter(|entry| entry.metadata.capabilities.iter().any(|c| c == capability))
                        .min_by(|a, b| {
                            // Compare by load_avg (lower is better)
                            let load_a = a.metadata.load_avg.unwrap_or(1.0);
                            let load_b = b.metadata.load_avg.unwrap_or(1.0);
                            load_a.partial_cmp(&load_b).unwrap_or(std::cmp::Ordering::Equal)
                        });

                    if let Some(hb) = best_live {
                        tracing::debug!(
                            "Dynamic load-balancer selected node '{}' (load_avg={:?}, memory={}MB) for capability '{}'",
                            hb.metadata.node_id,
                            hb.metadata.load_avg,
                            hb.metadata.memory_mb,
                            capability
                        );
                        
                        let ip = hb.metadata.node_id.parse::<IpAddr>().unwrap_or_else(|_| {
                            let current = self.topology.load();
                            current.nodes.iter()
                                .find(|n| n.ip.to_string() == hb.metadata.node_id)
                                .map(|n| n.ip)
                                .unwrap_or_else(|| "127.0.0.1".parse().unwrap())
                        });

                        let role = NodeRole::Compute; // Simplified

                        return Some(NodeProfile {
                            ip,
                            role,
                            capabilities: hb.metadata.capabilities.clone(),
                            runtime: RuntimeType::RustNative,
                        });
                    }
                }
            }
        }

        // 2. Fallback to static/hot-reloaded topology config
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

        tracing::info!("Initializing topology file watcher for {:?}", path);

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = tx.blocking_send(event);
                } else if let Err(e) = res {
                    tracing::error!("Topology watcher encountered OS event error: {:?}", e);
                }
            },
            notify::Config::default(),
        ).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        if let Some(parent) = path.parent() {
            watcher.watch(parent, RecursiveMode::NonRecursive)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            tracing::debug!("Watching parent directory: {:?}", parent);
        } else {
            watcher.watch(Path::new("."), RecursiveMode::NonRecursive)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            tracing::debug!("Watching current directory");
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
                        tracing::debug!("Topology file change event detected: {:?}", event.kind);
                        match tokio::fs::read_to_string(&path).await {
                            Ok(content) => {
                                match toml::from_str::<ClusterTopology>(&content) {
                                    Ok(new_topo) => {
                                        tracing::info!(
                                            "Topology configuration successfully hot-reloaded: {} nodes mapped",
                                            new_topo.nodes.len()
                                        );
                                        self.topology.store(Arc::new(new_topo));
                                    }
                                    Err(e) => {
                                        tracing::warn!("Ignoring malformed topology update in {:?}: {:?}", path, e);
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to read modified topology file {:?}: {:?}", path, e);
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
    use rig::tool::Tool;
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

        async fn definition(&self, description: String) -> rig::completion::ToolDefinition {
            rig::completion::ToolDefinition {
                name: Self::NAME.to_string(),
                description,
                parameters: serde_json::to_value(schemars::schema_for!(RouteArgs)).unwrap(),
            }
        }

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

    #[cfg(feature = "rig")]
    #[tokio::test]
    async fn test_rig_tool_routing() {
        use rig::tool::Tool;
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", make_test_toml()).unwrap();

        let router = Arc::new(ClusterTopologyRouter::new(file.path()).unwrap());
        let tool = rig_tool::ClusterRouterTool::new(router);

        let args = rig_tool::RouteArgs {
            capability: "cuda".to_string(),
        };

        let output = tool.call(args).await.unwrap();
        assert!(output.found);
        let node = output.node.unwrap();
        assert_eq!(node.ip, "100.64.0.10".parse::<IpAddr>().unwrap());
    }

    #[tokio::test]
    async fn test_dynamic_load_balancing_routing() {
        use loop_engineering_node_registry::registry::NodeRegistry;
        use loop_engineering_node_registry::proto::NodeHeartbeat;

        let file = NamedTempFile::new().unwrap();
        std::fs::write(file.path(), make_test_toml()).unwrap();

        let router = Arc::new(ClusterTopologyRouter::new(file.path()).unwrap());
        
        // Static fallback initially
        let node = router.find_optimal_node("cuda").unwrap();
        assert_eq!(node.ip, "100.64.0.10".parse::<IpAddr>().unwrap());

        // Associate registry with Node 1 (5 units running) and Node 2 (1 unit running)
        let registry = Arc::new(std::sync::Mutex::new(NodeRegistry::new()));
        {
            let mut reg = registry.lock().unwrap();
            reg.register_node(loop_engineering_node_registry::types::NodeEntry::new(
                loop_engineering_node_registry::types::NodeMetadata {
                    node_id: "100.64.0.88".to_string(),
                    capabilities: vec!["cuda".to_string()],
                    memory_mb: 8000,
                    load_avg: Some(0.5),
                    region: None,
                }
            ));
            reg.register_node(loop_engineering_node_registry::types::NodeEntry::new(
                loop_engineering_node_registry::types::NodeMetadata {
                    node_id: "100.64.0.99".to_string(),
                    capabilities: vec!["cuda".to_string()],
                    memory_mb: 8000,
                    load_avg: Some(0.3),
                    region: None,
                }
            ));
        }

        let router = router.with_registry(registry.clone());

        // Should route to Node 2 (100.64.0.99) since it has lower load
        let node = router.find_optimal_node("cuda").unwrap();
        assert_eq!(node.ip, "100.64.0.99".parse::<IpAddr>().unwrap());

        // Update Node 1 to have lower load -> should route to Node 1 now
        {
            let mut reg = registry.lock().unwrap();
            reg.register_node(loop_engineering_node_registry::types::NodeEntry::new(
                loop_engineering_node_registry::types::NodeMetadata {
                    node_id: "100.64.0.88".to_string(),
                    capabilities: vec!["cuda".to_string()],
                    memory_mb: 8000,
                    load_avg: Some(0.1),
                    region: None,
                }
            ));
        }

        let node = router.find_optimal_node("cuda").unwrap();
        assert_eq!(node.ip, "100.64.0.88".parse::<IpAddr>().unwrap());
    }
}
