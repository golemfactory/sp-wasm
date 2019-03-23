pub mod engine;
pub mod vfs;

use self::engine::*;
use self::vfs::*;

use itertools::Itertools;
use std::error::Error;

use lazy_static::lazy_static;
use std::path;
use std::sync::Mutex;

static REPO_PATH: &str = "mem://sp_wasm";
static REPO_PASS: &str = "wasm@golem";

lazy_static! {
    static ref VFS: Mutex<VirtualFS> =
        Mutex::new(VirtualFS::new(REPO_PATH, REPO_PASS).expect("couldn't create VirtualFS"));
}

pub struct Sandbox {
    engine: Engine,
}

impl Sandbox {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let engine = Engine::new()?;

        Ok(Self { engine })
    }

    pub fn set_exec_args<It>(self, exec_args: It) -> Result<Self, Box<dyn Error>>
    where
        It: IntoIterator,
        It::Item: AsRef<str>,
    {
        let exec_args = exec_args
            .into_iter()
            .map(|s| format!("'{}'", s.as_ref()))
            .join(", ");
        log::info!("Setting exec args [ {} ]", exec_args);

        let js = format!("Module['arguments'] = [ {} ];", exec_args);
        self.engine.evaluate_script(&js)?;

        Ok(self)
    }

    pub fn load_input_files<S>(self, input_path: S) -> Result<Self, Box<dyn Error>>
    where
        S: AsRef<str>,
    {
        log::info!("Loading input files at {}", input_path.as_ref());

        let mut js = "
        Module['preRun'] = function() {
        "
        .to_string();

        VFS.lock()
            .unwrap()
            .map_path(input_path.as_ref(), "/", &mut |abs_path, rel_path| {
                let rel_path_s: String = rel_path.to_string_lossy().into();
                if abs_path.is_dir() {
                    // create dir
                    js += &format!("FS.mkdir('{}');", rel_path_s);
                } else {
                    // create file
                    js += &format!(
                        "\n\tFS.writeFile('{}', new Uint8Array(readFile('{}')));",
                        rel_path_s, rel_path_s
                    );
                }
            })?;

        js += "\n};";
        self.engine.evaluate_script(&js)?;

        Ok(self)
    }

    pub fn run<S>(self, wasm_js: S, wasm_bin: S) -> Result<Self, Box<dyn Error>>
    where
        S: AsRef<str>,
    {
        log::info!("Running WASM {}", wasm_bin.as_ref());

        VFS.lock()
            .unwrap()
            .map_file(wasm_bin.as_ref(), "/main.wasm")?;

        let mut js = "Module['wasmBinary'] = readFile('/main.wasm');".to_string();
        let wasm_js = vfs::read_file(wasm_js.as_ref())?;
        let wasm_js = String::from_utf8(wasm_js)?;
        js += &wasm_js;
        self.engine.evaluate_script(&js)?;

        Ok(self)
    }

    pub fn save_output_files<S, It>(
        self,
        output_path: S,
        output_files: It,
    ) -> Result<(), Box<dyn Error>>
    where
        S: AsRef<str>,
        It: IntoIterator,
        It::Item: AsRef<str>,
    {
        for output_file in output_files {
            let mut output_rel_path = path::PathBuf::from("/");
            output_rel_path.push(output_file.as_ref());
            let output_rel_path_str: String = output_rel_path.as_path().to_string_lossy().into();

            self.engine.evaluate_script(&format!(
                "writeFile('{}', FS.readFile('{}'));",
                output_rel_path_str, output_rel_path_str,
            ))?;

            let mut output_abs_path = path::PathBuf::from(output_path.as_ref());
            output_abs_path.push(output_file.as_ref());

            log::info!(
                "Saving output at {}",
                output_abs_path.as_path().to_string_lossy()
            );

            let contents = VFS.lock().unwrap().read_file(output_rel_path)?;
            vfs::write_file(output_abs_path, &contents)?;
        }

        Ok(())
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }
}
