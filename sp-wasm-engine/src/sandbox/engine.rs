use super::VFS;
use crate::Result;
use mozjs::{
    glue::SetBuildId,
    jsapi::{
        BuildIdCharVector, CallArgs, CompartmentOptions, ContextOptionsRef, InitSelfHostedCode,
        JSAutoCompartment, JSContext, JSGCParamKey, JSObject, JSString, JS_BeginRequest,
        JS_DefineFunction, JS_DestroyContext, JS_EncodeStringToUTF8, JS_EndRequest, JS_NewContext,
        JS_NewGlobalObject, JS_ReportErrorASCII, JS_SetGCParameter, JS_SetNativeStackQuota,
        OnNewGlobalHookOption, RunJobs, SetBuildIdOp, UseInternalJobQueues, Value, JS,
    },
    jsval::{ObjectValue, UndefinedValue},
    panic::maybe_resume_unwind,
    rust::{
        CompileOptionsWrapper, Handle, HandleObject, JSEngine, MutableHandleValue, ToString,
        ToUint64, SIMPLE_GLOBAL_CLASS,
    },
    typedarray::{ArrayBuffer, CreateWith},
};
use std::{
    ffi,
    os::raw::c_uint,
    ptr::{self, NonNull},
    sync::Arc,
};

const STACK_QUOTA: usize = 128 * 8 * 1024;
const SYSTEM_CODE_BUFFER: usize = 10 * 1024;
const TRUSTED_SCRIPT_BUFFER: usize = 8 * 12800;

unsafe fn new_root_context() -> Result<NonNull<JSContext>> {
    let ctx = match NonNull::new(JS_NewContext(
        32_u32 * 1024_u32 * 1024_u32,
        1 << 20 as u32,
        ptr::null_mut(),
    )) {
        Some(ctx) => ctx,
        None => return Err(error::Error::SMNullPtr.into()),
    };
    let ctx_ptr = ctx.as_ptr();

    JS_SetGCParameter(ctx_ptr, JSGCParamKey::JSGC_MAX_BYTES, std::u32::MAX);
    JS_SetNativeStackQuota(
        ctx_ptr,
        STACK_QUOTA,
        STACK_QUOTA - SYSTEM_CODE_BUFFER,
        STACK_QUOTA - SYSTEM_CODE_BUFFER - TRUSTED_SCRIPT_BUFFER,
    );
    UseInternalJobQueues(ctx_ptr, false);
    InitSelfHostedCode(ctx_ptr);
    let contextopts = ContextOptionsRef(ctx_ptr);
    (*contextopts).set_baseline_(true);
    (*contextopts).set_ion_(true);
    (*contextopts).set_nativeRegExp_(true);
    (*contextopts).set_wasm_(true);
    (*contextopts).set_wasmBaseline_(true);
    (*contextopts).set_wasmIon_(true);
    JS_BeginRequest(ctx_ptr);

    Ok(ctx)
}

pub fn evaluate_script(
    ctx: NonNull<JSContext>,
    glob: HandleObject,
    script: &str,
    filename: &str,
    line_num: u32,
    rval: MutableHandleValue,
) -> Option<()> {
    let script_utf16: Vec<u16> = script.encode_utf16().collect();
    let filename_cstr = ffi::CString::new(filename.as_bytes()).unwrap();
    log::debug!(
        "Evaluating script from {} with content {}",
        filename,
        script
    );
    // SpiderMonkey does not approve of null pointers.
    let (ptr, len) = if script_utf16.len() == 0 {
        static EMPTY: &'static [u16] = &[];
        (EMPTY.as_ptr(), 0)
    } else {
        (script_utf16.as_ptr(), script_utf16.len() as c_uint)
    };
    assert!(!ptr.is_null());

    let ctx = ctx.as_ptr();
    let _ac = JSAutoCompartment::new(ctx, glob.get());
    let options = CompileOptionsWrapper::new(ctx, filename_cstr.as_ptr(), line_num);

    unsafe {
        if !JS::Evaluate2(
            ctx,
            options.ptr,
            ptr as *const u16,
            len as libc::size_t,
            rval.into(),
        ) {
            log::debug!("...err!");
            maybe_resume_unwind();
            None
        } else {
            // we could return the script result but then we'd have
            // to root it and so forth and, really, who cares?
            log::debug!("...ok!");
            Some(())
        }
    }
}

pub struct Engine {
    ctx: NonNull<JSContext>,
    global: NonNull<JSObject>,
}

