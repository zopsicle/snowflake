//! Backend for running compiled code.
//!
//! The current backend translates bytecode to JavaScript and runs it on V8.
//! This use of JavaScript and V8 is purely an implementation detail.
//! A future version will replace this with a custom virtual machine,
//! completely phasing out the use of JavaScript and the dependency on V8.
//! UNDER NO CIRCUMSTANCES MUST JAVASCRIPT BE EXPOSED IN THE PUBLIC API.

use std::{lazy::SyncOnceCell, panic::{RefUnwindSafe, UnwindSafe}, ptr::NonNull};

pub mod lower;

extern "C"
{
    type SekkaBackend;
    fn sekka_backend_init();
    fn sekka_backend_new() -> *mut SekkaBackend;
    fn sekka_backend_drop(backend: *mut SekkaBackend);
    fn sekka_backend_run_js(
        backend: *mut SekkaBackend,
        js_ptr: *const libc::c_char,
        js_len: libc::size_t,
    ) -> bool;
}

pub struct Backend
{
    raw: NonNull<SekkaBackend>,
}

unsafe impl Send for Backend { }

impl RefUnwindSafe for Backend { }
impl UnwindSafe for Backend { }

impl Backend
{
    pub fn new() -> Self
    {
        static INIT: SyncOnceCell<()> = SyncOnceCell::new();
        INIT.get_or_init(|| unsafe { sekka_backend_init() });

        let raw = unsafe { sekka_backend_new() };
        let raw = NonNull::new(raw).expect("Cannot create backend");

        let this = Self{raw};

        static RUNTIME_JS: &str = include_str!("runtime.js");
        let status = this.run_js(RUNTIME_JS);
        assert!(status, "Cannot initialize runtime");

        this
    }

    pub fn run_js(&self, js: &str) -> bool
    {
        unsafe {
            sekka_backend_run_js(
                /* backend */ self.raw.as_ptr(),
                /* js_ptr  */ js.as_ptr().cast(),
                /* js_len  */ js.len(),
            )
        }
    }
}

impl Drop for Backend
{
    fn drop(&mut self)
    {
        unsafe { sekka_backend_drop(self.raw.as_ptr()); }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn example()
    {
        let _backend = Backend::new();
        let _backend = Backend::new();
        let _backend = Backend::new();
        let _backend = Backend::new();
    }
}
