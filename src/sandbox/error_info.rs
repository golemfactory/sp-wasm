use mozjs::jsapi::JSContext;
use mozjs::jsapi::JS_ClearPendingException;
use mozjs::jsapi::JS_EncodeStringToUTF8;
use mozjs::jsapi::JS_IsExceptionPending;
use mozjs::jsval::UndefinedValue;
use mozjs::rust::wrappers::{JS_ErrorFromException, JS_GetPendingException};
use mozjs::rust::HandleObject;

use std::slice::from_raw_parts;

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

pub unsafe extern "C" fn report_pending_exception(ctx: *mut JSContext, dispatch_event: bool) {
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
