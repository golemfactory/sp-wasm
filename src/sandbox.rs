mod error_info;
mod virtual_fs;

use error_info::report_pending_exception;
use virtual_fs::VirtualFS;

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
use std::fs::File;
use std::io::prelude::*;
use std::ptr;
use std::str;

pub struct Sandbox {
    runtime: Runtime,
    global: *mut JSObject,
}

impl Sandbox {
    pub fn new(input_dir: &str, output_dir: &str) -> Self {
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

        let sandbox = Sandbox { runtime, global };
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

    pub fn map_dir(&mut self, input_dir: &str) {}

    pub fn add_output_file(&mut self, output_file: &str) {}

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
