use std::io;
use mozjs::rooted;
use mozjs::conversions::ToJSValConvertible;
use mozjs::rust::{MutableHandleValue};
use mozjs::rust::wrappers as jsw;
use mozjs::jsapi as js;
use mozjs::jsval;
use crate::vfsdo::{VolumeInfo, NodeInfo, NodeMode};
use crate::vfsops::{INode, Stream, VfsVolume};

pub mod vfsops;
pub mod vfsdo;

type Fd = Box<dyn vfsops::Stream + 'static>;
type ResolverDyn = Box<dyn VfsResolver + 'static>;

trait VfsResolver {

    fn info(&self, id : u32) -> VolumeInfo;

    fn lookup(&self, path : &str) -> io::Result<Option<NodeInfo>>;

    fn open(&self, path : &str, mode : NodeMode) -> io::Result<Box<dyn vfsops::Stream>>;

    fn readdir(&self, path : &str) -> io::Result<Vec<String>>;
}

pub struct VfsManager {
    fds : [Option<Fd>; 64],
    volumes : Vec<ResolverDyn>
}

struct Resolver<T : vfsops::VfsVolume> {
    volume : T,
    mount_point : String,
    mode : vfsdo::NodeMode,
}

impl<T : vfsops::VfsVolume + 'static> VfsResolver for Resolver<T> {
    fn info(&self, id: u32) -> VolumeInfo {
        VolumeInfo {
            id,
            mount_point: self.mount_point.clone(),
            mode: self.mode
        }
    }

    fn lookup(&self, path: &str) -> io::Result<Option<NodeInfo>> {
        let inode = self.volume.lookup(path)?;

        Ok(inode.as_ref().map( vfsops::INode::mode).map(|(n_type, n_mode)| NodeInfo {
            n_type, n_mode
        }))
    }

    fn open(&self, path: &str, mode : NodeMode) -> io::Result<Box<vfsops::Stream>> {
        self.volume.lookup(path)?
            .ok_or_else(|| io::ErrorKind::NotFound.into())
            .and_then(|ino| ino.open(mode))
            .map(|s| -> Box<dyn vfsops::Stream + 'static> {
                Box::new(s)
            })
    }

    fn readdir(&self, path : &str) -> io::Result<Vec<String>> {
        self.volume.lookup(path)?
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?
            .read_dir()
    }

}


impl VfsManager {

    pub fn new() -> Self {
        VfsManager {
            fds: [
                None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None,

                None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None,
            ],
            volumes: Vec::new()
        }
    }

    pub fn volumes(&self) -> Vec<VolumeInfo> {
        self.volumes.iter()
            .enumerate()
            .map(|(idx, v)| v.info(idx as u32))
            .collect()
    }

    pub fn lookup(&self, vol_id : usize, path : &str) -> io::Result<Option<NodeInfo>> {
        self.volumes
            .get(vol_id)
            .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?
            .lookup(path)
    }

    pub fn open(&mut self, vol_id : usize, path : &str, mode : NodeMode) -> io::Result<u32> {
        let (idx, f) = self.fds.iter_mut().enumerate()
            .find(|(_, fd)| fd.is_none())
            .ok_or_else(|| io::Error::from_raw_os_error(24 /*Too many open files*/))?;

        let stream = self.volumes
            .get(vol_id)
            .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?
            .open(path, mode)?;

        *f = Some(stream);
        Ok(idx as u32)
    }

    pub fn close(&mut self, fd : u32) -> io::Result<()> {
        if let Some(fd) = self.fds.get_mut(fd as usize).ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?.take() {
            fd.close()
        }
        else {
            Err(io::Error::from(io::ErrorKind::InvalidInput))
        }
    }
    // read(fd, buf, offset, len, position) -> int
    pub fn read(&mut self, fd : u32, buf : &mut [u8], position : u64) -> io::Result<u64> {
        let v = self.fds.get_mut(fd as usize)
            .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
        match v {
            Some(f) => f.read(buf, position),
            None => Err(io::Error::from(io::ErrorKind::InvalidInput))
        }
    }

    pub fn write(&mut self, fd : u32, buf : &[u8], position : u64) -> io::Result<u64> {
        let v = self.fds.get_mut(fd as usize)
            .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
        match v {
            Some(f) => f.write(buf, position),
            None => Err(io::Error::from(io::ErrorKind::InvalidInput))
        }
    }

    pub fn bind(&mut self, path : impl Into<String>, mode : NodeMode, v : impl VfsVolume + 'static) -> io::Result<()> {
        let resolver = Box::new(Resolver {
            volume: v,
            mount_point: path.into(),
            mode
        });
        self.volumes.push(resolver);
        Ok(())
    }

}