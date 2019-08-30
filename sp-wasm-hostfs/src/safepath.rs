use std::io;

pub struct SafePath<'a> {
    path : &'a str
}

impl<'a> From<&'a str> for SafePath<'a> {
    fn from(path: &'a str) -> Self {
        if path.starts_with("/") {
            Self { path: &path[1..] }
        }
        else {
            Self { path }
        }
    }
}

pub struct PathPart<'a> {
    name : &'a str,
    last : bool
}

impl<'a> PathPart<'a> {

    #[inline]
    pub fn is_last(&self) -> bool {
        self.last
    }
}

impl<'a> AsRef<str> for PathPart<'a> {
    fn as_ref(&self) -> &str {
        self.name
    }
}

impl<'a> Iterator for SafePath<'a> {
    type Item = io::Result<PathPart<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        let split_point = self.path.char_indices().find_map(|(idx, ch)|
            match ch {
                ':' | '\\' => Some(Err(io::ErrorKind::InvalidInput.into())),
                _ if ch.is_control() => Some(Err(io::ErrorKind::InvalidInput.into())),
                '/' => Some(Ok(idx)),
                _ => None
            });
        match split_point {
            Some(Ok(idx)) => {
                let (name, path) = self.path.split_at(idx);
                self.path = &path[1..];
                let last = path.is_empty();
                Some(Ok(PathPart {
                    name, last
                }))
            }
            Some(Err(e)) => Some(Err(e)),
            None => if self.path.is_empty() {
                None
            }
            else {
                let name = self.path;
                let last = true;
                self.path = "";
                Some(Ok(PathPart {
                    name, last
                }))
            }
        }
    }
}
