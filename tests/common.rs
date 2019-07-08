use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

pub fn create_workspace<S: AsRef<str>>(test_name: S) -> Result<PathBuf, String> {
    let mut workspace = env::temp_dir();
    let time_now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|err| err.to_string())?;
    let subdir = format!(
        "sp_wasm_tests_{}_{}",
        test_name.as_ref(),
        time_now.as_secs()
    );
    workspace.push(subdir);
    fs::create_dir(workspace.as_path()).map_err(|err| err.to_string())?;

    Ok(workspace)
}
