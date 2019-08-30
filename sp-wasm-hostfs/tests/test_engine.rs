use mozjs::rust::{Runtime, JSEngine, SIMPLE_GLOBAL_CLASS};
use std::ptr;
use mozjs::{rooted, jsapi};
use mozjs::jsval::UndefinedValue;
use std::ffi::{CStr};
use mozjs::rust::jsapi_wrapped as js;
use sp_wasm_hostfs::vfsdo::{VolumeInfo, NodeMode};
use mozjs::conversions::ToJSValConvertible;
use sp_wasm_hostfs::{build_js_api, VfsManager, dirfs};

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
            let _ = js::JS_DefineFunction(cx, env.handle(), b"puts\0".as_ptr() as *const _,
                                             Some(puts), 1, 0);

            rooted!(in(cx) let mut envv = mozjs::jsval::ObjectValue(env.get()));

            js::JS_SetProperty(cx, global, b"env\0".as_ptr() as *const _, envv.handle());

            rooted!(in(cx) let mut hostfs_api = mozjs::jsval::UndefinedValue());
            build_js_api(cx, hostfs_api.handle_mut());
            js::JS_SetProperty(cx, global, b"hostfs\0".as_ptr() as *const _, hostfs_api.handle());


            rooted!(in(cx) let mut vi = mozjs::jsval::UndefinedValue());
            VolumeInfo {
                id: 0,
                mount_point: "/tmp".to_string(),
                mode: NodeMode::Ro
            }.to_jsval(cx, vi.handle_mut());
            js::JS_SetProperty(cx, global, b"vi\0".as_ptr() as *const _, vi.handle());
        }

        VfsManager::with(|manager| {
            manager.bind("/in", NodeMode::Ro, dirfs::volume("/home/prekucki/workspace/wasm/test-ls")?)?;
            manager.bind("/out", NodeMode::Rw, dirfs::volume("/tmp")?)
        }).unwrap();

        let javascript = r#"

            env.puts(`id=${vi.id}, mount_point=${vi.mount_point}, mode=${vi.mode}`);

            vi.mode = 100;
            vi.tag = 'tag';
            env.puts(JSON.stringify(vi));
            let vix = Object.assign({}, vi);
            vix.mode = 100;
            vix.tag = 'tag';
            env.puts(JSON.stringify(vix));


            function vo(k, v) {
                env.puts(`${k} = ${JSON.stringify(v)}`);
            }

            env.puts(`hostfs=${JSON.stringify(hostfs.volumes())}`);
            env.puts(`dir=${JSON.stringify(hostfs.readdir(0, ''))}`);
            vo('test', hostfs.lookup(0, 'test.c'));

            try {
            let f1 = hostfs.open(0, 'test.c', 'ro', false);
            vo('f1', f1);
            let f2 = hostfs.open(0, 'test.c', 'rw');
            vo('f2', f2);
            hostfs.close(f1);
            let f3 = hostfs.open(0, 'test.c', 'ro', false);
            let f_out = hostfs.open(1, 'test.c', 'rw', true);
            vo('f3', f3);
            let buf = new Uint8Array(1024);
            let len = hostfs.read(f3, buf, 0, 10, 0);
            vo('len', len);
            vo('buf', buf);
            hostfs.write(f_out, buf, 0, len, 0);
            hostfs.close(f_out);
            }
            catch(e) {
                vo('err', e);
            }


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
        jsapi::JS_ReportErrorASCII(context, b"puts() requires exactly 1 argument\0".as_ptr() as *const _);
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