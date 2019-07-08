use super::node::*;
use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct File {
    node: Arc<Mutex<Node>>,
    rdr_pos: usize,
}

impl File {
    pub(crate) fn new(node: Arc<Mutex<Node>>) -> Self {
        Self { node, rdr_pos: 0 }
    }

    pub fn reset(&mut self) {
        self.rdr_pos = 0;
    }
}

impl Read for File {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.rdr_pos == self.node.lock().unwrap().contents.len() {
            return Ok(0);
        }

        let result = (&self.node.lock().unwrap().contents[self.rdr_pos..]).read(buf);
        result.map(|count| {
            self.rdr_pos += count;
            count
        })
    }
}

impl Write for File {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.node.lock().unwrap().contents.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.node.lock().unwrap().contents.flush()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn read() {
        let mut file = File::new(new_file_node("test.txt"));
        file.node
            .lock()
            .unwrap()
            .contents
            .write_all(b"Hello world!")
            .unwrap();

        let mut contents = Vec::new();
        file.read_to_end(&mut contents).unwrap();
        assert_eq!(file.node.lock().unwrap().contents, contents);

        // once read, need to reset to read again
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).unwrap();
        assert!(contents.is_empty());

        file.reset();
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).unwrap();
        assert_eq!(file.node.lock().unwrap().contents, contents);
    }

    #[test]
    fn write() {
        let mut file = File::new(new_file_node("test.txt"));

        let contents = b"Hello world!";
        file.write_all(contents).unwrap();
        assert_eq!(file.node.lock().unwrap().contents, b"Hello world!");

        let contents = b" This is a test...";
        file.write_all(contents).unwrap();
        assert_eq!(
            file.node.lock().unwrap().contents,
            b"Hello world! This is a test..."
        );

        assert!(file.flush().is_ok());
    }
}
