use mozjs::rust::{Runtime, JSEngine, SIMPLE_GLOBAL_CLASS};
use std::ptr;
use mozjs::{rooted, jsapi};
use mozjs::jsval::UndefinedValue;
use mozjs::rust::wrappers::*;
use std::ffi::{CStr};
use libc::c_char;
use mozjs::rust::jsapi_wrapped as js;
use sp_wasm_hostfs::vfsdo::{VolumeInfo, NodeMode};
use mozjs::conversions::ToJSValConvertible;

#[test]
fn test_engine() {
    let engine = JSEngine::init().unwrap();
    let runtime = Runtime::new(engine);
    let cx = runtime.cx();
    let h_option = jsapi::OnNewGlobalHookOption::FireOnNewGlobalHook;
    let c_option = jsapi::CompartmentOptions::default();
    unsafe {
        let global = jsapi::JS_NewGlobalObject(cx, &SIMPLE_GLOBAL_CLASS, ptr::null_mut(), h_option, &c_option);
        rooted!(in(cx) let global_root = global);
        let global = global_root.handle();
        let _ac = jsapi::JSAutoCompartment::new(cx, global.get());
        {
            rooted!(in(cx) let mut env = jsapi::JS_NewPlainObject(cx));
            let function = js::JS_DefineFunction(cx, env.handle(), b"puts\0".as_ptr() as *const c_char,
                                             Some(puts), 1, 0);

            rooted!(in(cx) let mut envv = mozjs::jsval::ObjectValue(env.get()));

            js::JS_SetProperty(cx, global, b"env\0".as_ptr() as *const c_char, envv.handle());
            rooted!(in(cx) let mut vi = mozjs::jsval::UndefinedValue());
            VolumeInfo {
                id: 0,
                mount_point: "/tmp".to_string(),
                mode: NodeMode::Ro
            }.to_jsval(cx, vi.handle_mut());
            js::JS_SetProperty(cx, global, b"vi\0".as_ptr() as *const c_char, vi.handle());
        }

        let javascript = r#"

            env.puts(`id=${vi.id}, mount_point=${vi.mount_point}, mode=${vi.mode}`);

            vi.mode = 100;
            vi.tag = 'tag';
            env.puts(JSON.stringify(vi));
            let vix = Object.assign({}, vi);
            vix.mode = 100;
            vix.tag = 'tag';
            env.puts(JSON.stringify(vix));


            this.tag = 'ala';
            env.puts('Test Iñtërnâtiônàlizætiøn ┬─┬ノ( º _ ºノ) ');
            if (env) {
                env.tag = 'ala';
                env.puts(JSON.stringify(env));
            }
            else {
                env.puts('no env');
            }
        "#;
        rooted!(in(cx) let mut rval = UndefinedValue());
        let _ = runtime.evaluate_script(global, javascript, "test.js", 0, rval.handle_mut()).unwrap();

    }

}

unsafe extern "C" fn puts(context: *mut jsapi::JSContext, argc: u32, vp: *mut jsapi::Value) -> bool {
    let args = jsapi::CallArgs::from_vp(vp, argc);

    if args.argc_ != 1 {
        jsapi::JS_ReportErrorASCII(context, b"puts() requires exactly 1 argument\0".as_ptr() as *const c_char);
        return false;
    }
    let arg = mozjs::rust::Handle::from_raw(args.get(0));
    rooted!(in(context) let message_root = mozjs::rust::ToString(context, arg));
    let message = mozjs::rust::wrappers::JS_EncodeStringToUTF8(context, message_root.handle());
    let message = CStr::from_ptr(message);
    println!("{}", std::str::from_utf8(message.to_bytes()).unwrap());

    args.rval().set(UndefinedValue());
    return true;
}