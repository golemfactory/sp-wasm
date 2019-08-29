use std::io;
use mozjs::rooted;
use mozjs::conversions::{ToJSValConvertible, FromJSValConvertible, ConversionBehavior};
use mozjs::rust::{MutableHandleValue, HandleValue};
use mozjs::rust::wrappers as jsw;
use mozjs::jsapi as js;
use mozjs::jsval;
use crate::vfsdo::{VolumeInfo, NodeInfo, NodeMode};
use crate::vfsops::{INode, Stream, VfsVolume};
use lazy_static::lazy_static;
use std::sync::RwLock;

pub mod vfsops;
pub mod vfsdo;
pub mod dirfs;

type Fd = Box<dyn vfsops::Stream + 'static + Send + Sync>;
type ResolverDyn = Box<dyn VfsResolver + 'static + Send + Sync>;

trait VfsResolver {

    fn info(&self, id : u32) -> VolumeInfo;

    fn lookup(&self, path : &str) -> io::Result<Option<NodeInfo>>;

    fn open(&self, path : &str, mode : NodeMode, create_new :bool) -> io::Result<Box<dyn vfsops::Stream + Send + Sync>>;

    fn readdir(&self, path : &str) -> io::Result<Vec<String>>;
}

pub struct VfsManager {
    fds : [Option<Fd>; 64],
    volumes : Vec<ResolverDyn>
}

struct Resolver<T : vfsops::VfsVolume> {
    volume : T,
    mount_point : String,
    mode : vfsdo::NodeMode,
}

impl<T : vfsops::VfsVolume + 'static> VfsResolver for Resolver<T> {
    fn info(&self, id: u32) -> VolumeInfo {
        VolumeInfo {
            id,
            mount_point: self.mount_point.clone(),
            mode: self.mode
        }
    }

    fn lookup(&self, path: &str) -> io::Result<Option<NodeInfo>> {
        let inode = self.volume.lookup(path)?;

        Ok(inode.as_ref().map( vfsops::INode::mode).map(|(n_type, n_mode)| NodeInfo {
            n_type, n_mode
        }))
    }

    fn open(&self, path: &str, mode : NodeMode, create_new : bool) -> io::Result<Box<vfsops::Stream + Send + Sync>> {
        self.volume.lookup(path)?
            .ok_or_else(|| io::ErrorKind::NotFound.into())
            .and_then(|ino| ino.open(mode, create_new))
            .map(|s| -> Box<dyn vfsops::Stream + 'static + Send + Sync> {
                Box::new(s)
            })
    }

    fn readdir(&self, path : &str) -> io::Result<Vec<String>> {
        self.volume.lookup(path)?
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?
            .read_dir()
    }

}



impl VfsManager {

    pub fn new() -> Self {
        VfsManager {
            fds: [
                None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None,

                None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None,
            ],
            volumes: Vec::new()
        }
    }

    pub fn volumes(&self) -> Vec<VolumeInfo> {
        self.volumes.iter()
            .enumerate()
            .map(|(idx, v)| v.info(idx as u32))
            .collect()
    }

    pub fn lookup(&self, vol_id : usize, path : &str) -> io::Result<Option<NodeInfo>> {
        self.volumes
            .get(vol_id)
            .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?
            .lookup(path)
    }

    pub fn readdir(&self, vol_id : usize, path : &str) -> io::Result<Vec<String>> {
        self.volumes
            .get(vol_id)
            .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?
            .readdir(path)
    }


    pub fn open(&mut self, vol_id : usize, path : &str, mode : NodeMode, create_new : bool) -> io::Result<u32> {
        let (idx, f) = self.fds.iter_mut().enumerate()
            .find(|(_, fd)| fd.is_none())
            .ok_or_else(|| io::Error::from_raw_os_error(24 /*Too many open files*/))?;

        let stream = self.volumes
            .get(vol_id)
            .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?
            .open(path, mode, create_new)?;

        *f = Some(stream);
        Ok(idx as u32)
    }

    pub fn close(&mut self, fd : u32) -> io::Result<()> {
        if let Some(fd) = self.fds.get_mut(fd as usize).ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?.take() {
            fd.close()
        }
        else {
            Err(io::Error::from(io::ErrorKind::InvalidInput))
        }
    }
    // read(fd, buf, offset, len, position) -> int
    pub fn read(&mut self, fd : u32, buf : &mut [u8], position : u64) -> io::Result<u64> {
        let v = self.fds.get_mut(fd as usize)
            .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
        match v {
            Some(f) => f.read(buf, position),
            None => Err(io::Error::from(io::ErrorKind::InvalidInput))
        }
    }

