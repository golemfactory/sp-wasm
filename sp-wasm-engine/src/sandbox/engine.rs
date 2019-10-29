use super::VFS;
use crate::Result;
use mozjs::glue::SetBuildId;
use mozjs::jsapi::BuildIdCharVector;
use mozjs::jsapi::CallArgs;
use mozjs::jsapi::CompartmentOptions;
use mozjs::jsapi::ContextOptionsRef;
use mozjs::jsapi::JSAutoCompartment;
use mozjs::jsapi::JSContext;
use mozjs::jsapi::JSObject;
use mozjs::jsapi::JSString;
use mozjs::jsapi::JS_DefineFunction;
use mozjs::jsapi::JS_EncodeStringToUTF8;
use mozjs::jsapi::JS_NewGlobalObject;
use mozjs::jsapi::JS_ReportErrorASCII;
use mozjs::jsapi::OnNewGlobalHookOption;
use mozjs::jsapi::SetBuildIdOp;
use mozjs::jsapi::Value;
use mozjs::jsval::ObjectValue;
use mozjs::jsval::UndefinedValue;
use mozjs::rust::{Handle, JSEngine, Runtime, ToString, ToUint64, SIMPLE_GLOBAL_CLASS};
use mozjs::typedarray::{ArrayBuffer, CreateWith};
use std::ptr;

pub struct Engine {
    runtime: Runtime,
    global: *mut JSObject,
}

impl Engine {
    pub fn new() -> Result<Self> {
        log::info!("Initializing SpiderMonkey engine");
        let engine = JSEngine::init().map_err(error::Error::from)?;
        let runtime = Runtime::new(engine);

        unsafe {
            let engine = Self::create_with(runtime)?;
            Ok(engine)
        }
    }