impl Drop for Engine {
    fn drop(&mut self) {
        let ctx = self.ctx.as_ptr();
        unsafe {
            JS_EndRequest(ctx);
            JS_DestroyContext(ctx);
        }
    }
}

impl Engine {
    pub fn new(_js_engine: Arc<JSEngine>) -> Result<Self> {
        log::info!("Creating new Engine instance");
        unsafe {
            let ctx = new_root_context()?;
            let engine = Self::create_with(ctx)?;
            Ok(engine)
        }
    }

    unsafe fn create_with(ctx: NonNull<JSContext>) -> Result<Self> {
        let h_option = OnNewGlobalHookOption::FireOnNewGlobalHook;
        let c_option = CompartmentOptions::default();
        let ctx_ptr = ctx.as_ptr();

        let global = match NonNull::new(JS_NewGlobalObject(
            ctx_ptr,
            &SIMPLE_GLOBAL_CLASS,
            ptr::null_mut(),
            h_option,
            &c_option,
        )) {
            Some(global) => global,
            None => return Err(error::Error::SMNullPtr.into()),
        };

        SetBuildIdOp(ctx_ptr, Some(Self::sp_build_id));

        // callbacks
        let global_ptr = global.as_ptr();
        rooted!(in(ctx_ptr) let global_root = global_ptr);
        let gl = global_root.handle();
        let _ac = JSAutoCompartment::new(ctx_ptr, gl.get());

        JS_DefineFunction(
            ctx_ptr,
            gl.into(),
            b"print\0".as_ptr() as *const libc::c_char,
            Some(Self::print),
            0,
            0,
        );

        JS_DefineFunction(
            ctx_ptr,
            gl.into(),
            b"usleep\0".as_ptr() as *const libc::c_char,
            Some(Self::usleep),
            0,
            0,
        );

        JS_DefineFunction(
            ctx_ptr,
            gl.into(),
            b"readFile\0".as_ptr() as *const libc::c_char,
            Some(Self::read_file),
            0,
            0,
        );

        JS_DefineFunction(
            ctx_ptr,
            gl.into(),
            b"writeFile\0".as_ptr() as *const libc::c_char,
            Some(Self::write_file),
            0,
            0,
        );

        // init print funcs
        Self::eval(
            ctx,
            global,
            "var Module = {
                'printErr': print,
                'print': print,
            };",
        )?;

        // init /dev/random emulation
        Self::eval(
            ctx,
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
            ctx,
            global,
            "
              var date = new Date(0);
              var timestamp = date.getTime();
              Date.now = function() { return timestamp; }
             ",
        )?;

        Ok(Self { ctx, global })
    }

    unsafe fn eval<S>(
        ctx: NonNull<JSContext>,
        global: NonNull<JSObject>,
        script: S,
    ) -> Result<Value>
    where
        S: AsRef<str>,
    {
        let ctx_ptr = ctx.as_ptr();
        let global = global.as_ptr();

        rooted!(in(ctx_ptr) let global_root = global);
        let global = global_root.handle();
        let _ac = JSAutoCompartment::new(ctx_ptr, global.get());

        rooted!(in(ctx_ptr) let mut rval = UndefinedValue());

        if let None = evaluate_script(ctx, global, script.as_ref(), "noname", 0, rval.handle_mut())
        {
            return Err(error::Error::SMJS(error::JSError::new(ctx_ptr)).into());
        }

        RunJobs(ctx_ptr);

        Ok(rval.get())
    }

    pub fn evaluate_script<S>(&self, script: S) -> Result<Value>
    where
        S: AsRef<str>,
    {
        log::debug!("Evaluating script {}", script.as_ref());
        unsafe { Self::eval(self.ctx, self.global, script) }
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

    #[derive(Debug, thiserror::Error, PartialEq)]
    pub enum Error {
        #[error("couldn't convert &[u8] to Uint8Array")]
        SliceToUint8ArrayConversion,
        #[error("couldn't convert Uint8Array to Vec<u8>")]
        Uint8ArrayToVecConversion,
        #[error("SpiderMonkey internal error")]
        SMInternal,
        #[error("Null pointer exception")]
        SMNullPtr,
        #[error("{0}")]
        SMJS(#[from] JSError),
    }

    impl From<JSEngineError> for Error {
        fn from(_err: JSEngineError) -> Self {
            Error::SMInternal
        }
    }

    #[derive(Debug, PartialEq, thiserror::Error)]
    #[error("JavaScript error: {message}")]
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
