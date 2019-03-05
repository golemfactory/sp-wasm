use std::collections::VecDeque;
use std::error::Error as StdError;
use std::fs;
use std::io::{self, Read};
use std::path;

pub fn map_path<P: AsRef<path::Path>>(
    repo: &mut zbox::Repo,
    path: P,
) -> Result<(), Box<dyn StdError>> {
    let mut rel_path = path::PathBuf::from("/");
    rel_path.push(path.as_ref().file_name().ok_or(error::RelativePathError)?);
    let abs_path = path::PathBuf::from(path.as_ref());

    let mut fifo = VecDeque::new();
    fifo.push_back((abs_path, rel_path));

    while let Some(path) = fifo.pop_front() {
        let (abs_path, rel_path) = path;

        log::debug!("abs_path = {:?}, rel_path = {:?}", abs_path, rel_path);

        if abs_path.is_dir() {
            repo.create_dir(&rel_path)?;

            log::debug!("created dir = {:?}", rel_path);

            for entry in fs::read_dir(abs_path)? {
                let entry = entry?;
                let abs_path = entry.path();

                let mut rel_path = rel_path.clone();
                rel_path.push(abs_path.file_name().ok_or(error::RelativePathError)?);

                fifo.push_back((abs_path, rel_path));
            }
        } else {
            let mut file = repo.create_file(&rel_path)?;
            let contents = read_file(&abs_path)?;
            file.write_once(&contents)?;

            log::debug!("copied file {:?} => {:?}", abs_path, rel_path);
        }
    }
    Ok(())
}

pub fn read_file<P: AsRef<path::Path>>(path: P) -> io::Result<Vec<u8>> {
    let mut file = fs::File::open(path)?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;
    Ok(contents)
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
}
