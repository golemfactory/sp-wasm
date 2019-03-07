use super::vfs;
use super::VFS;

use mozjs::glue::SetBuildId;
use mozjs::jsapi::BuildIdCharVector;
use mozjs::jsapi::CallArgs;
use mozjs::jsapi::CompartmentOptions;
use mozjs::jsapi::ContextOptionsRef;
use mozjs::jsapi::JSAutoCompartment;
use mozjs::jsapi::JSContext;
use mozjs::jsapi::JSObject;
use mozjs::jsapi::JS_ClearPendingException;
use mozjs::jsapi::JS_DefineFunction;
use mozjs::jsapi::JS_EncodeStringToUTF8;
use mozjs::jsapi::JS_IsExceptionPending;
use mozjs::jsapi::JS_NewGlobalObject;
use mozjs::jsapi::OnNewGlobalHookOption;
use mozjs::jsapi::SetBuildIdOp;
use mozjs::jsapi::Value;
use mozjs::jsval::ObjectValue;
use mozjs::jsval::UndefinedValue;
use mozjs::rust::wrappers::{JS_ErrorFromException, JS_GetPendingException};
use mozjs::rust::HandleObject;
use mozjs::rust::{JSEngine, Runtime, SIMPLE_GLOBAL_CLASS};
use mozjs::typedarray::{CreateWith, Uint8Array};

use std::slice::from_raw_parts;

use std::ffi::CStr;
use std::ptr;
use std::str;

pub struct Engine {
    runtime: Runtime,
    global: *mut JSObject,
}

impl Engine {
    pub fn new() -> Self {
        let engine =
            JSEngine::init().unwrap_or_else(|err| panic!("Error initializing JSEngine: {:?}", err));
        let runtime = Runtime::new(engine);

        unsafe { Self::create_with(runtime) }
    }

    unsafe fn create_with(runtime: Runtime) -> Self {
        let h_option = OnNewGlobalHookOption::FireOnNewGlobalHook;
        let c_option = CompartmentOptions::default();
        let ctx = runtime.cx();

        let global = JS_NewGlobalObject(
            ctx,
            &SIMPLE_GLOBAL_CLASS,
            ptr::null_mut(),
            h_option,
            &c_option,
        );

        // runtime options
        let ctx_opts = &mut *ContextOptionsRef(ctx);
        ctx_opts.set_wasm_(true);
        ctx_opts.set_wasmBaseline_(true);
        ctx_opts.set_wasmIon_(true);
        SetBuildIdOp(ctx, Some(Self::sp_build_id));

        // callbacks
        rooted!(in(ctx) let global_root = global);
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

        // init print funcs
        Self::eval(
            &runtime,
            global,
            "var Module = {'printErr': print, 'print': print};",
        );

        Self { runtime, global }
    }

    unsafe fn eval<S>(runtime: &Runtime, global: *mut JSObject, script: S)
    where
        S: AsRef<str>,
    {
        let ctx = runtime.cx();

        rooted!(in(ctx) let global_root = global);
        let global = global_root.handle();
        let _ac = JSAutoCompartment::new(ctx, global.get());

        rooted!(in(ctx) let mut rval = UndefinedValue());

        runtime
            .evaluate_script(global, script.as_ref(), "noname", 0, rval.handle_mut())
            .unwrap_or_else(|_| report_pending_exception(ctx, true));
    }

    pub fn evaluate_script<S>(&self, script: S)
    where
        S: AsRef<str>,
    {
        unsafe { Self::eval(&self.runtime, self.global, script) }
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

struct ErrorInfo {
    pub message: String,
    pub filename: String,
    pub lineno: libc::c_uint,
    pub column: libc::c_uint,
}

impl ErrorInfo {
    unsafe fn from_native_error(cx: *mut JSContext, object: HandleObject) -> Option<ErrorInfo> {
        let report = JS_ErrorFromException(cx, object);
        if report.is_null() {
            return None;
        }

        let filename = {
            let filename = (*report)._base.filename as *const u8;
            if !filename.is_null() {
                let length = (0..).find(|idx| *filename.offset(*idx) == 0).unwrap();
                let filename = from_raw_parts(filename, length as usize);
                String::from_utf8_lossy(filename).into_owned()
            } else {
                "none".to_string()
            }
        };

        let lineno = (*report)._base.lineno;
        let column = (*report)._base.column;

        let message = {
            let message = (*report)._base.message_.data_ as *const u8;
            let length = (0..).find(|idx| *message.offset(*idx) == 0).unwrap();
            let message = from_raw_parts(message, length as usize);
            String::from_utf8_lossy(message).into_owned()
        };

        Some(ErrorInfo {
            filename,
            message,
            lineno,
            column,
        })
    }
}

pub unsafe extern "C" fn report_pending_exception(ctx: *mut JSContext, _dispatch_event: bool) {
    if !JS_IsExceptionPending(ctx) {
        return;
    }

    rooted!(in(ctx) let mut value = UndefinedValue());

    if !JS_GetPendingException(ctx, value.handle_mut()) {
        JS_ClearPendingException(ctx);
        panic!("Uncaught exception: JS_GetPendingException failed");
    }

    JS_ClearPendingException(ctx);

    if value.is_object() {
        rooted!(in(ctx) let object = value.to_object());
        let error_info =
            ErrorInfo::from_native_error(ctx, object.handle()).unwrap_or_else(|| ErrorInfo {
                message: "uncaught exception: unknown (can't convert to string)".to_string(),
                filename: String::new(),
                lineno: 0,
                column: 0,
            });

        eprintln!(
            "Error at {}:{}:{} {}",
            error_info.filename, error_info.lineno, error_info.column, error_info.message
        );
    } else if value.is_string() {
        rooted!(in(ctx) let object = value.to_string());
        let message = JS_EncodeStringToUTF8(ctx, object.handle().into());
        let message = std::ffi::CStr::from_ptr(message);

        eprintln!(
            "Error: {}",
            String::from_utf8_lossy(message.to_bytes()).into_owned()
        );
    } else {
        panic!("Uncaught exception: failed to stringify primitive");
    };
}
