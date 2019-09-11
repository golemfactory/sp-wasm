use crate::vfsdo::*;
use crate::vfsops::*;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::io::{Cursor, SeekFrom};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::{fs, io, path};
use zip::ZipArchive;

struct ZipFs {
    root: Arc<ZipFsNode>,
}

impl ZipFs {
    fn new(path: impl Into<path::PathBuf> + 'static) -> io::Result<Self> {
        let path = path.into();
        let file = OpenOptions::new().read(true).open(&path)?;
        let mut archive = ZipArchive::new(file)?;
        let mut files = HashMap::new();
        let mut paths = Vec::new();
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let path: Rc<Path> = file.sanitized_name().into();
            paths.push(path.clone());
            if file.is_file() {
                files.insert(
                    path,
                    ZipFsNodeRef::File {
                        idx: i,
                        size: file.size(),
                    },
                );
            } else if file.is_dir() {
                files.insert(
                    path,
                    ZipFsNodeRef::Dir {
                        idx: i,
                        nodes: HashMap::new(),
                    },
                );
            }
        }
        let root: Rc<Path> = PathBuf::from("").into();
        files.insert(
            root.clone(),
            ZipFsNodeRef::Root {
                nodes: HashMap::new(),
            },
        );
        paths.sort();
        for path in paths.into_iter().rev() {
            let node = files.remove(&path).unwrap();
            let parent_path = path.parent().unwrap_or_else(|| root.as_ref());
            let node = Arc::new(node);
            match files.get_mut(parent_path) {
                Some(ZipFsNodeRef::Dir { nodes, .. }) => {
                    nodes.insert(path.file_name().unwrap().to_str().unwrap().into(), node)
                }
                Some(ZipFsNodeRef::Root { nodes }) => {
                    nodes.insert(path.file_name().unwrap().to_str().unwrap().into(), node)
                }
                _ => panic!("todo"),
            };
        }

        let inner = Arc::new(Mutex::new(ZipFsInner { path, archive }));

        let node_ref = Arc::new(files.remove(root.as_ref()).unwrap());

        let root = Arc::new(ZipFsNode { inner, node_ref });

        Ok(ZipFs { root })
    }
}

impl VfsVolume for ZipFs {
    type INode = ZipFsNode;

    fn root(&self) -> io::Result<Self::INode> {
        let inner = self.root.inner.clone();
        let node_ref = self.root.node_ref.clone();
        Ok(ZipFsNode { inner, node_ref })
    }
}

struct ZipFsInner {
    path: path::PathBuf,
    archive: ZipArchive<fs::File>,
}

impl ZipFsInner {
    fn load_file(&mut self, file_number: usize) -> io::Result<Vec<u8>> {
        let mut file = self.archive.by_index(file_number)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        Ok(buf)
    }
}

enum ZipFsNodeRef {
    File {
        idx: usize,
        size: u64,
    },
    Dir {
        idx: usize,
        nodes: HashMap<String, Arc<ZipFsNodeRef>>,
    },
    Root {
        nodes: HashMap<String, Arc<ZipFsNodeRef>>,
    },
}

struct ZipFsNode {
    inner: Arc<Mutex<ZipFsInner>>,
    node_ref: Arc<ZipFsNodeRef>,
}

struct ZipFsStream(io::Cursor<Vec<u8>>);

impl ZipFsNode {
    fn find_node(&self, name: &str) -> io::Result<Option<&Arc<ZipFsNodeRef>>> {
        let nodes = match self.node_ref.as_ref() {
            ZipFsNodeRef::Dir { nodes, .. } => nodes,
            ZipFsNodeRef::Root { nodes } => nodes,
            _ => return Err(io::ErrorKind::PermissionDenied.into()),
        };
        Ok(nodes.get(name))
    }
}

impl INode for ZipFsNode {
    type Stream = ZipFsStream;

    fn mode(&self) -> (NodeType, NodeMode, u64) {
        match self.node_ref.as_ref() {
            ZipFsNodeRef::File { size, .. } => (NodeType::File, NodeMode::Ro, *size),
            _ => (NodeType::Dir, NodeMode::Ro, 0),
        }
    }

    fn open(&self, name: &str, mode: NodeMode, create_new: bool) -> io::Result<Self::Stream> {
        //eprintln!("try open: {}", name);
        match mode {
            NodeMode::Ro => (),
            _ => return Err(io::ErrorKind::PermissionDenied.into()),
        };

        let file_index = match self.find_node(name)?.map(AsRef::as_ref) {
            Some(ZipFsNodeRef::File { idx, .. }) => *idx,
            _ => return Err(io::ErrorKind::InvalidInput.into()),
        };

        let content = self.inner.lock().unwrap().load_file(file_index)?;
        Ok(ZipFsStream(Cursor::new(content)))
    }

    fn mkdir(&mut self, name: &str) -> io::Result<Self> {
        Err(io::ErrorKind::PermissionDenied.into())
    }

    fn lookup(&self, name: &str) -> io::Result<Option<Self>> {
        if let Some(node) = self.find_node(name)? {
            let node_ref = node.clone();
            let inner = self.inner.clone();
            Ok(Some(ZipFsNode { inner, node_ref }))
        } else {
            Ok(None)
        }
    }

    fn read_dir(&self) -> io::Result<Vec<String>> {
        match self.node_ref.as_ref() {
            ZipFsNodeRef::Dir { nodes, .. } => Ok(nodes.keys().cloned().collect()),
            ZipFsNodeRef::Root { nodes, .. } => Ok(nodes.keys().cloned().collect()),
            _ => Err(io::ErrorKind::PermissionDenied.into()),
        }
    }
}

impl Stream for ZipFsStream {
    fn read(&mut self, buf: &mut [u8], position: u64) -> io::Result<u64> {
        let _ = self.0.seek(SeekFrom::Start(position))?;
        self.0.read(buf).map(|ret| ret as u64)
    }

    fn write(&mut self, buf: &[u8], position: u64) -> io::Result<u64> {
        Err(io::ErrorKind::PermissionDenied.into())
    }

    fn close(self: Box<Self>) -> io::Result<()> {
        Ok(())
    }
}

pub fn volume(zip_path: impl Into<PathBuf>) -> io::Result<impl VfsVolume + 'static> {
    ZipFs::new(zip_path.into())
}

#[cfg(test)]
mod test {
    use super::ZipFs;
    use crate::vfsdo::NodeType;
    use crate::vfsops::{INode, VfsVolume};

    #[test]
    fn test() -> failure::Fallible<()> {
        let vol = ZipFs::new("/home/prekucki/workspace/wasm/miniw.zip").unwrap();
        let mut stack = Vec::new();
        stack.push((String::from(""), vol.root().unwrap()));
        let mut n_files = 0;
        while !stack.is_empty() {
            let (path, inode) = stack.pop().unwrap();
            let (n_type, mode, _) = inode.mode();
            eprintln!(":: {}", path);
            match n_type {
                NodeType::Dir => {
                    for child in inode.read_dir().unwrap() {
                        stack.push((
                            format!("{}/{}", path, child),
                            inode.lookup(&child)?.unwrap(),
                        ))
                    }
                }
                _ => {
                    n_files += 1;
                    ()
                }
            }
        }
        eprintln!("files={}", n_files);
        Ok(())
    }

}
