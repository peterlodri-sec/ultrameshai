use std::path::Path;
use tokio::net::{UnixListener, UnixStream};
use prost::Message;
use crate::framed::{write_message, read_message};
use crate::error::Result;

/// A typed connection over UDS that can read/write protobuf messages.
pub struct UdsConnection {
    stream: UnixStream,
}

impl UdsConnection {
    pub async fn write<M: Message>(&mut self, msg: &M) -> Result<()> {
        write_message(&mut self.stream, msg).await
    }

    pub async fn read<M: Message + Default>(&mut self) -> Result<M> {
        read_message(&mut self.stream).await
    }
}

/// UDS server that accepts connections and runs a handler per connection.
pub struct UdsServer {
    listener: UnixListener,
}

impl UdsServer {
    pub async fn bind(path: impl AsRef<Path>) -> Result<Self> {
        // Remove stale socket
        let path = path.as_ref();
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        let listener = UnixListener::bind(path)?;
        Ok(Self { listener })
    }

    /// Accept connections and run handler. Handler is a closure that
    /// takes a UdsConnection and returns a future.
    pub async fn accept<F, Fut>(self, handler: F) -> Result<()>
    where
        F: Fn(UdsConnection) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send,
    {
        let handler = std::sync::Arc::new(handler);
        loop {
            let (stream, _) = self.listener.accept().await?;
            let handler = handler.clone();
            let conn = UdsConnection { stream };
            tokio::spawn(async move {
                handler(conn).await;
            });
        }
    }
}

/// UDS client that connects to a server and exchanges protobuf messages.
pub struct UdsClient {
    conn: UdsConnection,
}

impl UdsClient {
    pub async fn connect(path: impl AsRef<Path>) -> Result<Self> {
        let stream = UnixStream::connect(path).await?;
        Ok(Self {
            conn: UdsConnection { stream },
        })
    }

    pub async fn write<M: Message>(&mut self, msg: &M) -> Result<()> {
        self.conn.write(msg).await
    }

    pub async fn read<M: Message + Default>(&mut self) -> Result<M> {
        self.conn.read().await
    }
}