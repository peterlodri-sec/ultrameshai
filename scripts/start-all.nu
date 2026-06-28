# scripts/start-all.nu
# Oneshot command to boot the entire UltraMeshAI stack in Zellij

def main [] {
    print "🚀 Starting UltraMeshAI Stack..."
    
    # 1. Start Portail proxy in a background Zellij pane (split down)
    print "  -> Launching Portail..."
    zellij run -d down --name "Portail" -- sh -c "PORTAIL_LOG_DIR=/Users/lodripeter/workspace/peterlodri-sec/portail/logs cargo run --manifest-path /Users/lodripeter/workspace/peterlodri-sec/portail/Cargo.toml serve"
    
    # 2. Start Node Registry in a background Zellij pane (split down)
    print "  -> Launching Node Registry..."
    zellij run -d down --name "Node Registry" -- sh -c "HEARTBEAT_SECRET=todolist_secret cargo run --bin node-registry"
    
    # 3. Start OpenCode TUI in the main pane
    print "  -> Launching OpenCode..."
    opencode
}
