use super::vfsdo::{NodeMode, NodeType};
use std::io;

pub trait VfsVolume {
    type INode: INode + Send + Sync;

    fn root(&self) -> io::Result<Self::INode>;
}

pub trait INode: Sized + Send + Sync {
    type Stream: Stream + Send + Sync;

    fn mode(&self) -> (NodeType, NodeMode);

    fn open(&self, name: &str, mode: NodeMode, create_new: bool) -> io::Result<Self::Stream>;

    fn mkdir(&mut self, name : &str) -> io::Result<Self>;

    fn lookup(&self, name: &str) -> io::Result<Option<Self>>;

    fn read_dir(&self) -> io::Result<Vec<String>>;
}

pub trait Stream {
    fn read(&mut self, buf: &mut [u8], position: u64) -> io::Result<u64>;

    fn write(&mut self, buf: &[u8], position: u64) -> io::Result<u64>;

    fn close(self: Box<Self>) -> io::Result<()>;
}