    pub fn write(&mut self, fd : u32, buf : &[u8], position : u64) -> io::Result<u64> {
        let v = self.fds.get_mut(fd as usize)
            .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
        match v {
            Some(f) => f.write(buf, position),
            None => Err(io::Error::from(io::ErrorKind::InvalidInput))
        }
    }

    pub fn bind(&mut self, path : impl Into<String>, mode : NodeMode, v : impl VfsVolume + 'static + Send + Sync) -> io::Result<()> {
        let resolver = Box::new(Resolver {
            volume: v,
            mount_point: path.into(),
            mode
        });
        self.volumes.push(resolver);
        Ok(())
    }

    pub fn with<F, T>(action : F) -> T where F : FnOnce(&mut Self) -> T {
        let mut r = VFS.write().unwrap();
        action(std::ops::DerefMut::deref_mut(&mut r))
    }
}


lazy_static! {
    static ref VFS: RwLock<VfsManager> = RwLock::new(VfsManager::new());
}


mod js_hostfs {
    use super::*;
    use mozjs::conversions::{ToJSValConvertible, FromJSValConvertible, ConversionBehavior, ConversionResult};
    use std::ffi::CString;
    use mozjs::rust::HandleObject;

    macro_rules! fromjs {
        {
            in($cx:expr) $(let $v:ident : $t:ty = $args:ident[$idx:expr] & $b:expr;)+
        } => {
            $(
                let h = HandleValue::from_raw($args.get($idx));
                let $v : $t = match <$t>::from_jsval($cx,h, $b) {
                    Ok(ConversionResult::Success(v)) => v,
                    Ok(ConversionResult::Failure(_err)) => {
                         js::JS_ReportErrorASCII($cx, b"conversion error\0".as_ptr() as *const _);
                         return false;
                    }
                    Err(()) => return false
                };
            )+
        };
    }

    macro_rules! retjs {
        (in($cx:expr) $args:ident[rval] = $v:expr) => {
            return {
                let rval = MutableHandleValue::from_raw($args.rval());
                $v.to_jsval($cx, rval);
                true
            }
        };
    }

