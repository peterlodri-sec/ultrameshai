use std::fs::OpenOptions;
use std::path::Path;
use memmap2::MmapMut;
use prost::Message;
use crate::error::Result;

/// Writes a Protobuf message to a memory-mapped file.
/// Returns the encoded length of the message in bytes.
pub fn write_to_mmap<M: Message>(path: &Path, msg: &M) -> Result<usize> {
    let len = msg.encoded_len();
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    file.set_len(len as u64)?;
    
    if len > 0 {
        let mut mmap = unsafe { MmapMut::map_mut(&file)? };
        let mut buf = &mut mmap[..];
        msg.encode(&mut buf)?;
        mmap.flush()?;
    }
    Ok(len)
}

/// Reads a Protobuf message from a memory-mapped file.
pub fn read_from_mmap<M: Message + Default>(path: &Path, len: usize) -> Result<M> {
    if len == 0 {
        return Ok(M::default());
    }
    let file = OpenOptions::new().read(true).open(path)?;
    let mmap = unsafe { memmap2::Mmap::map(&file)? };
    let msg = M::decode(&mmap[..len])?;
    Ok(msg)
}
