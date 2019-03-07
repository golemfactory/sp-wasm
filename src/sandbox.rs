mod engine;
mod vfs;

use self::engine::*;

use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    static ref VFS: Mutex<vfs::VirtualFS> = Mutex::new(vfs::VirtualFS::new());
}

pub struct Sandbox {
    engine: Engine,
}

impl Sandbox {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_input_files<S>(&mut self, input_path: S)
    where
        S: AsRef<str>,
    {
        log::info!("Loading input files at {}", input_path.as_ref());

        let mut js = "
        Module['preRun'] = function() {
        "
        .to_string();

        let vfs = &mut VFS.lock().unwrap();
        vfs.map_path(input_path.as_ref(), &mut |rel_path, node| {
            let path = rel_path.to_str().unwrap();
            match node {
                vfs::FSNode::File(_) => {
                    // create file
                    js += &format!("\n\tFS.writeFile('{}', readFile('{}'));", path, path);
                }
                vfs::FSNode::Dir => {
                    // create dir
                    js += &format!("FS.mkdir('{}');", path);
                }
            }
        })
        .unwrap_or_else(|err| {
            panic!(
                "Failed to map {} into VirtualFS with error {}",
                input_path.as_ref(),
                err
            )
        });

        js += "\n};";

        log::debug!("{}", js);

        self.engine.evaluate_script(&js);
    }

    pub fn run<S>(&self, wasm_js: S, wasm_bin: S)
    where
        S: AsRef<str>,
    {
        VFS.lock()
            .unwrap()
            .map_file(wasm_bin.as_ref(), "/main.wasm")
            .unwrap();

        let mut js = "Module['wasmBinary'] = readFile('/main.wasm');".to_string();
        let wasm_js = vfs::read_file(wasm_js.as_ref()).unwrap_or_else(|err| {
            panic!(
                "Failed to read JavaScript file {} with error {}",
                wasm_js.as_ref(),
                err
            )
        });
        let wasm_js = String::from_utf8(wasm_js)
            .unwrap_or_else(|err| panic!("Failed to parse JavaScript with error {}", err));
        js += &wasm_js;
        self.engine.evaluate_script(&js);
    }

    pub fn save_output_files<S, It>(&self, output_path: S, output_files: It)
    where
        S: AsRef<str>,
        It: IntoIterator,
        It::Item: AsRef<str>,
    {
        for output_file in output_files {
            let output_file = path_clean::clean(&("/".to_string() + output_file.as_ref()));
            let output_path = output_path.as_ref().to_string() + &output_file;

            log::debug!("Saving output at {:?}", output_path);

            self.engine.evaluate_script(&format!(
                "writeFile('{}', FS.readFile('{}'));",
                output_path, output_file
            ));
        }
    }
}

impl Default for Sandbox {
    fn default() -> Self {
        Self {
            engine: Engine::new(),
        }
    }
}
