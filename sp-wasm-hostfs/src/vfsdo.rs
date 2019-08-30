use mozjs::{
    conversions::{ConversionResult, FromJSValConvertible, ToJSValConvertible},
    jsapi as js, jsval, rooted,
    rust::wrappers as jsw,
    rust::{HandleValue, MutableHandleValue},
};

#[derive(Clone, Copy)]
pub enum NodeType {
    Dir,
    File,
}

#[derive(Clone, Copy)]
pub enum NodeMode {
    Ro,
    Rw,
    Wo,
}

impl std::ops::BitAnd for NodeMode {
    type Output = Option<NodeMode>;

    fn bitand(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (NodeMode::Rw, _) => Some(NodeMode::Rw),
            (_, NodeMode::Rw) => Some(NodeMode::Rw),
            (NodeMode::Ro, NodeMode::Ro) => Some(NodeMode::Ro),
            (NodeMode::Wo, NodeMode::Wo) => Some(NodeMode::Wo),
            (_, _) => None,
        }
    }
}

impl ToJSValConvertible for NodeMode {
    unsafe fn to_jsval(&self, cx: *mut js::JSContext, rval: MutableHandleValue) {
        match self {
            NodeMode::Ro => "ro",
            NodeMode::Rw => "rw",
            NodeMode::Wo => "wo",
        }
        .to_jsval(cx, rval);
    }
}

impl FromJSValConvertible for NodeMode {
    type Config = ();

    unsafe fn from_jsval(
        cx: *mut js::JSContext,
        val: HandleValue,
        _: Self::Config,
    ) -> Result<ConversionResult<Self>, ()> {
        let s = match String::from_jsval(cx, val, ())? {
            ConversionResult::Failure(f) => return Ok(ConversionResult::Failure(f)),
            ConversionResult::Success(v) => v,
        };
        Ok(match s.as_ref() {
            "ro" => ConversionResult::Success(NodeMode::Ro),
            "rw" => ConversionResult::Success(NodeMode::Rw),
            "wo" => ConversionResult::Success(NodeMode::Wo),
            _ => ConversionResult::Failure(format!("invalid mode: {}", s).into()),
        })
    }
}

impl ToJSValConvertible for NodeType {
    unsafe fn to_jsval(&self, cx: *mut js::JSContext, rval: MutableHandleValue) {
        match self {
            NodeType::Dir => "d",
            NodeType::File => "f",
        }
        .to_jsval(cx, rval)
    }
}

pub struct VolumeInfo {
    pub id: u32,
    pub mount_point: String,
    pub mode: NodeMode,
}

impl ToJSValConvertible for VolumeInfo {
    unsafe fn to_jsval(&self, cx: *mut js::JSContext, mut rval: MutableHandleValue) {
        rooted!(in(cx) let mut obj = js::JS_NewPlainObject(cx));
        rooted!(in(cx) let id = jsval::UInt32Value(self.id));
        rooted!(in(cx) let mut mount_point = jsval::UndefinedValue());
        rooted!(in(cx) let mut mode = jsval::UndefinedValue());
        self.mount_point.to_jsval(cx, mount_point.handle_mut());
        self.mode.to_jsval(cx, mode.handle_mut());
        jsw::JS_SetProperty(cx, obj.handle(), b"id\0".as_ptr() as *const _, id.handle());
        jsw::JS_SetProperty(
            cx,
            obj.handle(),
            b"mount_point\0".as_ptr() as *const _,
            mount_point.handle(),
        );
        jsw::JS_SetProperty(
            cx,
            obj.handle(),
            b"mode\0".as_ptr() as *const _,
            mode.handle(),
        );
        jsw::JS_FreezeObject(cx, obj.handle());
        rval.set(jsval::ObjectValue(obj.get()))
    }
}

pub struct NodeInfo {
    pub n_type: NodeType,
    pub n_mode: NodeMode,
}

impl ToJSValConvertible for NodeInfo {
    unsafe fn to_jsval(&self, cx: *mut js::JSContext, mut rval: MutableHandleValue) {
        rooted!(in(cx) let mut obj = js::JS_NewPlainObject(cx));
        {
            rooted!(in(cx) let mut n_type = jsval::UndefinedValue());
            self.n_type.to_jsval(cx, n_type.handle_mut());
            jsw::JS_SetProperty(
                cx,
                obj.handle(),
                b"type\0".as_ptr() as *const _,
                n_type.handle(),
            );
        }
        {
            rooted!(in(cx) let mut n_mode = jsval::UndefinedValue());
            self.n_mode.to_jsval(cx, n_mode.handle_mut());
            jsw::JS_SetProperty(
                cx,
                obj.handle(),
                b"mode\0".as_ptr() as *const _,
                n_mode.handle(),
            );
        }
        jsw::JS_FreezeObject(cx, obj.handle());
        rval.set(jsval::ObjectValue(obj.get()))
    }
}
