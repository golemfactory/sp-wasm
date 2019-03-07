mod error_info;
mod vfs;

use error_info::*;

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
use mozjs::typedarray::{CreateWith, Uint8Array};

use std::ffi::CStr;
use std::ptr;
use std::str;

use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    static ref VFS: Mutex<vfs::VirtualFS> = Mutex::new(vfs::VirtualFS::new());
}

pub struct Sandbox {
    runtime: Runtime,
    global: *mut JSObject,
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

        self.evaluate_script(&js);
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
        self.evaluate_script(&js);
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

            self.evaluate_script(&format!(
                "writeFile('{}', FS.readFile('{}'));",
                output_path, output_file
            ));
        }
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
                b"writeFile\0".as_ptr() as *const libc::c_char,
                Some(Self::write_file),
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

        let vfs = VFS.lock().unwrap();
        let contents = vfs.read_file(filename).unwrap();

        rooted!(in(ctx) let mut rval = ptr::null_mut::<JSObject>());
        Uint8Array::create(ctx, CreateWith::Slice(&contents), rval.handle_mut()).unwrap();

        args.rval().set(ObjectValue(rval.get()));
        true
    }

    unsafe extern "C" fn write_file(ctx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
        let args = CallArgs::from_vp(vp, argc);

        let arg = mozjs::rust::Handle::from_raw(args.get(0));
        let filename = mozjs::rust::ToString(ctx, arg);

        typedarray!(in(ctx) let contents: Uint8Array = args.get(1).to_object());
        let contents: Vec<u8> = contents.unwrap().to_vec();

        rooted!(in(ctx) let filename_root = filename);
        let filename = JS_EncodeStringToUTF8(ctx, filename_root.handle().into());
        let filename = CStr::from_ptr(filename);
        let filename = str::from_utf8(filename.to_bytes()).unwrap();
        vfs::write_file(filename, &contents).unwrap();

        args.rval().set(UndefinedValue());
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

        sandbox
    }
}
