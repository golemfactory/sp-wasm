pub use tempfile::TempDir;

pub fn create_workspace() -> Result<TempDir, String> {
    use tempfile::Builder;

    Builder::new()
        .prefix("sp-wasm")
        .tempdir()
        .map_err(|err| format!("couldn't create temp dir with error: {}", err))
}
