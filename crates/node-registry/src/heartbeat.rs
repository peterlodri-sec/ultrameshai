use std::net::{SocketAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};
use tokio::net::UdpSocket;
use prost::Message;
use crate::proto::NodeHeartbeat;
use crate::registry::NodeRegistry;
use crate::error::Result;

/// Broadcasts this node's heartbeat over UDP multicast.
pub struct HeartbeatBroadcaster {
    socket: UdpSocket,
    addr: SocketAddr,
}

impl HeartbeatBroadcaster {
    pub async fn new(multicast_addr: &str) -> Result<Self> {
        let addr: SocketAddr = multicast_addr.parse()?;
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.set_multicast_loop_v4(true)?;
        Ok(Self { socket, addr })
    }

    pub async fn broadcast(&self, heartbeat: &NodeHeartbeat) -> Result<()> {
        let mut buf = Vec::with_capacity(256);
        heartbeat.encode(&mut buf)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        self.socket.send_to(&buf, self.addr).await?;
        Ok(())
    }
}

/// Listens for heartbeats from other nodes and updates a shared registry.
pub struct HeartbeatListener {
    multicast_addr: String,
    registry: Arc<Mutex<NodeRegistry>>,
}

impl HeartbeatListener {
    pub fn new(multicast_addr: &str, registry: Arc<Mutex<NodeRegistry>>) -> Self {
        Self {
            multicast_addr: multicast_addr.to_string(),
            registry,
        }
    }

    pub async fn listen(self) -> Result<()> {
        let addr: SocketAddr = self.multicast_addr.parse()?;
        let ipv4 = match addr {
            SocketAddr::V4(v4) => v4.ip().clone(),
            _ => return Err(crate::error::RegistryError::Io(
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "multicast must be IPv4")
            )),
        };

        let socket = UdpSocket::bind(addr).await?;
        socket.join_multicast_v4(ipv4, Ipv4Addr::UNSPECIFIED)?;

        let mut buf = vec![0u8; 4096];
        loop {
            let (len, _) = socket.recv_from(&mut buf).await?;
            if let Ok(hb) = NodeHeartbeat::decode(&buf[..len]) {
                let mut reg = self.registry.lock().unwrap();
                reg.update_from_heartbeat(&hb);
            }
        }
    }
}