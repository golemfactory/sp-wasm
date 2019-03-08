#![warn(clippy::all)]

#[macro_use]
extern crate mozjs;
extern crate libc;

mod sandbox;

use sandbox::Sandbox;

use docopt::Docopt;
use serde::Deserialize;

const USAGE: &str = "
Standalone SpiderMonkey instance that can be used to run any Emscripten
generated WASM according to the Golem calling convention.

Usage:
    sp_wasm -I <input-dir> -O <output-dir> -j <wasm-js> -w <wasm> -o <output-file>...
    sp_wasm (-h | --help)

Options:
    -h --help       Show this screen.
";

#[derive(Debug, Deserialize)]
struct Args {
    arg_input_dir: String,
    arg_output_dir: String,
    arg_wasm_js: String,
    arg_wasm: String,
    arg_output_file: Vec<String>,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|dopt| dopt.deserialize())
        .unwrap_or_else(|e| e.exit());

    env_logger::init();

    Sandbox::new()
        .and_then(|sandbox| sandbox.load_input_files(&args.arg_input_dir))
        .and_then(|sandbox| sandbox.run(&args.arg_wasm_js, &args.arg_wasm))
        .and_then(|sandbox| {
            sandbox.save_output_files(&args.arg_output_dir, args.arg_output_file.iter())
        })
        .unwrap_or_else(|err| eprintln!("{}", err));
}
