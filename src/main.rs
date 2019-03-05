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
    sp_wasm <workspace-dir> <wasm-js> <wasm>
    sp_wasm (-h | --help)

Options:
    -h --help       Show this screen.
";

#[derive(Debug, Deserialize)]
struct Args {
    arg_workspace_dir: String,
    arg_wasm_js: String,
    arg_wasm: String,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|dopt| dopt.deserialize())
        .unwrap_or_else(|e| e.exit());

    let mut sandbox = Sandbox::new();
    sandbox.map_workspace(&args.arg_workspace_dir);
    sandbox.run(&args.arg_wasm_js, &args.arg_wasm);
}
