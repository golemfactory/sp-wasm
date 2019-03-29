use std::collections::VecDeque;
use std::fs;
use std::io::{Read, Write};
use std::path;

use sp_wasm_memfs::prelude::*;

use super::Result;
use crate::error::Error;

pub struct VirtualFS {
    backend: MemFS,
}

impl VirtualFS {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read_file<P>(&mut self, path: P) -> Result<Vec<u8>>
    where
        P: AsRef<path::Path>,
    {
        let mut file = self.backend.open_file(path.as_ref())?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;

        Ok(contents)
    }

    pub fn write_file<P>(&mut self, path: P, contents: &[u8]) -> Result<()>
    where
        P: AsRef<path::Path>,
    {
        log::debug!("Writing file={:?}", path.as_ref());
        let mut file = self.backend.create_file(path.as_ref())?;
        file.write_all(contents)?;

        Ok(())
    }

    pub fn create_dir_all<P>(&mut self, path: P) -> Result<()>
    where
        P: AsRef<path::Path>,
    {
        log::debug!("Creating subdirs={:?}", path.as_ref());
        self.backend.create_dir_all(path)?;

        Ok(())
    }

    pub fn map_file<P>(&mut self, source_path: P, dest_path: P) -> Result<()>
    where
        P: AsRef<path::Path>,
    {
        let contents = hostfs::read_file(source_path.as_ref())?;
        self.write_file(dest_path.as_ref(), &contents)?;

        Ok(())
    }

    pub fn map_path<P1, P2>(
        &mut self,
        source_path: P1,
        dest_path: P2,
        cb: &mut FnMut(&path::Path, &path::Path),
    ) -> Result<()>
    where
        P1: AsRef<path::Path>,
        P2: AsRef<path::Path>,
    {
        let dest_path = dest_path.as_ref().to_owned();
        let source_path = source_path.as_ref().to_owned();
        let mut fifo = VecDeque::new();

        for entry in fs::read_dir(source_path)? {
            let entry = entry?;
            let source_path = entry.path();

            let mut dest_path = dest_path.clone();
            dest_path.push(source_path.file_name().ok_or(Error::InvalidPath)?);

            fifo.push_back((source_path, dest_path));
        }

        while let Some((source_path, dest_path)) = fifo.pop_front() {
            log::debug!(
                "source_path = {:?}, dest_path = {:?}",
                source_path,
                dest_path
            );
            cb(&source_path, &dest_path);

            if source_path.is_dir() {
                self.backend.create_dir(dest_path.as_path())?;
                log::debug!("mapped dir = {:?}", dest_path);

                for entry in fs::read_dir(source_path)? {
                    let entry = entry?;
                    let source_path = entry.path();

                    let mut dest_path = dest_path.clone();
                    dest_path.push(source_path.file_name().ok_or(Error::InvalidPath)?);

                    fifo.push_back((source_path, dest_path));
                }
            } else {
                self.map_file(source_path.as_path(), dest_path.as_path())?;
                log::debug!("mapped file {:?} => {:?}", source_path, dest_path);
            }
        }

        Ok(())
    }
}

impl Default for VirtualFS {
    fn default() -> Self {
        Self {
            backend: MemFS::default(),
        }
    }
}

pub mod hostfs {
    use super::Result;

    use std::fs;
    use std::io::{Read, Write};
    use std::path;

    use path_clean::PathClean;

    pub fn read_file<P>(path: P) -> Result<Vec<u8>>
    where
        P: AsRef<path::Path>,
    {
        let mut file = fs::File::open(path)?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;

        Ok(contents)
    }

    pub fn write_file<P>(path: P, contents: &[u8]) -> Result<()>
    where
        P: AsRef<path::Path>,
    {
        let mut file = fs::File::create(path.as_ref())?;
        file.write_all(contents)?;

        Ok(())
    }

    pub fn create_dir_all<P>(path: P) -> Result<()>
    where
        P: AsRef<path::Path>,
    {
        fs::create_dir_all(path)?;

        Ok(())
    }

    pub fn sanitize_path<P>(path: P) -> Result<path::PathBuf>
    where
        P: AsRef<path::Path>,
    {
        let mut sanitized = path::PathBuf::from("/");
        sanitized.push(path.as_ref());
        let sanitized = sanitized.clean();
        let path = sanitized.strip_prefix("/").map(path::PathBuf::from)?;

        Ok(path)
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    #[test]
    fn sanitize_path() {
        assert_eq!(
            Ok(PathBuf::from("out.txt")),
            super::hostfs::sanitize_path("../../../out.txt")
        );

        assert_eq!(
            Ok(PathBuf::from("out.txt")),
            super::hostfs::sanitize_path("out/../../../out.txt")
        );

        assert_eq!(
            Ok(PathBuf::from("out/out.txt")),
            super::hostfs::sanitize_path("out/../out/../out/../out/out.txt")
        );
    }
}
