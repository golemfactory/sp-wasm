use std::cell::Cell;

use sp_wasm::prelude::*;

// We need this since SpiderMonkey JSEngine
// can only every be initialized once per
// execution context!
thread_local!(static SANDBOX: Cell<*Sandbox> = Cell::new(std::ptr::null()));

pub fn sandbox() -> &'static Sandbox {
    SANDBOX.with(|sandbox| { sandbox.get() })
}
