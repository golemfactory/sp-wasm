use sp_wasm_engine::prelude::*;
use sp_wasm_tests::*;

use std::error;
use std::fs;
use std::io::{Read, Write};
use std::path;

fn run<P>(test_dir: P) -> Result<(), Box<dyn error::Error>>
where
    P: AsRef<path::Path>,
{
    const INPUT_PART1: &'static [u8] = include_bytes!("../../assets/aaa.txt");
    const INPUT_PART2: &'static [u8] = include_bytes!("../../assets/bbb.txt");
    const EM_JS: &'static [u8] = include_bytes!("../../assets/test.js");
    const EM_WASM: &'static [u8] = include_bytes!("../../assets/test.wasm");

    let mut input_dir = path::PathBuf::from(test_dir.as_ref());
    input_dir.push("in/");
    fs::create_dir(input_dir.as_path())?;
    let mut f = fs::File::create(input_dir.join("aaa.txt"))?;
    f.write_all(INPUT_PART1)?;

    let mut input_subdir = path::PathBuf::from(input_dir.as_path());
    input_subdir.push("a/");
    fs::create_dir(input_subdir.as_path())?;
    let mut f = fs::File::create(input_subdir.join("bbb.txt"))?;
    f.write_all(INPUT_PART2)?;

    let mut js = path::PathBuf::from(test_dir.as_ref());
    js.push("test.js");
    let mut f = fs::File::create(js.as_path())?;
    f.write_all(EM_JS)?;

    let mut wasm = path::PathBuf::from(test_dir.as_ref());
    wasm.push("test.wasm");
    let mut f = fs::File::create(wasm.as_path())?;
    f.write_all(EM_WASM)?;

    let mut output_dir = path::PathBuf::from(test_dir.as_ref());
    output_dir.push("out/");
    fs::create_dir(output_dir.as_path())?;

    Sandbox::new()
        .and_then(|sandbox| sandbox.set_exec_args(vec!["test"]))
        .and_then(|sandbox| sandbox.load_input_files(input_dir.to_str().unwrap()))
        .and_then(|sandbox| sandbox.run(js.to_str().unwrap(), wasm.to_str().unwrap()))
        .and_then(|sandbox| {
            sandbox.save_output_files(output_dir.to_str().unwrap(), vec!["ccc.txt", "c/ddd.txt"])
        })?;

    let mut file = fs::File::open(output_dir.join("ccc.txt"))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    _assert_eq("THIS IS PART1:\ntest\ntest\n".to_owned(), contents)?;

    let mut file = fs::File::open(output_dir.join("c/ddd.txt"))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    _assert_eq("THIS IS PART2:\ninput\ntest\n".to_owned(), contents)?;

    Ok(())
}

fn main() {
    let dir = create_tmp();
    let res = run(dir.as_path());
    destroy_tmp(dir);
    unwrap_res(res);
}
