mod common;

use common::*;
use sp_wasm_engine::prelude::*;
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
};

const EM_JS: &'static [u8] = include_bytes!("assets/gettimeofday.js");
const EM_WASM: &'static [u8] = include_bytes!("assets/gettimeofday.wasm");

fn gettimeofday_impl() -> Result<(), String> {
    let test_dir = create_workspace()?;
    let mut input_dir = PathBuf::from(test_dir.path());
    input_dir.push("in/");
    fs::create_dir(input_dir.as_path()).map_err(|err| err.to_string())?;

    let mut js = PathBuf::from(test_dir.path());
    js.push("gettimeofday.js");
    let mut f = File::create(js.as_path()).map_err(|err| err.to_string())?;
    f.write_all(EM_JS).map_err(|err| err.to_string())?;

    let mut wasm = PathBuf::from(test_dir.path());
    wasm.push("gettimeofday.wasm");
    let mut f = File::create(wasm.as_path()).map_err(|err| err.to_string())?;
    f.write_all(EM_WASM).map_err(|err| err.to_string())?;

    let mut output_dir = PathBuf::from(test_dir.path());
    output_dir.push("out/");
    fs::create_dir(output_dir.as_path()).map_err(|err| err.to_string())?;

    let engine = Engine::new().map_err(|err| err.to_string())?;
    Sandbox::new(&engine)
        .and_then(|sandbox| sandbox.load_input_files(input_dir.to_str().unwrap()))
        .and_then(|sandbox| sandbox.run(js.to_str().unwrap(), wasm.to_str().unwrap()))
        .and_then(|sandbox| {
            sandbox.save_output_files(output_dir.to_str().unwrap(), vec!["out.txt"])
        })
        .map_err(|err| err.to_string())?;

    let mut file = File::open(output_dir.join("out.txt")).map_err(|err| err.to_string())?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|err| err.to_string())?;

    assert_eq!("0\n".to_owned(), contents);

    Ok(())
}

#[test]
fn gettimeofday() {
    if let Err(e) = gettimeofday_impl() {
        eprintln!("unexpected error occurred: {}", e)
    }
}
