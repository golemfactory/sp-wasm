use super::vfsops::*;
use crate::vfsdo::*;
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{fs, io};

struct DirFs {
    root: Arc<DirFsInode>,
}

struct DirFsInode {
    parent: Option<Arc<DirFsInode>>,
    lookup_path: PathBuf,
    m: fs::Metadata,
}

impl VfsVolume for DirFs {
    type INode = Arc<DirFsInode>;

    fn root(&self) -> io::Result<Arc<DirFsInode>> {
        Ok(self.root.clone())
    }
}

impl INode for Arc<DirFsInode> {
    type Stream = File;

    fn lookup(&self, name: &str) -> io::Result<Option<Self>> {
        let node = match name {
            "." => Some(self.clone()),
            ".." => Some(
                self.parent
                    .as_ref()
                    .map(Clone::clone)
                    .unwrap_or_else(|| self.clone()),
            ),
            _ => {
                let lookup_path = self.lookup_path.join(name);
                if lookup_path.exists() {
                    let m = lookup_path.metadata()?;

                    Some(Arc::new(DirFsInode {
                        lookup_path,
                        m,
                        parent: Some(self.clone()),
                    }))
                } else {
                    None
                }
            }
        };
        Ok(node)
    }

    fn mode(&self) -> (NodeType, NodeMode) {
        let p = self.m.permissions();
        let mode = if p.readonly() {
            NodeMode::Ro
        } else {
            NodeMode::Rw
        };
        let node_type = if self.m.is_dir() {
            NodeType::Dir
        } else {
            NodeType::File
        };

        (node_type, mode)
    }

    fn open(&self, name: &str, mode: NodeMode, create_new: bool) -> io::Result<Self::Stream> {
        let mut opts = OpenOptions::new();
        match mode {
            NodeMode::Ro => opts.read(true),
            NodeMode::Rw => opts.read(true).write(true),
            NodeMode::Wo => opts.write(true),
        }
        .create(create_new)
        .open(&self.lookup_path.join(name))
    }

    fn mkdir(&mut self, name: &str) -> io::Result<Self> {
        let lookup_path = self.lookup_path.join(name);
        fs::create_dir(&lookup_path)?;
        let m = lookup_path.metadata()?;
        Ok(Arc::new(DirFsInode {
            lookup_path,
            m,
            parent: Some(self.clone()),
        }))
    }

    fn read_dir(&self) -> io::Result<Vec<String>> {
        let mut out = Vec::new();
        for ent in fs::read_dir(&self.lookup_path)? {
            let ent = ent?;
            if let Some(file_name) = ent.file_name().to_str() {
                out.push(file_name.into())
            }
        }
        Ok(out)
    }
}

impl Stream for File {
    fn read(&mut self, buf: &mut [u8], position: u64) -> io::Result<u64> {
        let pos = io::Seek::seek(self, io::SeekFrom::Start(position))?;
        let len = io::Read::read(self, buf)?;
        Ok(len as u64)
    }

    fn write(&mut self, buf: &[u8], position: u64) -> io::Result<u64> {
        let pos = io::Seek::seek(self, io::SeekFrom::Start(position))?;
        let len = io::Write::write(self, buf)?;
        Ok(len as u64)
    }

    fn close(self: Box<Self>) -> io::Result<()> {
        Ok(())
    }
}

pub fn volume(base_path: impl Into<PathBuf>) -> io::Result<impl VfsVolume + 'static> {
    let lookup_path = base_path.into();
    let m = lookup_path.metadata()?;
    let parent = None;

    Ok(DirFs {
        root: Arc::new(DirFsInode {
            lookup_path,
            m,
            parent,
        }),
    })
}
