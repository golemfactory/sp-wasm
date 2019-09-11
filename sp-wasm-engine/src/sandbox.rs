pub mod engine;
pub mod load;
pub mod vfs;

use self::engine::*;
use self::vfs::*;
use super::Result;
use std::sync::Mutex;
use std::{io, path};

pub use self::engine::EngineRef;
use crate::sandbox::load::Bytes;
use failure::_core::convert::TryInto;
use itertools::Itertools;
use lazy_static::lazy_static;
use sp_wasm_hostfs::vfsdo::NodeMode;
use sp_wasm_hostfs::vfsops::VfsVolume;
use std::fs::OpenOptions;
use std::io::Read;
use std::path::{Path, PathBuf};

lazy_static! {
    static ref VFS: Mutex<VirtualFS> = Mutex::new(VirtualFS::default());
}

pub struct Sandbox {
    engine: Engine,
}

impl Sandbox {
    pub fn init_ejs() -> Result<EngineRef> {
        Engine::init()
    }

    pub fn new_on_engine(engine: EngineRef) -> Result<Self> {
        let engine = Engine::new_on_engine(engine)?;

        Ok(Self { engine })
    }

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

    pub fn work_dir(self, dir: &str) -> Result<Self> {
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

    pub fn run<JsSrc: TryInto<Bytes>, WasmSrc: TryInto<Bytes>>(
        self,
        wasm_js: JsSrc,
        wasm_bin: WasmSrc,
    ) -> Result<Self>
    where
        JsSrc::Error: Into<crate::error::Error>,
        WasmSrc::Error: Into<crate::error::Error>,
    {
        let js_vec = wasm_js.try_into().map_err(Into::into)?;
        let wasm_js = std::str::from_utf8(js_vec.as_slice())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let mut wasm_vec = wasm_bin.try_into().map_err(Into::into)?;

        VFS.lock()
            .unwrap()
            .write_file("/main.wasm", wasm_vec.as_slice())?;

        let mut js = "Module['wasmBinary'] = readFile('/main.wasm');".to_string();
        js += &wasm_js;
        self.engine.evaluate_script(&js)?;

        Ok(self)
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
