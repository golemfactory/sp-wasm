use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, PartialEq)]
pub(crate) enum FileType {
    Dir,
    File,
}

#[derive(Debug)]
pub(crate) struct Node {
    pub name: String,
    pub file_type: FileType,
    pub children: BTreeMap<String, Arc<Mutex<Node>>>,
    pub contents: Vec<u8>,
}

impl Node {
    pub fn new<S>(name: S, file_type: FileType) -> Self
    where
        S: Into<String>,
    {
        Self {
            name: name.into(),
            file_type,
            children: BTreeMap::new(),
            contents: Vec::new(),
        }
    }

    pub fn is_file(&self) -> bool {
        self.file_type == FileType::File
    }

    pub fn is_dir(&self) -> bool {
        self.file_type == FileType::Dir
    }
}

pub(crate) fn new_file_node<S>(name: S) -> Arc<Mutex<Node>>
where
    S: Into<String>,
{
    Arc::new(Mutex::new(Node::new(name, FileType::File)))
}

pub(crate) fn new_dir_node<S>(name: S) -> Arc<Mutex<Node>>
where
    S: Into<String>,
{
    Arc::new(Mutex::new(Node::new(name, FileType::Dir)))
}
