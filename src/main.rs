#![warn(clippy::all)]

#[macro_use]
extern crate mozjs;
extern crate libc;

mod logger;
mod sandbox;

use sandbox::Sandbox;

use docopt::Docopt;
use serde::Deserialize;

const USAGE: &str = "
Standalone SpiderMonkey instance that can be used to run any Emscripten
generated WASM according to the Golem calling convention.

Usage:
    sp_wasm -O <output-dir> -I <input-dir> -j <wasm-js> -w <wasm> -o <output-file> [-v | --verbose]
    sp_wasm (-h | --help)

Options:
    -O              Path to the output directory.
    -I              Path to the input directory.
    -j              Path to the JS glue script.
    -w              Path to the WASM binary.
    -o              Path to the expected output file produced by WASM binary.
    -v --verbose    Turn logging on.
    -h --help       Show this screen.
";

#[derive(Debug, Deserialize)]
struct Args {
    arg_output_dir: String,
    arg_input_dir: String,
    arg_wasm_js: String,
    arg_wasm: String,
    arg_output_file: String,
    flag_verbose: bool,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|dopt| dopt.deserialize())
        .unwrap_or_else(|e| e.exit());

    if args.flag_verbose {
        logger::init().unwrap_or_else(|err| eprintln!("Failed to initialize logger: {}", err));
    }

    let mut sandbox = Sandbox::new();
    // sandbox.add_input_dir(args.arg_input_dir);
    // sandbox.add_output_dir(args.arg_output_dir);
    // sandbox.add_output_file(args.arg_output_file);
    // sandbox.load_wasm(args.arg_wasm_js, args.arg_wasm);
    sandbox.run();
}
