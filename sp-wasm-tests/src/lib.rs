use std::env;
use std::error;
use std::fmt;
use std::fs;
use std::path;

pub fn create_tmp() -> path::PathBuf {
    let mut test_dir = env::temp_dir();
    test_dir.push("sp_wasm_tests/");
    fs::create_dir(test_dir.as_path())
        .unwrap_or_else(|_| panic!("couldn't create test dir in {:?}", test_dir));
    test_dir
}

pub fn destroy_tmp<P>(path: P)
where
    P: AsRef<path::Path>,
{
    fs::remove_dir_all(path).unwrap()
}

pub fn unwrap_res(result: Result<(), Box<dyn error::Error>>) {
    if let Err(err) = result {
        panic!("{}", err)
    }
}

#[derive(Debug)]
pub struct AssertionError<T: fmt::Debug + PartialEq> {
    left: T,
    right: T,
}

impl<T> error::Error for AssertionError<T> where T: fmt::Debug + PartialEq {}

impl<T> fmt::Display for AssertionError<T>
where
    T: fmt::Debug + PartialEq,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "(left == right)\n  left: {:?},\n right: {:?},\n",
            self.left, self.right
        )
    }
}

pub fn _assert_eq<T>(left: T, right: T) -> Result<(), AssertionError<T>>
where
    T: fmt::Debug + PartialEq,
{
    if left == right {
        Ok(())
    } else {
        Err(AssertionError { left, right })
    }
}
