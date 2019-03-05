mod error_info;
mod file_manip;

use error_info::*;
use file_manip::*;

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

use std::ffi::CStr;
use std::ptr;
use std::str;

use lazy_static::lazy_static;
use std::sync::Mutex;

const SANDBOX_PATH: &str = "sandbox";
const SANDBOX_PWD: &str = "$andb0x_g0l3m_w@sm";

lazy_static! {
    static ref VFS: Mutex<zbox::Repo> = Mutex::new(
        zbox::RepoOpener::new()
            .create_new(true)
            .open(&format!("file://{}", SANDBOX_PATH), SANDBOX_PWD)
            .unwrap_or_else(|err| {
                panic!(
                    "Failed to create new VFS at {} with error: {}",
                    format!("file://{}", SANDBOX_PATH),
                    err,
                )
            })
    );
}

pub struct Sandbox {
    runtime: Runtime,
    global: *mut JSObject,
}

impl Sandbox {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn map_input_path(&mut self, input_path: &str) {
        log::info!("Mapping input directories at {}", input_path);

        let mut vfs = VFS.lock().unwrap();

        map_path(&mut vfs, input_path).unwrap_or_else(|err| {
            panic!(
                "Failed to map {} into VirtualFS with error {}",
                input_path, err
            )
        });

        let mut js = "
        Module['preRun'] = function() {
        "
        .to_string();

        let mut path = std::path::PathBuf::from("/");
        path.push(std::path::Path::new(input_path).file_name().unwrap());

        js += &format!("FS.mkdir('{}');", path.to_str().unwrap());

        visit(&vfs, path, &mut |entry| {
            if vfs.is_dir(entry.path()) {
                js += &format!("\n\tFS.mkdir('{}');", entry.path().to_str().unwrap());
            } else {
                let s = entry.path().to_str().unwrap();
                js += &format!(
                    "\n\tFS.writeFile('{}', new Uint8Array(readFile('{}')));",
                    s, s
                );
            }
        })
        .unwrap();

        js += "\n};";
        println!("{}", js);
        self.evaluate_script(&js);
    }

    pub fn map_output_path(&mut self, output_path: &str) {
        log::info!("Mapping output directories at {}", output_path);

        map_path(&mut VFS.lock().unwrap(), output_path).unwrap_or_else(|err| {
            panic!(
                "Failed to map {} into VirtualFS with error {}",
                output_path, err
            )
        })
    }

    pub fn run(&self, wasm_js: &str, wasm_bin: &str) {
        let mut js = format!("Module['wasmBinary'] = readFileFS('{}');", wasm_bin);
        let wasm_js = read_file_fs(wasm_js).unwrap_or_else(|err| {
            panic!(
                "Failed to read JavaScript file {} with error {}",
                wasm_js, err
            )
        });
        let wasm_js = String::from_utf8(wasm_js)
            .unwrap_or_else(|err| panic!("Failed to parse JavaScript with error {}", err));
        js += &wasm_js;
        self.evaluate_script(&js);
    }

    fn init(&self) {
        let ctx = self.runtime.cx();

        unsafe {
            // runtime options
            let ctx_opts = &mut *ContextOptionsRef(ctx);
            ctx_opts.set_wasm_(true);
            ctx_opts.set_wasmBaseline_(true);
            ctx_opts.set_wasmIon_(true);
            SetBuildIdOp(ctx, Some(Self::sp_build_id));

            // callbacks
            rooted!(in(ctx) let global_root = self.global);
            let gl = global_root.handle();
            let _ac = JSAutoCompartment::new(ctx, gl.get());

            JS_DefineFunction(
                ctx,
                gl.into(),
                b"print\0".as_ptr() as *const libc::c_char,
                Some(Self::print),
                0,
                0,
            );

            JS_DefineFunction(
                ctx,
                gl.into(),
                b"readFile\0".as_ptr() as *const libc::c_char,
                Some(Self::read_file),
                0,
                0,
            );

            JS_DefineFunction(
                ctx,
                gl.into(),
                b"readFileFS\0".as_ptr() as *const libc::c_char,
                Some(Self::read_file_fs),
                0,
                0,
            );
        }

        // init print funcs
        self.evaluate_script("var Module = {'printErr': print, 'print': print};");
    }

    pub fn evaluate_script(&self, script: &str) {
        let ctx = self.runtime.cx();

        rooted!(in(ctx) let mut rval = UndefinedValue());
        rooted!(in(ctx) let global = self.global);

        self.runtime
            .evaluate_script(global.handle(), script, "noname", 0, rval.handle_mut())
            .unwrap_or_else(|_| unsafe {
                report_pending_exception(ctx, true);
            });
    }

    unsafe extern "C" fn sp_build_id(build_id: *mut BuildIdCharVector) -> bool {
        let sp_id = b"SP\0";
        SetBuildId(build_id, &sp_id[0], sp_id.len())
    }

    unsafe extern "C" fn read_file(ctx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
        let args = CallArgs::from_vp(vp, argc);

        let arg = mozjs::rust::Handle::from_raw(args.get(0));
        let filename = mozjs::rust::ToString(ctx, arg);

        rooted!(in(ctx) let filename_root = filename);
        let filename = JS_EncodeStringToUTF8(ctx, filename_root.handle().into());
        let filename = CStr::from_ptr(filename);
        let filename = str::from_utf8(filename.to_bytes()).unwrap();
        let contents = read_file(&mut VFS.lock().unwrap(), filename).unwrap();

        rooted!(in(ctx) let mut rval = ptr::null_mut::<JSObject>());
        ArrayBuffer::create(ctx, CreateWith::Slice(&contents), rval.handle_mut()).unwrap();

        args.rval().set(ObjectValue(rval.get()));
        true
    }

    unsafe extern "C" fn read_file_fs(ctx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
        let args = CallArgs::from_vp(vp, argc);

        let arg = mozjs::rust::Handle::from_raw(args.get(0));
        let filename = mozjs::rust::ToString(ctx, arg);

        rooted!(in(ctx) let filename_root = filename);
        let filename = JS_EncodeStringToUTF8(ctx, filename_root.handle().into());
        let filename = CStr::from_ptr(filename);
        let filename = str::from_utf8(filename.to_bytes()).unwrap();
        let contents = read_file_fs(filename).unwrap();

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
}

// impl Drop for Sandbox {
//     fn drop(&mut self) {
//         // remove repo if was created successfully
//         if let Ok(res) = zbox::Repo::exists(&format!("file://{}", Self::SANDBOX_PATH)) {
//             if res {
//                 std::fs::remove_dir_all(Self::SANDBOX_PATH)
//                     .unwrap_or_else(|err| log::error!("Sandbox VirtualFS didn't exist: {:?}", err));

//                 log::debug!("Sandbox VirtualFS removed");
//             }
//         }
//     }
// }

impl Default for Sandbox {
    fn default() -> Self {
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

        let sandbox = Self { runtime, global };
        sandbox.init();
        zbox::init_env();

        sandbox
    }
}
