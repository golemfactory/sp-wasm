use std::collections::{BTreeMap, VecDeque};
use std::error::Error as StdError;
use std::fs;
use std::io::{self, Read, Write};
use std::path;

pub enum FSNode {
    File(Vec<u8>),
    Dir,
}

pub struct VirtualFS {
    pub mapping: BTreeMap<String, FSNode>,
}

impl VirtualFS {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn map_file<P>(&mut self, abs_path: P, rel_path: P) -> io::Result<&FSNode>
    where
        P: AsRef<path::Path>,
    {
        let contents = read_file(&abs_path)?;
        let rel_path: String = rel_path.as_ref().to_string_lossy().into();
        self.mapping
            .insert(rel_path.clone(), FSNode::File(contents));
        Ok(&self.mapping[&rel_path])
    }

    pub fn map_dir<P>(&mut self, path: P) -> io::Result<&FSNode>
    where
        P: AsRef<path::Path>,
    {
        let path: String = path.as_ref().to_string_lossy().into();
        self.mapping.insert(path.clone(), FSNode::Dir);
        Ok(&self.mapping[&path])
    }

    pub fn map_path<P>(
        &mut self,
        path: P,
        cb: &mut FnMut(&path::Path, &FSNode),
    ) -> Result<(), Box<dyn StdError>>
    where
        P: AsRef<path::Path>,
    {
        let mut rel_path = path::PathBuf::from("/");
        rel_path.push(path.as_ref().file_name().ok_or(error::RelativePathError)?);
        let abs_path = path::PathBuf::from(path.as_ref());

        let mut fifo = VecDeque::new();
        fifo.push_back((abs_path, rel_path));

        while let Some(path) = fifo.pop_front() {
            let (abs_path, rel_path) = path;
            log::debug!("abs_path = {:?}, rel_path = {:?}", abs_path, rel_path);

            if abs_path.is_dir() {
                cb(&rel_path, self.map_dir(&rel_path)?);
                log::debug!("mapped dir = {:?}", rel_path);

                for entry in fs::read_dir(abs_path)? {
                    let entry = entry?;
                    let abs_path = entry.path();

                    let mut rel_path = rel_path.clone();
                    rel_path.push(abs_path.file_name().ok_or(error::RelativePathError)?);

                    fifo.push_back((abs_path, rel_path));
                }
            } else {
                cb(&rel_path, self.map_file(&abs_path, &rel_path)?);
                log::debug!("mapped file {:?} => {:?}", abs_path, rel_path);
            }
        }
        Ok(())
    }

    pub fn get_file_contents<S>(&self, path: S) -> Result<&[u8], error::FileNotMappedError>
    where
        S: Into<String>,
    {
        let path = path.into();
        self.mapping
            .get(&path)
            .and_then(|node| match node {
                FSNode::File(ref contents) => Some(contents.as_slice()),
                FSNode::Dir => None,
            })
            .ok_or_else(|| error::FileNotMappedError(path))
    }
}

impl Default for VirtualFS {
    fn default() -> Self {
        Self {
            mapping: BTreeMap::new(),
        }
    }
}

pub fn read_file<P>(path: P) -> io::Result<Vec<u8>>
where
    P: AsRef<path::Path>,
{
    let mut file = fs::File::open(path)?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;
    Ok(contents)
}

pub fn write_file<P>(path: P, contents: &[u8]) -> io::Result<()>
where
    P: AsRef<path::Path>,
{
    let mut file = fs::File::create(path.as_ref())?;
    file.write_all(contents)
}

pub mod error {
    use std::error::Error;
    use std::fmt;

    #[derive(Debug)]
    pub struct RelativePathError;

    impl Error for RelativePathError {}

    impl fmt::Display for RelativePathError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "couldn't extract relative path")
        }
    }

    #[derive(Debug)]
    pub struct FileNotMappedError(pub String);

    impl Error for FileNotMappedError {}

    impl fmt::Display for FileNotMappedError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "file {} was not mapped", self.0)
        }
    }
}
