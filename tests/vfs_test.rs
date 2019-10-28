mod common;

use common::*;
use sp_wasm_engine::prelude::*;
use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

fn vfs_impl() -> Result<(), String> {
    let test_dir = create_workspace()?;
    let mut dir = PathBuf::from(test_dir.path());
    dir.push("a.txt");
    let mut f = File::create(dir).map_err(|err| err.to_string())?;
    f.write_all(b"aaa").map_err(|err| err.to_string())?;

    let mut dir = PathBuf::from(test_dir.path());
    dir.push("sub/");
    fs::create_dir(dir.as_path()).map_err(|err| err.to_string())?;
    dir.push("b.txt");
    let mut f = File::create(dir).map_err(|err| err.to_string())?;
    f.write_all(b"bbb").map_err(|err| err.to_string())?;

    // map into VFS
    let mut vfs = VirtualFS::new();
    vfs.map_path(test_dir.path(), "/", &mut |_, _| {})
        .map_err(|err| err.to_string())?;

    let contents = vfs.read_file("/a.txt").map_err(|err| err.to_string())?;
    assert_eq!(b"aaa".to_vec(), contents);

    let contents = vfs.read_file("/sub/b.txt").map_err(|err| err.to_string())?;
    assert_eq!(b"bbb".to_vec(), contents);

    // create file directly in VFS
    vfs.write_file("/sub/c.txt", b"ccc")
        .map_err(|err| err.to_string())?;

    let contents = vfs.read_file("/sub/c.txt").map_err(|err| err.to_string())?;
    assert_eq!(b"ccc".to_vec(), contents);

    // try & create file in subdir that doesn't exist
    let res = vfs.write_file("/sub/sub2/d.txt", b"ddd");
    assert_eq!(res.is_err(), true);

    Ok(())
}

#[test]
fn vfs() {
    if let Err(e) = vfs_impl() {
        eprintln!("unexpected error occurred: {}", e)
    }
}