    unsafe fn create_with(runtime: Runtime) -> Result<Self> {
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
            b"usleep\0".as_ptr() as *const libc::c_char,
            Some(Self::usleep),
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
            "var Module = {
                'printErr': print,
                'print': print,
            };",
        )?;

        // init /dev/random emulation
        Self::eval(
            &runtime,
            global,
            "var golem_MAGIC = 0;
            golem_randEmu = function() {
                golem_MAGIC = Math.pow(golem_MAGIC + 1.8912, 3) % 1;
                return golem_MAGIC;
            };
            var crypto = {
                getRandomValues: function(array) {
                    for (var i = 0; i < array.length; i++)
                        array[i] = (golem_randEmu() * 256) | 0
                }
            };",
        )?;

        // make time ops fully deterministic
        Self::eval(
            &runtime,
            global,
            "
              var date = new Date(0);
              var timestamp = date.getTime();
              Date.now = function() { return timestamp; }
             ",
        )?;

        Ok(Self { runtime, global })
    }

    unsafe fn eval<S>(runtime: &Runtime, global: *mut JSObject, script: S) -> Result<Value>
    where
        S: AsRef<str>,
    {
        let ctx = runtime.cx();

        rooted!(in(ctx) let global_root = global);
        let global = global_root.handle();
        let _ac = JSAutoCompartment::new(ctx, global.get());

        rooted!(in(ctx) let mut rval = UndefinedValue());

        if runtime
            .evaluate_script(global, script.as_ref(), "noname", 0, rval.handle_mut())
            .is_err()
        {
            return Err(error::Error::SMJS(error::JSError::new(ctx)).into());
        }

        Ok(rval.get())
    }

    pub fn evaluate_script<S>(&self, script: S) -> Result<Value>
    where
        S: AsRef<str>,
    {
        log::debug!("Evaluating script {}", script.as_ref());
        unsafe { Self::eval(&self.runtime, self.global, script) }
    }

    unsafe extern "C" fn sp_build_id(build_id: *mut BuildIdCharVector) -> bool {
        let sp_id = b"SP\0";
        SetBuildId(build_id, &sp_id[0], sp_id.len())
    }

    unsafe extern "C" fn read_file(ctx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
        let args = CallArgs::from_vp(vp, argc);

        if args.argc_ != 1 {
            JS_ReportErrorASCII(
                ctx,
                b"readFile(filename) requires exactly 1 argument\0".as_ptr() as *const libc::c_char,
            );
            return false;
        }

        let arg = Handle::from_raw(args.get(0));
        let filename = js_string_to_utf8(ctx, ToString(ctx, arg));

        if let Err(err) = (|| -> Result<()> {
            let contents = VFS.lock().unwrap().read_file(&filename)?;

            rooted!(in(ctx) let mut rval = ptr::null_mut::<JSObject>());
            ArrayBuffer::create(ctx, CreateWith::Slice(&contents), rval.handle_mut())
                .map_err(|_| error::Error::SliceToUint8ArrayConversion)?;

            args.rval().set(ObjectValue(rval.get()));
            Ok(())
        })() {
            JS_ReportErrorASCII(
                ctx,
                format!("failed to read file '{}' with error: {}\0", &filename, err)
                    .as_bytes()
                    .as_ptr() as *const libc::c_char,
            );
            return false;
        }

        true
    }

    unsafe extern "C" fn write_file(ctx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
        let args = CallArgs::from_vp(vp, argc);

        if args.argc_ != 2 {
            JS_ReportErrorASCII(
                ctx,
                b"writeFile(filename, data) requires exactly 2 arguments\0".as_ptr()
                    as *const libc::c_char,
            );
            return false;
        }

        let arg = Handle::from_raw(args.get(0));
        let filename = js_string_to_utf8(ctx, ToString(ctx, arg));

        if let Err(err) = (|| -> Result<()> {
            typedarray!(in(ctx) let contents: ArrayBufferView = args.get(1).to_object());
            let contents: Vec<u8> = contents
                .map_err(|_| error::Error::Uint8ArrayToVecConversion)?
                .to_vec();

            VFS.lock().unwrap().write_file(&filename, &contents)?;

            Ok(())
        })() {
            JS_ReportErrorASCII(
                ctx,
                format!("failed to write file '{}' with error: {}\0", &filename, err)
                    .as_bytes()
                    .as_ptr() as *const libc::c_char,
            );
            return false;
        }

        args.rval().set(UndefinedValue());
        true
    }

    unsafe extern "C" fn print(ctx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
        let args = CallArgs::from_vp(vp, argc);

        if args.argc_ > 1 {
            JS_ReportErrorASCII(
                ctx,
                b"print(msg=\"\") requires 0 or 1 arguments\0".as_ptr() as *const libc::c_char,
            );
            return false;
        }

        let message = if args.argc_ == 0 {
            "".to_string()
        } else {
            let arg = Handle::from_raw(args.get(0));
            js_string_to_utf8(ctx, ToString(ctx, arg))
        };

        println!("{}", message);

        args.rval().set(UndefinedValue());
        true
    }

    unsafe extern "C" fn usleep(ctx: *mut JSContext, argc: u32, vp: *mut Value) -> bool {
        use std::{thread::sleep, time::Duration};

        let args = CallArgs::from_vp(vp, argc);

        if args.argc_ != 1 {
            JS_ReportErrorASCII(
                ctx,
                b"usleep(useconds) requires exactly 1 argument\0".as_ptr() as *const libc::c_char,
            );
            return false;
        }

        let arg = Handle::from_raw(args.get(0));
        let useconds = match ToUint64(ctx, arg) {
            Ok(useconds) => useconds,
            Err(()) => {
                JS_ReportErrorASCII(
                    ctx,
                    b"couldn't extract value from input arg 'useconds'\0".as_ptr()
                        as *const libc::c_char,
                );
                return false;
            }
        };

        // sleep
        sleep(Duration::from_micros(useconds));

        args.rval().set(UndefinedValue());
        true
    }
}

unsafe fn js_string_to_utf8(ctx: *mut JSContext, js_string: *mut JSString) -> String {
    rooted!(in(ctx) let string_root = js_string);
    let string = JS_EncodeStringToUTF8(ctx, string_root.handle().into());
    let string = std::ffi::CStr::from_ptr(string);
    String::from_utf8_lossy(string.to_bytes()).into_owned()
}

pub mod error {
    use super::js_string_to_utf8;
    use super::JSContext;
    use super::UndefinedValue;
    use mozjs::jsapi::JS_ClearPendingException;
    use mozjs::jsapi::JS_IsExceptionPending;
    use mozjs::rust::jsapi_wrapped::JS_Stringify;
    use mozjs::rust::wrappers::{JS_ErrorFromException, JS_GetPendingException};
    use mozjs::rust::HandleObject;
    use mozjs::rust::HandleValue;
    use mozjs::rust::JSEngineError;
    use std::slice;

