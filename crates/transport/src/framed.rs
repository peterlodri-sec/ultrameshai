use bytes::BytesMut;
use prost::Message;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use crate::error::{TransportError, Result};

/// Maximum message size: 4MB (enough for large stats blobs)
const MAX_MESSAGE_SIZE: usize = 4 * 1024 * 1024;

/// Write a length-delimited protobuf message.
/// Format: [4-byte big-endian length][protobuf bytes]
pub async fn write_message<W, M>(writer: &mut W, msg: &M) -> Result<()>
where
    W: AsyncWrite + Unpin,
    M: Message,
{
    let mut buf = BytesMut::new();
    msg.encode(&mut buf)?;
    let len = buf.len() as u32;
    writer.write_all(&len.to_be_bytes()).await?;
    writer.write_all(&buf).await?;
    Ok(())
}

/// Read a length-delimited protobuf message.
pub async fn read_message<R, M>(reader: &mut R) -> Result<M>
where
    R: AsyncRead + Unpin,
    M: Message + Default,
{
    let mut len_buf = [0u8; 4];
    let n = reader.read(&mut len_buf).await?;
    if n == 0 {
        return Err(TransportError::ConnectionClosed);
    }
    if n < 4 {
        reader.read_exact(&mut len_buf[n..]).await?;
    }
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > MAX_MESSAGE_SIZE {
        return Err(TransportError::MessageTooLarge { size: len, max: MAX_MESSAGE_SIZE });
    }
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).await?;
    let msg = M::decode(&buf[..])?;
    Ok(msg)
}