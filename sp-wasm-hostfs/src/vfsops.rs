use std::io;
use super::vfsdo::{NodeMode, NodeType};


pub trait VfsVolume {
    type INode : INode;

    fn lookup(&self, path : &str) -> io::Result<Option<Self::INode>>;

}

pub trait INode {
    type Stream : Stream;

    fn mode(&self) -> (NodeType, NodeMode);

    fn open(&self, mode : NodeMode) -> io::Result<Self::Stream>;

    fn read_dir(&self) -> io::Result<Vec<String>>;

}

pub trait Stream {

    fn read(&mut self, buf : &mut [u8], position : u64) -> io::Result<u64>;

    fn write(&mut self, buf : &[u8], position : u64) -> io::Result<u64>;

    fn close(self : Box<Self>) -> io::Result<()>;
}