    #[derive(Debug, Fail)]
    pub enum Error {
        #[fail(display = "couldn't convert &[u8] to Uint8Array")]
        SliceToUint8ArrayConversion,

        #[fail(display = "couldn't convert Uint8Array to Vec<u8>")]
        Uint8ArrayToVecConversion,

        #[fail(display = "SpiderMonkey internal error")]
        SMInternal,

        #[fail(display = "{}", _0)]
        SMJS(#[cause] JSError),
    }

    impl PartialEq for Error {
        fn eq(&self, other: &Error) -> bool {
            match (self, other) {
                (&Error::SliceToUint8ArrayConversion, &Error::SliceToUint8ArrayConversion) => true,
                (&Error::Uint8ArrayToVecConversion, &Error::Uint8ArrayToVecConversion) => true,
                (&Error::SMInternal, &Error::SMInternal) => true,
                (&Error::SMJS(ref left), &Error::SMJS(ref right)) => left == right,
                (_, _) => false,
            }
        }
    }

    impl From<JSEngineError> for Error {
        fn from(_err: JSEngineError) -> Self {
            Error::SMInternal
        }
    }

    impl From<JSError> for Error {
        fn from(err: JSError) -> Self {
            Error::SMJS(err)
        }
    }

    #[derive(Debug, PartialEq, Fail)]
    #[fail(display = "JavaScript error: {}", message)]
    pub struct JSError {
        pub message: String,
    }

    impl JSError {
        const MAX_JSON_STRINGIFY: usize = 1024;

        pub unsafe fn new(ctx: *mut JSContext) -> Self {
            Self::create_with(ctx)
        }

        unsafe fn create_with(ctx: *mut JSContext) -> Self {
            if !JS_IsExceptionPending(ctx) {
                return Self {
                    message: "Uncaught exception: exception reported but not pending".to_string(),
                };
            }

            rooted!(in(ctx) let mut value = UndefinedValue());

            if !JS_GetPendingException(ctx, value.handle_mut()) {
                JS_ClearPendingException(ctx);
                return Self {
                    message: "Uncaught exception: JS_GetPendingException failed".to_string(),
                };
            }

            JS_ClearPendingException(ctx);

            if value.is_object() {
                rooted!(in(ctx) let object = value.to_object());
                Self::from_native_error(ctx, object.handle()).unwrap_or_else(|| {
                    // try serializing to JSON
                    let mut data = vec![0; Self::MAX_JSON_STRINGIFY];
                    if !JS_Stringify(
                        ctx,
                        &mut value.handle_mut(),
                        HandleObject::null(),
                        HandleValue::null(),
                        Some(Self::stringify_cb),
                        data.as_mut_ptr() as *mut libc::c_void,
                    ) {
                        return Self {
                            message: "Uncaught exception: unknown (can't convert to string)"
                                .to_string(),
                        };
                    }

                    if let Ok(data) = std::ffi::CString::from_vec_unchecked(data).into_string() {
                        Self { message: data }
                    } else {
                        Self {
                            message: "Uncaught exception: unknown (can't convert to string)"
                                .to_string(),
                        }
                    }
                })
            } else if value.is_string() {
                let message = js_string_to_utf8(ctx, value.to_string());
                Self { message }
            } else {
                Self {
                    message: "Uncaught exception: failed to stringify primitive".to_string(),
                }
            }
        }

        unsafe fn from_native_error(ctx: *mut JSContext, obj: HandleObject) -> Option<Self> {
            let report = JS_ErrorFromException(ctx, obj);
            if report.is_null() {
                return None;
            }

            let message = {
                let message = (*report)._base.message_.data_ as *const u8;
                let length = (0..).find(|idx| *message.offset(*idx) == 0).unwrap();
                let message = slice::from_raw_parts(message, length as usize);
                String::from_utf8_lossy(message).into_owned()
            };

            Some(Self { message })
        }

        unsafe extern "C" fn stringify_cb(
            bytes: *const u16,
            len: u32,
            data: *mut libc::c_void,
        ) -> bool {
            let data = std::slice::from_raw_parts_mut(data as *mut u8, Self::MAX_JSON_STRINGIFY);
            let bytes = std::slice::from_raw_parts(bytes, len as usize);
            for i in 0..len as usize {
                // TODO substitute UTF16 chars with unknown symbol in UTF8
                data[i] = bytes[i].to_le_bytes()[0];
            }
            true
        }
    }
}
