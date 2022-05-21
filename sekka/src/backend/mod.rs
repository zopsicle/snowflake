use std::ptr::NonNull;

extern "C"
{
    type SekkaBackend;
    fn sekka_backend_new(
        runtime_js_ptr: *const libc::c_char,
        runtime_js_len: libc::size_t,
    ) -> *mut SekkaBackend;
    fn sekka_backend_drop(this: *mut SekkaBackend);
}

pub struct Backend
{
    raw: NonNull<SekkaBackend>,
}

impl Backend
{
    pub fn new() -> Self
    {
        static RUNTIME_JS: &str = include_str!("runtime.js");
        let raw = unsafe {
            sekka_backend_new(
                /* runtime_js_ptr */ RUNTIME_JS.as_ptr().cast(),
                /* runtime_js_len */ RUNTIME_JS.len(),
            )
        };
        let raw = NonNull::new(raw).unwrap();
        Self{raw}
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
        let backend = Backend::new();
    }
}
