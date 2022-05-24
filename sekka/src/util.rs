use std::sync::{Arc, Weak};

/// Like [`Arc::new_cyclic`], but can fail with an error.
///
/// The [`Default`] bound on [`T`] is used to obtain a temporary value.
/// This is due to implementation difficulties surrounding [`Arc::new_cyclic`].
pub fn try_new_cyclic<F, T, E>(data_fn: F) -> Result<Arc<T>, E>
    where F: FnOnce(&Weak<T>) -> Result<T, E>
        , T: Default
{
    let mut error = None;
    let default = |err| { error = Some(err); T::default() };
    let maybe = Arc::new_cyclic(|weak| data_fn(weak).unwrap_or_else(default));
    match error {
        None      => Ok(maybe),
        Some(err) => Err(err),
    }
}
