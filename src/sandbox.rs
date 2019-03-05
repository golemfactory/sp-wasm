mod error_info;

use error_info::report_pending_exception;

use mozjs::glue::SetBuildId;
use mozjs::jsapi::BuildIdCharVector;
use mozjs::jsapi::CallArgs;
use mozjs::jsapi::CompartmentOptions;
use mozjs::jsapi::ContextOptionsRef;
use mozjs::jsapi::JSAutoCompartment;
use mozjs::jsapi::JSContext;
use mozjs::jsapi::JSObject;
use mozjs::jsapi::JS_DefineFunction;
use mozjs::jsapi::JS_EncodeStringToUTF8;
use mozjs::jsapi::JS_NewGlobalObject;
use mozjs::jsapi::OnNewGlobalHookOption;
use mozjs::jsapi::SetBuildIdOp;
use mozjs::jsapi::Value;
use mozjs::jsval::ObjectValue;
use mozjs::jsval::UndefinedValue;
use mozjs::rust::{JSEngine, Runtime, SIMPLE_GLOBAL_CLASS};
use mozjs::typedarray::{ArrayBuffer, CreateWith};

use std::collections::{HashMap, VecDeque};
use std::ffi::CStr;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::ptr;
use std::str;

pub struct Sandbox {
    runtime: Runtime,
    global: *mut JSObject,
    vfs: zbox::Repo,
}

impl Sandbox {
    const SANDBOX_PATH: &'static str = "sandbox";
    const SANDBOX_PWD: &'static str = "$andb0x_g0l3m_w@sm";

    pub fn new() -> Self {
        let engine =
            JSEngine::init().unwrap_or_else(|err| panic!("Error initializing JSEngine: {:?}", err));
        let runtime = Runtime::new(engine);
        let ctx = runtime.cx();
        let h_option = OnNewGlobalHookOption::FireOnNewGlobalHook;
        let c_option = CompartmentOptions::default();

        let global = unsafe {
            JS_NewGlobalObject(
                ctx,
                &SIMPLE_GLOBAL_CLASS,
                ptr::null_mut(),
                h_option,
                &c_option,
            )
        };

        zbox::init_env();
        let vfs = zbox::RepoOpener::new()
            .create_new(true)
            .open(&format!("file://{}", Self::SANDBOX_PATH), Self::SANDBOX_PWD)
            .unwrap_or_else(|err| {
                panic!(
                    "Error initializing VirtualFS: repo already exists: {:?}",
                    err
                )
            });

        let sandbox = Sandbox {
            runtime,
            global,
            vfs,
        };
        sandbox.init();

        sandbox
    }

    fn init(&self) {
        let ctx = self.runtime.cx();

        unsafe {
            // runtime options
            let ctx_opts = &mut *ContextOptionsRef(ctx);
            ctx_opts.set_wasm_(true);
            ctx_opts.set_wasmBaseline_(true);
            ctx_opts.set_wasmIon_(true);
            SetBuildIdOp(ctx, Some(sp_build_id));

            // callbacks
            rooted!(in(ctx) let global_root = self.global);
            let gl = global_root.handle();
            let _ac = JSAutoCompartment::new(ctx, gl.get());

            JS_DefineFunction(
                ctx,
                gl.into(),
                b"print\0".as_ptr() as *const libc::c_char,
                Some(print),
                0,
                0,
            );
        }

        // init print funcs
        self.evaluate_script("var Module = {'printErr': print, 'print': print};");
    }

    pub fn map_input_dir(&mut self, input_dir: &str) {
        let input_path = Path::new(input_dir);
        let rel_input_path = input_path
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .unwrap_or_else(|| panic!("invalid filename!"));
        let mut rel_path = PathBuf::new();
        rel_path.push("/");
        rel_path.push(rel_input_path);

        let mut abs_path = PathBuf::new();
        abs_path.push(input_path);
        let path = PathCombo(abs_path.into_boxed_path(), rel_path.into_boxed_path());

        visit(path, &mut |path: &PathCombo| {
            println!("{:?} => {:?}", path.0, path.1);

            if path.0.is_dir() {
                self.vfs.create_dir(&path.1).unwrap();
            } else {
                let mut file = self.vfs.create_file(&path.1).unwrap();
                // write contents over
                let contents = read_file(&path.0).unwrap();
                file.write_once(&contents).unwrap();
            }
        })
        .unwrap_or_else(|err| {
            panic!("couldn't map input files into MemFS: {:?}", err);
        });

        rvisit(&self.vfs, Path::new("/"), &|entry| {
            let path = entry.path();
            println!("{:?}", path);
        })
        .unwrap();
    }

