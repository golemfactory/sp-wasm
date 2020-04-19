use version_check;

fn main() {
    match version_check::is_max_version("1.38.0") {
        None => println!("cargo:warning=sp-wasm: error querying Rust version"),
        Some(false) => panic!("sp-wasm: only Rust <= 1.38.0 is supported, build aborted"),
        Some(true) => {}
    };
}
