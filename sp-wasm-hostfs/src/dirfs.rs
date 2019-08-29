use std::{io, path, fs};
use std::path::PathBuf;
use super::vfsops::*;
use crate::vfsdo::*;
use itertools::Itertools;
use std::fs::{OpenOptions, File};

struct DirFs {
    base : PathBuf
}

struct DirFsInode {
    lookup_path : PathBuf,
    m : fs::Metadata
}

impl VfsVolume for DirFs {
    type INode = DirFsInode;

    fn lookup(&self, path: &str) -> io::Result<Option<Self::INode>> {
        let lookup_path = self.base.join(path);
        Ok(if lookup_path.exists() {
            let m = lookup_path.metadata()?;

            Some(DirFsInode {
                lookup_path, m
            })
        }
        else {
            None
        })
    }
}

impl INode for DirFsInode {
    type Stream = File;

    fn mode(&self) -> (NodeType, NodeMode) {
        let p = self.m.permissions();
        let mode = if p.readonly() {
            NodeMode::Ro
        }
        else {
            NodeMode::Rw
        };
        let node_type = if self.m.is_dir() {
            NodeType::Dir
        }
        else {
            NodeType::File
        };

        (node_type, mode)
    }

    fn open(&self, mode: NodeMode, create_new :bool) -> io::Result<Self::Stream> {
        let mut opts = OpenOptions::new();
        match mode {
            NodeMode::Ro => opts.read(true),
            NodeMode::Rw => opts.read(true).write(true),
            NodeMode::Wo => opts.write(true)
        }.create(create_new).open(&self.lookup_path)
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
        let pos = io::Seek::seek(self, io::SeekFrom::Current(position as i64))?;
        let len = io::Read::read(self, buf)?;
        Ok(len as u64)
    }

    fn write(&mut self, buf: &[u8], position: u64) -> io::Result<u64> {
        let pos = io::Seek::seek(self, io::SeekFrom::Current(position as i64))?;
        let len = io::Write::write(self, buf)?;
        Ok(len as u64)
    }

    fn close(self : Box<Self>) -> io::Result<()> {
        Ok(())
    }
}


pub fn volume(base_path : impl Into<PathBuf>) -> impl VfsVolume {
    DirFs {
       base:  base_path.into()
    }
}