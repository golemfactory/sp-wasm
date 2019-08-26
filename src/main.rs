#![warn(clippy::all)]

use sp_wasm_engine::prelude::*;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Opts {
    #[structopt(short, parse(from_occurrences))]
    verbose: u8,
    #[structopt(subcommand)]
    command: Command,
}

#[derive(StructOpt)]
enum Command {
    /// Run a command
    #[structopt(name = "run")]
    Run {
        /// Memory limit (0 - unlimited)
        #[structopt(long, short, default_value = "0")]
        memory: u64,
        /// Bind mount a volume
        #[structopt(long, short)]
        volume: Vec<String>,
        #[structopt(long = "workdir", short)]
        work_dir: Option<String>,
        program: PathBuf,
        args: Vec<String>,
    },
}

fn main() -> failure::Fallible<()> {
    let opts = Opts::from_args();
    env_logger::init_from_env(
        env_logger::Env::default().default_filter_or(match opts.verbose {
            0 => "error",
            1 => "info",
            _ => "sp_wasm_engine=debug,info",
        }),
    );

    match opts.command {
        Command::Run {
            memory,
            volume,
            work_dir,
            program,
            args,
        } => {
            let program_js = program.with_extension("js");
            let program_wasm = program.with_extension("wasm");

            let mut sb = Sandbox::new()?.set_exec_args(args)?;

            sb.init()?;

            sb.run(program_js.to_str().unwrap(), program_wasm.to_str().unwrap())?;

            /*.and_then(|sandbox| sandbox.load_input_files(input_dir))
            .and_then(|sandbox| sandbox.run(wasm_js, wasm_bin))
            .and_then(|sandbox| sandbox.save_output_files(output_dir, output_files.iter()))?;*/
        }
    }

    Ok(())
}
