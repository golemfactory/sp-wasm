use std::collections::VecDeque;
use std::error::Error as StdError;
use std::fs;
use std::io::{self, Read, Write};
use std::path;

use path_clean::PathClean;
use zbox::Repo;

pub struct VirtualFS {
    repo: Repo,
}

impl VirtualFS {
    pub fn new(path_to_repo: &str, password: &str) -> Result<Self, Box<dyn StdError>> {
        zbox::init_env();
        let repo = zbox::RepoOpener::new()
            .create(true)
            .open(path_to_repo, password)?;
        Ok(Self { repo })
    }

    pub fn read_file<P>(&mut self, path: P) -> Result<Vec<u8>, Box<dyn StdError>>
    where
        P: AsRef<path::Path>,
    {
        let mut file = self.repo.open_file(path.as_ref())?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;
        Ok(contents)
    }

    pub fn write_file<P>(&mut self, path: P, contents: &[u8]) -> Result<(), zbox::Error>
    where
        P: AsRef<path::Path>,
    {
        let mut file = self.repo.create_file(path.as_ref())?;
        file.write_once(contents)
    }

    pub fn map_file<P>(&mut self, abs_path: P, rel_path: P) -> Result<(), Box<dyn StdError>>
    where
        P: AsRef<path::Path>,
    {
        let contents = read_file(abs_path.as_ref())?;
        self.write_file(rel_path.as_ref(), &contents)?;
        Ok(())
    }

    pub fn map_path<P>(
        &mut self,
        path: P,
        relative_root: P,
        cb: &mut FnMut(&path::Path, &path::Path),
    ) -> Result<(), Box<dyn StdError>>
    where
        P: AsRef<path::Path>,
    {
        let rel_path = relative_root.as_ref().to_owned();
        let abs_path = path.as_ref().to_owned();
        let mut fifo = VecDeque::new();

        for entry in fs::read_dir(abs_path)? {
            let entry = entry?;
            let abs_path = entry.path();

            let mut rel_path = rel_path.clone();
            rel_path.push(abs_path.file_name().ok_or(error::RelativePathError)?);

            fifo.push_back((abs_path, rel_path));
        }

        while let Some((abs_path, rel_path)) = fifo.pop_front() {
            log::debug!("abs_path = {:?}, rel_path = {:?}", abs_path, rel_path);
            cb(&abs_path, &rel_path);

            if abs_path.is_dir() {
                self.repo.create_dir(rel_path.as_path())?;
                log::debug!("mapped dir = {:?}", rel_path);

                for entry in fs::read_dir(abs_path)? {
                    let entry = entry?;
                    let abs_path = entry.path();

                    let mut rel_path = rel_path.clone();
                    rel_path.push(abs_path.file_name().ok_or(error::RelativePathError)?);

                    fifo.push_back((abs_path, rel_path));
                }
            } else {
                self.map_file(abs_path.as_path(), rel_path.as_path())?;
                log::debug!("mapped file {:?} => {:?}", abs_path, rel_path);
            }
        }
        Ok(())
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

pub fn sanitize_path<P>(path: P) -> Result<path::PathBuf, path::StripPrefixError>
where
    P: AsRef<path::Path>,
{
    let mut sanitized = path::PathBuf::from("/");
    sanitized.push(path.as_ref());
    let sanitized = sanitized.clean();
    sanitized.strip_prefix("/").map(path::PathBuf::from)
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    #[test]
    fn sanitize_path() {
        assert_eq!(
            Ok(PathBuf::from("out.txt")),
            super::sanitize_path("../../../out.txt")
        );

        assert_eq!(
            Ok(PathBuf::from("out.txt")),
            super::sanitize_path("out/../../../out.txt")
        );

        assert_eq!(
            Ok(PathBuf::from("out/out.txt")),
            super::sanitize_path("out/../out/../out/../out/out.txt")
        );
    }
}
