use sp_wasm_engine::prelude::*;
use std::path::PathBuf;
use structopt::StructOpt;

/// Standalone SpiderMonkey instance that can be used to run Emscripten
/// generated Wasm according to the Golem calling convention.
#[derive(StructOpt, Debug)]
#[structopt(name = "wasm-sandbox", version = env!("CARGO_PKG_VERSION"))]
struct Opts {
    /// Path to input dir
    #[structopt(short = "I", long = "input_dir", parse(from_os_str))]
    input_dir: PathBuf,
    /// Path to output dir
    #[structopt(short = "O", long = "output_dir", parse(from_os_str))]
    output_dir: PathBuf,
    /// Path to Emscripten JavaScript glue code
    #[structopt(short = "j", long = "wasm_js", parse(from_os_str))]
    wasm_js: PathBuf,
    /// Path to Emscripten Wasm binary
    #[structopt(short = "w", long = "wasm_bin", parse(from_os_str))]
    wasm_bin: PathBuf,
    /// Paths to expected output files
    #[structopt(
        short = "o",
        long = "output_file",
        parse(from_os_str),
        number_of_values = 1
    )]
    output_files: Vec<PathBuf>,
    /// The args to pass to Wasm module
    #[structopt()]
    args: Vec<String>,
}

fn main() {
    let opts = Opts::from_args();
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    Sandbox::new()
        .and_then(|sandbox| sandbox.set_exec_args(opts.args.iter()))
        .and_then(|sandbox| sandbox.load_input_files(&opts.input_dir))
        .and_then(|sandbox| sandbox.run(&opts.wasm_js, &opts.wasm_bin))
        .and_then(|sandbox| sandbox.save_output_files(&opts.output_dir, opts.output_files.iter()))
        .unwrap_or_else(|err| {
            eprintln!("{}", err);
            std::process::exit(1)
        });
}
