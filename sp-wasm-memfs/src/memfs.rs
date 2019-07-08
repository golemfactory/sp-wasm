use super::error::*;
use super::file::*;
use super::node::*;
use super::Result;
use path_clean::PathClean;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tool::prelude::*;

#[derive(Debug)]
pub struct MemFS {
    root: Arc<Mutex<Node>>,
}

impl MemFS {
    pub fn new() -> Self {
        Self::default()
    }

    fn walk<P>(&self, path: P, node: Arc<Mutex<Node>>) -> Result<Arc<Mutex<Node>>>
    where
        P: AsRef<Path>,
    {
        let mut components = path.as_ref().components();
        components.next(); // skip first
        let path = components.as_path();

        let name: String = match components.next() {
            Some(component) => component
                .as_os_str()
                .to_str()
                .ok_or(Error::InvalidPath(
                    component.as_os_str().to_string_lossy().to_string(),
                ))?
                .to_owned(),
            None => return Ok(node),
        };

        if node.lock().unwrap().is_file() {
            if node.lock().unwrap().name == name {
                return Ok(Arc::clone(&node));
            }
        } else {
            if let Some(next) = node.lock().unwrap().children.get(&name) {
                return self.walk(path, Arc::clone(&next));
            }
        }

        Err(Error::NotFound(name))
    }

    fn normalize_path<P>(path: P) -> Result<PathBuf>
    where
        P: AsRef<Path>,
    {
        if !path.as_ref().has_root() {
            return Err(Error::InvalidPath(
                path.as_ref().to_string_lossy().to_string(),
            ));
        }

        Ok(PathBuf::from(path.as_ref()).clean())
    }

    fn resolve_parent<P>(path: P) -> Result<(PathBuf, String)>
    where
        P: AsRef<Path>,
    {
        let path = Self::normalize_path(path)?;
        let parent = path.parent().ok_or(Error::IsRoot)?;
        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or(Error::InvalidPath(path.to_string_lossy().to_string()))?;

        Ok((parent.to_owned(), filename.to_owned()))
    }

    pub fn create_dir<P>(&self, path: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let (parent, filename) = Self::resolve_parent(path)?;
        let node = self.walk(parent, Arc::clone(&self.root))?;
        node.lock()
            .unwrap()
            .children
            .insert(filename.clone(), new_dir_node(filename));

        Ok(())
    }

    pub fn create_dir_all<P>(&self, path: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let walk_create = fix(|f, path: PathBuf| -> Result<Arc<Mutex<Node>>> {
            let (parent, filename) = Self::resolve_parent(path)?;

            let node = if self.is_dir(parent.as_path())? {
                self.walk(parent, Arc::clone(&self.root))?
            } else {
                f(parent)?
            };

            let new_child = new_dir_node(filename.clone());
            node.lock()
                .unwrap()
                .children
                .insert(filename.clone(), Arc::clone(&new_child));

            Ok(new_child)
        });

        if let Err(err) = walk_create(path.as_ref().to_owned()) {
            match err {
                Error::IsRoot => Ok(()),
                _ => Err(err),
            }
        } else {
            Ok(())
        }
    }

    pub fn create_file<P>(&self, path: P) -> Result<File>
    where
        P: AsRef<Path>,
    {
        let (parent, filename) = Self::resolve_parent(path)?;
        let node = self.walk(parent, Arc::clone(&self.root))?;
        let file_node = new_file_node(filename.clone());
        node.lock()
            .unwrap()
            .children
            .insert(filename, Arc::clone(&file_node));

        Ok(File::new(file_node))
    }

    pub fn open_file<P>(&self, path: P) -> Result<File>
    where
        P: AsRef<Path>,
    {
        let (parent, filename) = Self::resolve_parent(path)?;
        self.walk(parent.join(filename), Arc::clone(&self.root))
            .map(|node| File::new(node))
    }

    pub fn is_dir<P>(&self, path: P) -> Result<bool>
    where
        P: AsRef<Path>,
    {
        let path = Self::normalize_path(path)?;
        match self.walk(path, Arc::clone(&self.root)) {
            Ok(node) => Ok(node.lock().unwrap().is_dir()),
            _ => Ok(false),
        }
    }

    pub fn is_file<P>(&self, path: P) -> Result<bool>
    where
        P: AsRef<Path>,
    {
        let path = Self::normalize_path(path)?;
        match self.walk(path, Arc::clone(&self.root)) {
            Ok(node) => Ok(node.lock().unwrap().is_file()),
            _ => Ok(false),
        }
    }
}

