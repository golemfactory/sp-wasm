use sp_wasm_engine::prelude::*;
use sp_wasm_tests::{_assert_eq, create_tmp, destroy_tmp, unwrap_res};

use std::error;
use std::fs;
use std::io::Write;
use std::path;

fn run<P>(test_dir: P) -> Result<(), Box<dyn error::Error>>
where
    P: AsRef<path::Path>,
{
    // create dir structure in /tmp
    let mut dir = path::PathBuf::from(test_dir.as_ref());
    dir.push("a.txt");
    let mut f = fs::File::create(dir)?;
    f.write_all(b"aaa")?;

    let mut dir = path::PathBuf::from(test_dir.as_ref());
    dir.push("sub/");
    fs::create_dir(dir.as_path())?;
    dir.push("b.txt");
    let mut f = fs::File::create(dir)?;
    f.write_all(b"bbb")?;

    // map into VFS
    let mut vfs = VirtualFS::new();
    vfs.map_path(test_dir.as_ref(), "/", &mut |_, _| {})?;

    let contents = vfs.read_file("/a.txt")?;
    _assert_eq(b"aaa".to_vec(), contents)?;

    let contents = vfs.read_file("/sub/b.txt")?;
    _assert_eq(b"bbb".to_vec(), contents)?;

    // create file directly in VFS
    vfs.write_file("/sub/c.txt", b"ccc")?;

    let contents = vfs.read_file("/sub/c.txt")?;
    _assert_eq(b"ccc".to_vec(), contents)?;

    // try & create file in subdir that doesn't exist
    let res = vfs.write_file("/sub/sub2/d.txt", b"ddd");
    _assert_eq(res.is_err(), true)?;

    Ok(())
}

fn main() {
    let dir = create_tmp();
    let res = run(dir.as_path());
    destroy_tmp(dir);
    unwrap_res(res);
}
