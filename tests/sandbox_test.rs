mod common;

use common::*;
use sp_wasm_engine::prelude::*;
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
};

const INPUT_PART1: &'static [u8] = include_bytes!("assets/aaa.txt");
const INPUT_PART2: &'static [u8] = include_bytes!("assets/bbb.txt");
const EM_JS: &'static [u8] = include_bytes!("assets/test.js");
const EM_WASM: &'static [u8] = include_bytes!("assets/test.wasm");

fn sandbox_impl() -> Result<(), String> {
    let test_dir = create_workspace()?;
    let mut input_dir = PathBuf::from(test_dir.path());
    input_dir.push("in/");
    fs::create_dir(input_dir.as_path()).map_err(|err| err.to_string())?;
    let mut f = File::create(input_dir.join("aaa.txt")).map_err(|err| err.to_string())?;
    f.write_all(INPUT_PART1).map_err(|err| err.to_string())?;

    let mut input_subdir = PathBuf::from(input_dir.as_path());
    input_subdir.push("a/");
    fs::create_dir(input_subdir.as_path()).map_err(|err| err.to_string())?;
    let mut f = File::create(input_subdir.join("bbb.txt")).map_err(|err| err.to_string())?;
    f.write_all(INPUT_PART2).map_err(|err| err.to_string())?;

    let mut js = PathBuf::from(test_dir.path());
    js.push("test.js");
    let mut f = File::create(js.as_path()).map_err(|err| err.to_string())?;
    f.write_all(EM_JS).map_err(|err| err.to_string())?;

    let mut wasm = PathBuf::from(test_dir.path());
    wasm.push("test.wasm");
    let mut f = File::create(wasm.as_path()).map_err(|err| err.to_string())?;
    f.write_all(EM_WASM).map_err(|err| err.to_string())?;

    let mut output_dir = PathBuf::from(test_dir.path());
    output_dir.push("out/");
    fs::create_dir(output_dir.as_path()).map_err(|err| err.to_string())?;

    let engine = Engine::new().map_err(|err| err.to_string())?;
    Sandbox::new(&engine)
        .and_then(|sandbox| sandbox.set_exec_args(vec!["test"]))
        .and_then(|sandbox| sandbox.load_input_files(input_dir.to_str().unwrap()))
        .and_then(|sandbox| sandbox.run(js.to_str().unwrap(), wasm.to_str().unwrap()))
        .and_then(|sandbox| {
            sandbox.save_output_files(output_dir.to_str().unwrap(), vec!["ccc.txt", "c/ddd.txt"])
        })
        .map_err(|err| err.to_string())?;

    let mut file = File::open(output_dir.join("ccc.txt")).map_err(|err| err.to_string())?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|err| err.to_string())?;

    assert_eq!("THIS IS PART1:\ntest\ntest\n".to_owned(), contents);

    let mut file = File::open(output_dir.join("c/ddd.txt")).map_err(|err| err.to_string())?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|err| err.to_string())?;

    assert_eq!("THIS IS PART2:\ninput\ntest\n".to_owned(), contents);

    Ok(())
}

#[test]
fn sandbox() {
    if let Err(e) = sandbox_impl() {
        eprintln!("unexpected error occurred: {}", e)
    }
}