impl Default for MemFS {
    fn default() -> Self {
        Self {
            root: Arc::new(Mutex::new(Node::new("/", FileType::Dir))),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn normalize_path() -> Result<()> {
        assert!(MemFS::normalize_path("tmp/").is_err());
        assert!(MemFS::normalize_path("a.txt").is_err());
        assert!(MemFS::normalize_path(".").is_err());
        assert!(MemFS::normalize_path("").is_err());
        assert!(MemFS::normalize_path(" ").is_err());

        assert_eq!(MemFS::normalize_path("/")?, PathBuf::from("/"));
        assert_eq!(MemFS::normalize_path("//")?, PathBuf::from("/"));
        assert_eq!(MemFS::normalize_path("/../")?, PathBuf::from("/"));
        assert_eq!(MemFS::normalize_path("/./")?, PathBuf::from("/"));
        assert_eq!(MemFS::normalize_path("/.././")?, PathBuf::from("/"));
        assert_eq!(MemFS::normalize_path("/tmp/../")?, PathBuf::from("/"));
        assert_eq!(MemFS::normalize_path("/tmp/a/../")?, PathBuf::from("/tmp"));

        Ok(())
    }

    #[test]
    fn resolve_parent() -> Result<()> {
        assert_eq!(MemFS::resolve_parent("/").unwrap_err(), Error::IsRoot);
        assert_eq!(
            MemFS::resolve_parent("tmp").unwrap_err(),
            Error::InvalidPath("tmp".to_owned())
        );

        assert_eq!(
            MemFS::resolve_parent("/tmp")?,
            (PathBuf::from("/"), "tmp".to_owned())
        );
        assert_eq!(
            MemFS::resolve_parent("/tmp/a/b/c")?,
            (PathBuf::from("/tmp/a/b"), "c".to_owned())
        );

        Ok(())
    }

    #[test]
    fn create_dir() -> Result<()> {
        let fs = MemFS::new();
        fs.create_dir("/tmp")?;
        fs.create_dir("/tmp/a")?;
        fs.create_dir("/tmp/a/b")?;
        fs.create_dir("/dev")?;

        assert!(fs.is_dir("/tmp")?);
        assert!(fs.is_dir("/tmp/a")?);
        assert!(fs.is_dir("/tmp/a/b")?);
        assert!(fs.is_dir("/dev")?);

        assert_eq!(
            fs.create_dir("/tmp/c/d").unwrap_err(),
            Error::NotFound("c".to_owned())
        );
        assert_eq!(fs.create_dir("/").unwrap_err(), Error::IsRoot);
        assert_eq!(
            fs.create_dir("tmp/a/c").unwrap_err(),
            Error::InvalidPath("tmp/a/c".to_owned())
        );

        Ok(())
    }

    #[test]
    fn create_dir_all() -> Result<()> {
        let fs = MemFS::new();
        fs.create_dir_all("/tmp/a/b/c")?;

        assert!(fs.is_dir("/tmp")?);
        assert!(fs.is_dir("/tmp/a")?);
        assert!(fs.is_dir("/tmp/a/b")?);
        assert!(fs.is_dir("/tmp/a/b/c")?);

        assert!(fs.create_dir_all("/").is_ok());

        Ok(())
    }

    #[test]
    fn create_file() -> Result<()> {
        let fs = MemFS::new();
        fs.create_dir("/tmp")?;
        fs.create_file("/tmp/a")?;
        fs.create_file("/b")?;
        fs.create_dir("/dev")?;
        fs.create_file("/dev/random")?;

        assert!(fs.is_file("/tmp/a")?);
        assert!(fs.is_file("/b")?);
        assert!(fs.is_file("/dev/random")?);

        assert_eq!(fs.create_file("/").unwrap_err(), Error::IsRoot);
        assert_eq!(
            fs.create_file("c").unwrap_err(),
            Error::InvalidPath("c".to_owned())
        );
        assert_eq!(
            fs.create_file("/d/c").unwrap_err(),
            Error::NotFound("d".to_owned())
        );

        Ok(())
    }

    #[test]
    fn open_file() -> Result<()> {
        let fs = MemFS::new();
        fs.create_file("/test")?;

        assert!(fs.is_file("/test")?);
        assert!(fs.open_file("/test").is_ok());

        assert_eq!(fs.open_file("/").unwrap_err(), Error::IsRoot);
        assert_eq!(
            fs.open_file("/bbb").unwrap_err(),
            Error::NotFound("bbb".to_owned())
        );
        assert_eq!(
            fs.open_file("bbb").unwrap_err(),
            Error::InvalidPath("bbb".to_owned())
        );

        Ok(())
    }
}
