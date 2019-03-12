mod engine;
mod vfs;

use self::engine::*;

use itertools::Itertools;
use std::error::Error;

use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    static ref VFS: Mutex<vfs::VirtualFS> = Mutex::new(vfs::VirtualFS::new());
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
            .map_path(input_path.as_ref(), &mut |rel_path, node| {
                let path: String = rel_path.to_string_lossy().into();
                match node {
                    vfs::FSNode::File(_) => {
                        // create file
                        js += &format!(
                            "\n\tFS.writeFile('{}', new Uint8Array(readFile('{}')));",
                            path, path
                        );
                    }
                    vfs::FSNode::Dir => {
                        // create dir
                        if path != "/" {
                            js += &format!("FS.mkdir('{}');", path);
                        }
                    }
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
            let mut output_path = std::path::PathBuf::from(output_path.as_ref());
            output_path.push(vfs::sanitize_path(output_file.as_ref())?);

            log::info!(
                "Saving output at {}",
                output_path.as_path().to_string_lossy()
            );

            self.engine.evaluate_script(&format!(
                "writeFile('{}', FS.readFile('{}'));",
                output_path.as_path().to_string_lossy(),
                output_file.as_ref()
            ))?;
        }

        Ok(())
    }
}