    macro_rules! try_js {
        (in($cx:expr) $e:expr) => {
            match $e {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("e={}", e);
                    let msg = CString::new(format!("{}", e)).unwrap();
                    js::JS_ReportErrorASCII($cx, msg.as_ptr() as *const _);
                    return false;
                }
            }
        };
    }

    pub(super) unsafe extern "C" fn volumes(cx: *mut js::JSContext, argc: u32, vp: *mut js::Value) -> bool {
        let args = js::CallArgs::from_vp(vp, argc);

        retjs! {
            in(cx) args[rval] = VFS.read().unwrap().volumes()
        }
    }

    pub(super) unsafe extern "C" fn readdir(cx: *mut js::JSContext, argc: u32, vp: *mut js::Value) -> bool {
        let args = js::CallArgs::from_vp(vp, argc);
        if args.argc_ != 2 {
            js::JS_ReportErrorASCII(
                cx,
                b"readdir(vol_id, path) requires exactly 2 arguments\0".as_ptr()
                    as *const _,
            );
            return false;
        }

        fromjs! {
            in(cx)
            let vol_id : u32 = args[0] & ConversionBehavior::EnforceRange;
            let path : String = args[1] & ();
        }

        retjs! {
            in(cx) args[rval] = try_js!(in(cx) VFS.read().unwrap().readdir(vol_id as usize, path.as_ref()))
        }
    }

    pub(super) unsafe extern "C" fn lookup(cx: *mut js::JSContext, argc: u32, vp: *mut js::Value) -> bool {
        let args = js::CallArgs::from_vp(vp, argc);
        fromjs! {
            in(cx)
            let vol_id : u32 = args[0] & ConversionBehavior::EnforceRange;
            let path : String = args[1] & ();
        }
        let node = try_js!(in(cx) VFS.read().unwrap().lookup(vol_id as usize, &path));
        retjs! {
            in(cx) args[rval] = node
        }
    }

    //open(vol_id, path, mode, create_new) -> int
    pub(super) unsafe extern "C" fn open(cx: *mut js::JSContext, argc: u32, vp: *mut js::Value) -> bool {
        let args = js::CallArgs::from_vp(vp, argc);
        fromjs! {
            in(cx)
            let vol_id : u32 = args[0] & ConversionBehavior::EnforceRange;
            let path : String = args[1] & ();
            let mode : NodeMode = args[2] & ();
            let create_new : bool = args[3] & ();
        }
        let fd = try_js!(in(cx)VFS.write().unwrap().open(vol_id as usize, &path, mode, create_new));
        retjs! {
            in(cx) args[rval] = fd
        }
    }

    //pub fn close(&mut self, fd : u32) -> io::Result<()> {
    pub(super) unsafe extern "C" fn close(cx: *mut js::JSContext, argc: u32, vp: *mut js::Value) -> bool {
        let args = js::CallArgs::from_vp(vp, argc);
        fromjs! { in(cx) let fd : u32 = args[0] & ConversionBehavior::EnforceRange; }
        try_js!(in(cx)VFS.write().unwrap().close(fd));

        true
    }

    // read(fd, buf, offset, len, position) -> int
    pub(super) unsafe extern "C" fn read(cx: *mut js::JSContext, argc: u32, vp: *mut js::Value) -> bool {
        let args = js::CallArgs::from_vp(vp, argc);
        fromjs! {
            in(cx)
            let fd : u32 = args[0] & ConversionBehavior::EnforceRange;
            let offset : u32 = args[2] & ConversionBehavior::EnforceRange;
            let len : u32 = args[3] & ConversionBehavior::EnforceRange;
            let position : u64 = args[2] & ConversionBehavior::EnforceRange;
        }
        let buf_handle = HandleValue::from_raw(args.get(1));
        if !buf_handle.get().is_object() {
            // TODO: Trow err
            return false;
        }

        let obj = buf_handle.get().to_object();
        use mozjs::typedarray::ArrayBuffer;

        // TODO: throw err
        let mut t =  mozjs::typedarray::TypedArray::<mozjs::typedarray::Uint8, *mut js::JSObject>::from(obj).unwrap();

        let buf_slice = t.as_mut_slice();

        let to = (offset+len) as usize;
        let from  = offset as usize;
        let ret = try_js!(in(cx) VFS.write().unwrap().read(fd, &mut buf_slice[from..to], position));
        retjs! {
            in(cx) args[rval] = ret
        }
    }

    pub(super) unsafe extern "C" fn write(cx: *mut js::JSContext, argc: u32, vp: *mut js::Value) -> bool {
        let args = js::CallArgs::from_vp(vp, argc);
        fromjs! {
            in(cx)
            let fd : u32 = args[0] & ConversionBehavior::EnforceRange;
            let offset : u32 = args[2] & ConversionBehavior::EnforceRange;
            let len : u32 = args[3] & ConversionBehavior::EnforceRange;
            let position : u64 = args[2] & ConversionBehavior::EnforceRange;
        }
        let buf_handle = HandleValue::from_raw(args.get(1));
        if !buf_handle.get().is_object() {
            // TODO: Trow err
            return false;
        }

        let obj = buf_handle.get().to_object();
        use mozjs::typedarray::ArrayBuffer;

        // TODO: throw err
        let mut t =  mozjs::typedarray::TypedArray::<mozjs::typedarray::Uint8, *mut js::JSObject>::from(obj).unwrap();

        let buf_slice = t.as_slice();

        let to = (offset+len) as usize;
        let from  = offset as usize;
        let ret = try_js!(in(cx) VFS.write().unwrap().write(fd, &buf_slice[from..to], position));
        retjs! {
            in(cx) args[rval] = ret
        }
    }


}



pub unsafe fn build_js_api(cx : *mut js::JSContext, mut rval : MutableHandleValue) -> bool {
    rooted!(in(cx) let hostfs_api = js::JS_NewPlainObject(cx));
    let _ = jsw::JS_DefineFunction(cx, hostfs_api.handle(), b"volumes\0".as_ptr() as *const _,
                                         Some(js_hostfs::volumes), 0, 0);
    let _ = jsw::JS_DefineFunction(cx, hostfs_api.handle(), b"readdir\0".as_ptr() as *const _,
                                   Some(js_hostfs::readdir), 2, 0);
    let _ = jsw::JS_DefineFunction(cx, hostfs_api.handle(), b"lookup\0".as_ptr() as *const _,
                                   Some(js_hostfs::lookup), 2, 0);
    let _ = jsw::JS_DefineFunction(cx, hostfs_api.handle(), b"open\0".as_ptr() as *const _,
                                   Some(js_hostfs::open), 4, 0);
    let _ = jsw::JS_DefineFunction(cx, hostfs_api.handle(), b"close\0".as_ptr() as *const _,
                                   Some(js_hostfs::close), 1, 0);
    let _ = jsw::JS_DefineFunction(cx, hostfs_api.handle(), b"read\0".as_ptr() as *const _,
                                   Some(js_hostfs::read), 5, 0);
    let _ = jsw::JS_DefineFunction(cx, hostfs_api.handle(), b"write\0".as_ptr() as *const _,
                                   Some(js_hostfs::write), 5, 0);


    rval.set(jsval::ObjectValue(hostfs_api.get()));
    true
}