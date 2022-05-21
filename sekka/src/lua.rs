use {lua_sys::*, std::{error, fmt, ffi::CStr, ptr::NonNull, slice}};

pub type Result<T> =
    std::result::Result<T, Error>;

pub struct Error
{
    status: libc::c_int,
    error: Box<[u8]>,
}

impl fmt::Debug for Error
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        f.debug_struct("Error")
            .field("status", &self.status)
            .field("error", &self.error.escape_ascii().to_string())
            .finish()
    }
}

impl fmt::Display for Error
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        fmt::Display::fmt(&self.error.escape_ascii(), f)
    }
}

impl error::Error for Error
{
}

pub struct State
{
    raw: NonNull<lua_State>,
}

impl State
{
    pub fn newstate() -> Result<Self>
    {
        let raw = unsafe { luaL_newstate() };
        let raw = NonNull::new(raw).ok_or(
            Error{status: LUA_ERRMEM, error: [].into()}
        )?;
        Ok(Self{raw})
    }

    pub fn do_string(&self, name: &CStr, source: &[u8]) -> Result<()>
    {
        let status = unsafe {
            luaL_loadbufferx(
                /* L    */ self.raw.as_ptr(),
                /* buff */ source.as_ptr().cast::<libc::c_char>(),
                /* sz   */ source.len() as u64,
                /* name */ name.as_ptr(),
                /* mode */ "t\0".as_ptr().cast::<libc::c_char>(),
            )
        };
        if status != LUA_OK {
            let error = unsafe { self.pop_string() };
            return Err(Error{status, error});
        }

        let status = unsafe {
            lua_pcallk(
                /* L        */ self.raw.as_ptr(),
                /* nargs    */ 0,
                /* nresults */ 0,
                /* msgh     */ 0,
                /* ctx      */ 0,
                /* k        */ None,
            )
        };
        if status != LUA_OK {
            let error = unsafe { self.pop_string() };
            return Err(Error{status, error});
        }

        Ok(())
    }

    /// Convert the top stack element to a string and pop it off the stack.
    ///
    /// # Safety
    ///
    /// The top stack element must be a string.
    unsafe fn pop_string(&self) -> Box<[u8]>
    {
        let mut len = 0;

        let ptr = luaL_checklstring(
            /* L     */ self.raw.as_ptr(),
            /* index */ -1,
            /* len   */ &mut len,
        );

        debug_assert!(!ptr.is_null());

        lua_settop(
            /* L     */ self.raw.as_ptr(),
            /* index */ -2,
        );

        Box::from(slice::from_raw_parts(ptr.cast(), len as usize))
    }
}

impl Drop for State
{
    fn drop(&mut self)
    {
        unsafe { lua_close(self.raw.as_ptr()); }
    }
}
