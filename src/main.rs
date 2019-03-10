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
    sp_wasm -I <input-dir> -O <output-dir> -j <wasm-js> -w <wasm> -o <output-file>... [-- <args>...]
    sp_wasm (-h | --help)

Options:
    -I <input-dir>          Path to the input dir.
    -O <output-dir>         Path to the output dir.
    -j <wasm-js>            Path to the Emscripten JavaScript glue code.
    -w <wasm-bin>           Path to the Emscripten WASM binary.
    -o <output-file>        Path to expected file.
    <args>                  WASM program arguments. 
    -h --help               Show this screen.
";

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct Args {
    flag_I: String,
    flag_O: String,
    flag_j: String,
    flag_w: String,
    flag_o: Vec<String>,
    arg_args: Vec<String>,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|dopt| dopt.deserialize())
        .unwrap_or_else(|e| e.exit());

    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    Sandbox::new()
        .and_then(|sandbox| sandbox.set_exec_args(args.arg_args.iter()))
        .and_then(|sandbox| sandbox.load_input_files(&args.flag_I))
        .and_then(|sandbox| sandbox.run(&args.flag_j, &args.flag_w))
        .and_then(|sandbox| sandbox.save_output_files(&args.flag_O, args.flag_o.iter()))
        .unwrap_or_else(|err| eprintln!("{}", err));
}