    pub fn map_output_dir(&mut self, output_dir: &str, output_file: &str) {}

    pub fn run(&self, wasm_js: &str, wasm_bin: &str) {
        self.evaluate_script("print(1);")
    }

    fn evaluate_script(&self, script: &str) {
        let ctx = self.runtime.cx();

        rooted!(in(ctx) let mut rval = UndefinedValue());
        rooted!(in(ctx) let global = self.global);

        self.runtime
            .evaluate_script(global.handle(), script, "noname", 0, rval.handle_mut())
            .unwrap_or_else(|_| unsafe {
                report_pending_exception(ctx, true);
            });
    }
}

struct PathCombo(Box<Path>, Box<Path>);

fn rvisit(vfs: &zbox::Repo, path: &Path, cb: &Fn(&zbox::DirEntry)) -> zbox::Result<()> {
    println!("{:?}", path);

    if vfs.is_dir(path) {
        for entry in vfs.read_dir(path)? {
            let path = entry.path();

            if vfs.is_dir(path) {
                rvisit(vfs, &path, cb)?;
            } else {
                cb(&entry);
            }
        }
    }

    Ok(())
}

fn visit(path: PathCombo, cb: &mut FnMut(&PathCombo)) -> std::io::Result<()> {
    let mut fifo = VecDeque::new();
    fifo.push_back(path);

    while let Some(path) = fifo.pop_front() {
        cb(&path);
        let PathCombo(abs_path, rel_path) = path;

        if abs_path.is_dir() {
            for entry in fs::read_dir(abs_path)? {
                let entry = entry?;
                let abs_path = entry.path();

                let rel_input_path = abs_path
                    .file_name()
                    .and_then(std::ffi::OsStr::to_str)
                    .unwrap_or_else(|| panic!("invalid filename!"));
                let mut rel_path = rel_path.clone().into_path_buf();
                rel_path.push(rel_input_path);
                let path = PathCombo(abs_path.into_boxed_path(), rel_path.into_boxed_path());
                fifo.push_back(path);
            }
        }
    }

    Ok(())
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        // remove repo if was created successfully
        if let Ok(res) = zbox::Repo::exists(&format!("file://{}", Self::SANDBOX_PATH)) {
            if res {
                std::fs::remove_dir_all(Self::SANDBOX_PATH)
                    .unwrap_or_else(|err| log::error!("Sandbox VirtualFS didn't exist: {:?}", err));
            }
        }
    }
}

fn read_file<P: AsRef<Path>>(path: P) -> std::io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;

    Ok(contents)
}

unsafe extern "C" fn sp_build_id(build_id: *mut BuildIdCharVector) -> bool {
    let sp_id = b"SP\0";
    SetBuildId(build_id, &sp_id[0], sp_id.len())
}

unsafe extern "C" fn readWasm(ctx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
    let args = CallArgs::from_vp(vp, argc);

    let arg = mozjs::rust::Handle::from_raw(args.get(0));
    let filename = mozjs::rust::ToString(ctx, arg);

    rooted!(in(ctx) let filename_root = filename);
    let filename = JS_EncodeStringToUTF8(ctx, filename_root.handle().into());
    let filename = CStr::from_ptr(filename);

    let mut file = File::open(str::from_utf8(filename.to_bytes()).unwrap()).unwrap();
    let mut contents = Vec::new();
    file.read_to_end(&mut contents);

    rooted!(in(ctx) let mut rval = ptr::null_mut::<JSObject>());
    ArrayBuffer::create(ctx, CreateWith::Slice(&contents), rval.handle_mut()).unwrap();

    args.rval().set(ObjectValue(rval.get()));
    true
}

unsafe extern "C" fn print(ctx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
    let args = CallArgs::from_vp(vp, argc);

    let arg = mozjs::rust::Handle::from_raw(args.get(0));
    let js = mozjs::rust::ToString(ctx, arg);

    rooted!(in(ctx) let message_root = js);
    let message = JS_EncodeStringToUTF8(ctx, message_root.handle().into());
    let message = CStr::from_ptr(message);

    println!("{}", str::from_utf8(message.to_bytes()).unwrap());

    args.rval().set(UndefinedValue());
    true
}
