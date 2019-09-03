pub mod engine;
pub mod vfs;

use self::engine::*;
use self::vfs::*;
use super::Result;
use std::sync::Mutex;
use std::{io, path};

use itertools::Itertools;
use lazy_static::lazy_static;
use sp_wasm_hostfs::vfsdo::NodeMode;
use sp_wasm_hostfs::vfsops::VfsVolume;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

lazy_static! {
    static ref VFS: Mutex<VirtualFS> = Mutex::new(VirtualFS::default());
}

pub struct Sandbox {
    engine: Engine,
}

impl Sandbox {
    pub fn new() -> Result<Self> {
        let engine = Engine::new()?;

        Ok(Self { engine })
    }

    pub fn set_exec_args<It>(self, exec_args: It) -> Result<Self>
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

    pub fn work_dir(self, dir : &str) -> Result<Self> {
        let js = format!("Module['work_dir'] = {};", serde_json::to_string(dir)?);
        self.engine.evaluate_script(&js)?;
        Ok(self)
    }

    pub fn init(&mut self) -> Result<()> {
        let preload = include_str!("preload.js");

        self.engine.evaluate_script(preload)?;
        Ok(())
    }

    pub fn load_input_files<S>(self, input_path: S) -> Result<Self>
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
            .map_path(input_path.as_ref(), "/", &mut |source_path, dest_path| {
                let dest_path_s: String = dest_path.to_string_lossy().into();
                if source_path.is_dir() {
                    // create dir
                    js += &format!("FS.mkdir('{}');", dest_path_s);
                } else {
                    // create file
                    js += &format!(
                        "\n\tFS.writeFile('{}', new Uint8Array(readFile('{}')));",
                        dest_path_s, dest_path_s
                    );
                }
            })?;

        js += "\n};";
        self.engine.evaluate_script(&js)?;

        Ok(self)
    }

    pub fn run<S>(self, wasm_js: S, wasm_bin: S) -> Result<Self>
    where
        S: AsRef<str>,
    {
        log::info!("Running WASM {}", wasm_bin.as_ref());

        VFS.lock()
            .unwrap()
            .map_file(wasm_bin.as_ref(), "/main.wasm")?;

        let mut js = "Module['wasmBinary'] = readFile('/main.wasm');".to_string();
        let wasm_js = hostfs::read_file(wasm_js.as_ref())?;
        let wasm_js = String::from_utf8(wasm_js)?;
        js += &wasm_js;
        self.engine.evaluate_script(&js)?;

        Ok(self)
    }

    pub fn save_output_files<S, It>(self, output_path: S, output_files: It) -> Result<()>
    where
        S: AsRef<str>,
        It: IntoIterator,
        It::Item: AsRef<str>,
    {
        for output_file in output_files {
            // sanitize output file path (may contain subdirs)
            let output_file = hostfs::sanitize_path(output_file.as_ref())?;

            // create subdirs if they don't exist
            let mut output_vfs_path = path::PathBuf::from("/");
            output_vfs_path.push(output_file.as_path());

            if let Some(p) = output_vfs_path.parent() {
                VFS.lock().unwrap().create_dir_all(p)?;
            }

            // copy files from JS_FS to MemFS
            let output_vfs_path_str: String = output_vfs_path.as_path().to_string_lossy().into();
            self.engine.evaluate_script(&format!(
                "
                try {{
                    writeFile('{}', FS.readFile('{}'));
                }}
                catch(error) {{
                    throw new Error(\"Error writing to file '{}': \" + error);
                }}",
                output_vfs_path_str, output_vfs_path_str, output_vfs_path_str
            ))?;

            // create files on the host
            let mut output_hostfs_path = path::PathBuf::from(output_path.as_ref());

            if let Some(p) = output_file.parent() {
                log::debug!("Creating subdirs={:?}", output_hostfs_path.join(p));
                hostfs::create_dir_all(output_hostfs_path.join(p))?;
            }

            output_hostfs_path.push(output_file.as_path());

            log::info!(
                "Saving output at {}",
                output_hostfs_path.as_path().to_string_lossy()
            );

            let contents = VFS.lock().unwrap().read_file(output_vfs_path)?;
            hostfs::write_file(output_hostfs_path, &contents)?;
        }

        Ok(())
    }

    fn mount_vol(
        &mut self,
        path: impl Into<String>,
        mode: NodeMode,
        v: impl VfsVolume + 'static + Send + Sync,
    ) -> io::Result<()> {
        let path = path.into();
        sp_wasm_hostfs::VfsManager::with(move |vfs| vfs.bind(path, mode, v))
    }

    pub fn mount(
        &mut self,
        src: impl Into<PathBuf>,
        des: &str,
        mode: NodeMode,
    ) -> std::io::Result<()> {
        let path = src.into();
        if path.is_file() {
            self.mount_vol(des, NodeMode::Ro, sp_wasm_hostfs::zipfs::volume(path)?)
        } else {
            self.mount_vol(des, mode, sp_wasm_hostfs::dirfs::volume(path)?)
        }
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }
}
