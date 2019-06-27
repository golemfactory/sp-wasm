#![warn(clippy::all)]

use clap::{value_t_or_exit, values_t_or_exit, App, Arg};
use sp_wasm_engine::prelude::*;

fn main() {
    let args = App::new("wasm-sandbox")
        .version("0.2.1")
        .author("Golem RnD Team <contact@golem.network>")
        .about("Standalone SpiderMonkey instance that can be used to run Emscripten generated Wasm according to the Golem calling convention.")
        .arg(
            Arg::with_name("input_dir")
                .short("I")
                .value_name("input-dir")
                .required(true)
                .takes_value(true)
                .help("Path to input dir"),
        )
        .arg(
            Arg::with_name("output_dir")
                .short("O")
                .value_name("output-dir")
                .required(true)
                .takes_value(true)
                .help("Path to output dir"),
        )
        .arg(
            Arg::with_name("wasm_js")
                .short("j")
                .value_name("wasm-js")
                .required(true)
                .takes_value(true)
                .help("Path to Emscripten JavaScript glue code"),
        )
        .arg(
            Arg::with_name("wasm_bin")
                .short("w")
                .value_name("wasm-bin")
                .required(true)
                .takes_value(true)
                .help("Path to Emscripten Wasm binary"),
        )
        .arg(
            Arg::with_name("output_file")
                .short("o")
                .value_name("output-file")
                .required(true)
                .multiple(true)
                .help("Path to expected output file"),
        )
        .arg(
            Arg::with_name("wasm_args")
                .value_name("wasm-args")
                .multiple(true)
                .help("Wasm program args"),
        )
        .get_matches();

    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    let input_dir = value_t_or_exit!(args.value_of("input_dir"), String);
    let output_dir = value_t_or_exit!(args.value_of("output_dir"), String);
    let wasm_js = value_t_or_exit!(args.value_of("wasm_js"), String);
    let wasm_bin = value_t_or_exit!(args.value_of("wasm_bin"), String);
    let output_files = values_t_or_exit!(args.values_of("output_file"), String);
    let wasm_args = values_t_or_exit!(args.values_of("wasm_args"), String);

    Sandbox::new()
        .and_then(|sandbox| sandbox.set_exec_args(wasm_args.iter()))
        .and_then(|sandbox| sandbox.load_input_files(input_dir))
        .and_then(|sandbox| sandbox.run(wasm_js, wasm_bin))
        .and_then(|sandbox| sandbox.save_output_files(output_dir, output_files.iter()))
        .unwrap_or_else(|err| eprintln!("{}", err));
}
