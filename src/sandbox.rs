mod engine;
mod file_manip;

use engine::Engine;
use file_manip::*;

pub struct Sandbox {
    engine: Engine,
    vfs: zbox::Repo,
}

impl Sandbox {
    const SANDBOX_PATH: &'static str = "sandbox";
    const SANDBOX_PWD: &'static str = "$andb0x_g0l3m_w@sm";

    pub fn new() -> Self {
        Self::default()
    }

    pub fn map_input_path(&mut self, input_path: &str) {
        log::info!("Mapping input directories at {}", input_path);

        map_path(&mut self.vfs, input_path).unwrap_or_else(|err| {
            panic!(
                "Failed to map {} into VirtualFS with error {}",
                input_path, err
            )
        })
    }

    pub fn map_output_path(&mut self, output_path: &str) {
        log::info!("Mapping output directories at {}", output_path);

        map_path(&mut self.vfs, output_path).unwrap_or_else(|err| {
            panic!(
                "Failed to map {} into VirtualFS with error {}",
                output_path, err
            )
        })
    }

    pub fn run(&self, wasm_js: &str, wasm_bin: &str) {
        let mut js = format!("Module['wasmBinary'] = readFile('{}');", wasm_bin);
        let wasm_js = read_file(wasm_js).unwrap_or_else(|err| {
            panic!(
                "Failed to read JavaScript file {} with error {}",
                wasm_js, err
            )
        });
        let wasm_js = String::from_utf8(wasm_js)
            .unwrap_or_else(|err| panic!("Failed to parse JavaScript with error {}", err));
        js += &wasm_js;
        self.engine.evaluate_script(&js);
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        // remove repo if was created successfully
        if let Ok(res) = zbox::Repo::exists(&format!("file://{}", Self::SANDBOX_PATH)) {
            if res {
                std::fs::remove_dir_all(Self::SANDBOX_PATH)
                    .unwrap_or_else(|err| log::error!("Sandbox VirtualFS didn't exist: {:?}", err));

                log::debug!("Sandbox VirtualFS removed");
            }
        }
    }
}

impl Default for Sandbox {
    fn default() -> Self {
        zbox::init_env();
        let path = &format!("file://{}", Self::SANDBOX_PATH);

        Self {
            engine: Engine::new(),
            vfs: zbox::RepoOpener::new()
                .create_new(true)
                .open(path, Self::SANDBOX_PWD)
                .unwrap_or_else(|err| {
                    panic!("Failed to create new VFS at {} with error: {}", path, err,)
                }),
        }
    }
}
