//! Garbage-collected heaps and objects.

pub use self::object::UnsafeHandle;

use {
    self::{
        object::OwnedHandle,
        unsafe_ref_cell::UnsafeRefCell,
    },
    scope_exit::scope_exit,
    smallvec::SmallVec,
    std::{
        cell::Cell,
        collections::HashSet,
        mem::MaybeUninit,
        ptr::{NonNull, addr_of, null},
        sync::Mutex,
    },
};

mod object;
mod unsafe_ref_cell;

/* -------------------------------------------------------------------------- */
/*                Data types for isolates, mutators, and scopes               */
/* -------------------------------------------------------------------------- */

/// Thread-safe garbage-collected heap.
pub struct Isolate
{
    /// Interned undef object.
    ///
    /// Newly created scoped handles refer to this object.
    /// The garbage collector treats this as a root.
    undef: UnsafeHandle,

    gen_1: Mutex<Vec<OwnedHandle>>,

    mutators: Mutex<HashSet<*const Mutator>>,
}

/// Thread-local interface to an isolate.
///
/// Each mutator has its own zeroth generation, on which allocations happen.
/// These allocations can happen without any need for synchronization.
/// This makes working with the isolate through mutators very fast!
///
/// To create a mutator, use [`Isolate::with_mutator`].
pub struct Mutator
{
    /// The isolate to which this mutator belongs.
    isolate: *const Isolate,

    /// The area in which new objects are allocated.
    gen_0: UnsafeRefCell<Vec<OwnedHandle>>,

    /// The bottommost scope, or null if none.
    first_scope: Cell<*const Scope>,
}

/// LIFO-managed collection of roots.
///
/// Keeping track of roots individually is inefficient,
/// because bookkeeping would happen at every handle clone and drop.
/// We amortize these costs by keeping track of roots in bulk, through scopes.
/// Each mutator manages a stack of scopes, in tandem with the call stack.
/// Each scope manages a collection of handles,
/// which the garbage collector treats as roots.
///
/// To create a scope, use [`Mutator::with_scope`] or [`Scope::with_scope`].
/// Creating a new scope must always happen at the top of the stack;
/// creating a scope that would be a sibling of another scope panics.
pub struct Scope
{
    /// The isolate to which this scope belongs.
    isolate: *const Isolate,

    /// Append-only list of handles on this scope.
    handles: UnsafeRefCell<SmallVec<[UnsafeHandle; 2]>>,

    /// The next scope, or null if none.
    next_scope: Cell<*const Scope>,
}

/// Scope-managed handle to an object.
#[derive(Clone, Copy)]
pub struct ScopedHandle<'a>
{
    /// The scope to which this handle belongs.
    scope: &'a Scope,

    /// Indexes into [`Scope::handles`].
    index: usize,
}

// Isolates are inferred !Send + !Sync because of *const Mutator.
// But we only access mutators during garbage collection safe points.
unsafe impl Send for Isolate { }
unsafe impl Sync for Isolate { }

/* -------------------------------------------------------------------------- */
/*                   Managing isolates, mutators, and scopes                  */
/* -------------------------------------------------------------------------- */

impl Isolate
{
    /// Create a new isolate.
    pub fn new() -> Self
    {
        let mut this = Self{
            undef: UnsafeHandle{inner: NonNull::dangling()},
            gen_1: Mutex::new(Vec::new()),
            mutators: Mutex::new(HashSet::new()),
        };

        this.undef = this.with_mutator(|mutator| {
            // SAFETY: Alloc and init are called correctly.
            unsafe {
                let undef = mutator.alloc_undef();
                Mutator::init_undef(undef);
                undef
            }
        });

        this
    }

    /// Create a mutator for the duration of a callback.
    ///
    /// This registers and unregisters the mutator with the isolate,
    /// which are operations that require temporarily taking a lock.
    /// Hence it is advised to create mutators infrequently.
    ///
    /// The reason this is a callback-based interface
    /// is so that we can reliably unregister the mutator.
    /// Instead returning a guard object would not work,
    /// since the caller could [`forget`] it,
    /// causing a dangling pointer to the mutator
    /// in the isolate's mutator registry.
    ///
    /// [`forget`]: `std::mem::forget`
    pub fn with_mutator<F, R>(&self, f: F) -> R
        where F: FnOnce(&Mutator) -> R
    {
        // Create the mutator on the call stack.
        let mutator = Mutator{
            isolate: self,
            gen_0: UnsafeRefCell::new(Vec::new()),
            first_scope: Cell::new(null()),
        };

        // Register the mutator with the isolate.
        let mut mutators = self.mutators.lock().unwrap();
        mutators.insert(addr_of!(mutator));
        drop(mutators);

        scope_exit! {

            // The mutator disappearing shouldn't invalidate any references,
            // so promote any objects from generation 0 to generation 1.
            let mut gen_1 = self.gen_1.lock().unwrap();
            // SAFETY: gen_0 is not currently borrowed.
            let mut gen_0 = unsafe { mutator.gen_0.borrow_mut() };
            gen_1.extend(gen_0.drain(..));
            drop(gen_0);
            drop(gen_1);

            // Unregister the mutator with the isolate.
            // After this, the stop-the-world mechanism will
            // no longer wait for this mutator to enter a safe point.
            // Hence, this must happen *after* promoting to generation 1!
            let mut mutators = self.mutators.lock().unwrap();
            mutators.remove(&addr_of!(mutator));
            drop(mutators);

        }

        f(&mutator)
    }
}

impl Mutator
{
    /// The isolate to which this mutator belongs.
    pub fn isolate(&self) -> &Isolate
    {
        // SAFETY: The isolate always outlives any mutators it owns.
        unsafe { &*self.isolate }
    }

    /// Create a scope for the duration of a callback.
    ///
    /// See [`Scope`] for how scopes are managed.
    pub fn with_scope<F, R>(&self, f: F) -> R
        where F: FnOnce(&Scope) -> R
    {
        // There must not already be any scopes.
        assert!(self.first_scope.get().is_null());

        // Create the scope on the call stack.
        let scope = Scope{
            isolate: self.isolate,
            handles: UnsafeRefCell::new(SmallVec::new()),
            next_scope: Cell::new(null()),
        };

        // Register the scope with the mutator.
        self.first_scope.set(addr_of!(scope));

        scope_exit! {
            // Unregister the scope with the mutator.
            self.first_scope.set(null());
        }

        f(&scope)
    }
}

impl Scope
{
    /// The isolate to which this scope belongs.
    pub fn isolate(&self) -> &Isolate
    {
        // SAFETY: The isolate always outlives any scopes it owns.
        unsafe { &*self.isolate }
    }

    /// Create a scope for the duration of a callback.
    ///
    /// See [`Scope`] for how scopes are managed.
    pub fn with_scope<F, R>(&self, f: F) -> R
        where F: FnOnce(&Scope) -> R
    {
        // This scope must be the topmost scope.
        assert!(self.next_scope.get().is_null());

        // Create the scope on the call stack.
        let scope = Scope{
            isolate: self.isolate,
            handles: UnsafeRefCell::new(SmallVec::new()),
            next_scope: Cell::new(null()),
        };

        // Register the scope with the mutator.
        self.next_scope.set(addr_of!(scope));

        scope_exit! {
            // Unregister the scope with the mutator.
            self.next_scope.set(null());
        }

        f(&scope)
    }

    /// Create a new handle, initialized to undef.
    pub fn new_handle(&self) -> ScopedHandle
    {
        // SAFETY: handles is not currently borrowed.
        let mut handles = unsafe { self.handles.borrow_mut() };
        let index = handles.len();
        handles.push(self.isolate().undef);
        drop(handles);
        ScopedHandle{scope: self, index}
    }
}

impl<'a> ScopedHandle<'a>
{
    /// Get the underlying handle.
    pub fn get(self) -> UnsafeHandle
    {
        // SAFETY: handles is not currently borrowed.
        let handles = unsafe { self.scope.handles.borrow() };

        // SAFETY: handles is append-only, so index is still in bounds.
        unsafe { *handles.get_unchecked(self.index) }
    }

    /// Set the underlying handle.
    ///
    /// # Safety
    ///
    /// The unsafe handle must reference a live object
    /// on the isolate that this scoped handle belongs to.
    pub unsafe fn set(self, handle: UnsafeHandle)
    {
        // SAFETY: handles is not currently borrowed.
        let mut handles = self.scope.handles.borrow_mut();

        // SAFETY: handles is append-only, so index is still in bounds.
        *handles.get_unchecked_mut(self.index) = handle;
    }
}

/* -------------------------------------------------------------------------- */
/*                              Creating objects                              */
/* -------------------------------------------------------------------------- */

/// Methods for creating objects.
///
/// For each type of object there are three
/// classes of methods: `new`, `alloc`, and `init`.
/// The `new` methods simply call `alloc` followed by `init`.
/// The `alloc` methods allocate uninitialized memory.
/// The `init` methods initialize this memory.
///
/// Multiple `alloc` calls may be made followed by multiple `init` calls.
/// This enables the creation of mutually recursive objects.
/// If no mutually recursive objects are needed,
/// the safe `new` methods can be used instead.
///
/// # Safety
///
/// After a call to `alloc` has been made,
/// the appropriate `init` method must be called
/// prior to the next garbage collection safe point.
/// Do not panic between `alloc` and `init`!
///
/// Any callbacks passed to `new` and `init` methods
/// must not panic and must not mutate the isolate.
/// If given buffers, those must be left fully initialized.
///
/// When creating an object that references other objects,
/// the referenced objects must live on the same isolate
/// as the object being created.
///
/// # Examples
///
/// Use the `new` methods to atomically create objects:
///
/// ```
/// # use {sekka::isolate::Isolate, std::mem::MaybeUninit};
/// # let isolate = Isolate::new();
/// # isolate.with_mutator(|mutator| {
/// mutator.with_scope(|scope| {
///     let handle_1 = scope.new_handle();
///     let handle_2 = scope.new_handle();
///     mutator.new_undef(handle_1);
///     mutator.new_string_from_bytes(handle_2, b"Hello, world!");
/// });
/// # });
/// ```
///
/// Create three tuples that reference each other:
///
/// ```
/// # use sekka::isolate::{Isolate, Mutator};
/// # let isolate = Isolate::new();
/// # isolate.with_mutator(|mutator| {
/// mutator.with_scope(|scope| {
///     let handle_1 = scope.new_handle();
///     let handle_2 = scope.new_handle();
///     let handle_3 = scope.new_handle();
///     unsafe {
///         let tuple_1 = mutator.alloc_tuple(2);
///         let tuple_2 = mutator.alloc_tuple(2);
///         let tuple_3 = mutator.alloc_tuple(2);
///         Mutator::init_tuple_from_fn(tuple_1, 2, |buf| {
///             buf[0].write(tuple_2);
///             buf[1].write(tuple_3);
///         });
///         Mutator::init_tuple_from_fn(tuple_2, 2, |buf| {
///             buf[0].write(tuple_1);
///             buf[1].write(tuple_3);
///         });
///         Mutator::init_tuple_from_fn(tuple_3, 2, |buf| {
///             buf[0].write(tuple_1);
///             buf[1].write(tuple_2);
///         });
///         handle_1.set(tuple_1);
///         handle_2.set(tuple_2);
///         handle_3.set(tuple_3);
///     }
/// });
/// # });
/// ```
#[allow(missing_docs)]
impl Mutator
{
    unsafe fn alloc(&self, size: usize) -> UnsafeHandle
    {
        let inner = libc::malloc(size);
        let inner = NonNull::new(inner).unwrap();
        let unsafe_handle = UnsafeHandle{inner: inner.cast()};
        let owned_handle = OwnedHandle{inner: unsafe_handle};

        let mut gen_0 = self.gen_0.borrow_mut();
        gen_0.push(owned_handle);
        drop(gen_0);

        unsafe_handle
    }

    /* ------------------------------------------------------------------ */
    /*                                Undef                               */
    /* ------------------------------------------------------------------ */

    pub unsafe fn alloc_undef(&self) -> UnsafeHandle
    {
        use self::object::Undef;
        self.alloc(Undef::size())
    }

    pub unsafe fn init_undef(object: UnsafeHandle)
    {
        use self::object::Undef;
        Undef::init(object.inner.as_ptr().cast());
    }

    pub fn new_undef(&self, into: ScopedHandle)
    {
        unsafe {
            let handle = self.alloc_undef();
            Self::init_undef(handle);
            into.set(handle);
        }
    }

    /* ------------------------------------------------------------------ */
    /*                               String                               */
    /* ------------------------------------------------------------------ */

    pub unsafe fn alloc_string(&self, len: usize) -> UnsafeHandle
    {
        use self::object::String;
        self.alloc(String::size(len))
    }

    pub unsafe fn init_string_from_fn<F>(object: UnsafeHandle, len: usize, f: F)
        where F: FnOnce(&mut [MaybeUninit<u8>])
    {
        use self::object::String;
        String::init(object.inner.as_ptr().cast(), len, f)
    }

    pub unsafe fn new_string_from_fn<F>
        (&self, into: ScopedHandle, len: usize, f: F)
        where F: FnOnce(&mut [MaybeUninit<u8>])
    {
        let handle = self.alloc_string(len);
        Self::init_string_from_fn(handle, len, f);
        into.set(handle);
    }

    pub unsafe fn init_string_from_bytes(object: UnsafeHandle, bytes: &[u8])
    {
        Self::init_string_from_fn(object, bytes.len(), |buf| {
            MaybeUninit::write_slice(buf, bytes);
        });
    }

    pub fn new_string_from_bytes(&self, into: ScopedHandle, bytes: &[u8])
    {
        unsafe {
            let handle = self.alloc_string(bytes.len());
            Self::init_string_from_bytes(handle, bytes);
            into.set(handle);
        }
    }

    /* ------------------------------------------------------------------ */
    /*                                Tuple                               */
    /* ------------------------------------------------------------------ */

    pub unsafe fn alloc_tuple(&self, len: usize) -> UnsafeHandle
    {
        use self::object::Tuple;
        self.alloc(Tuple::size(len))
    }

    pub unsafe fn init_tuple_from_fn<F>(object: UnsafeHandle, len: usize, f: F)
        where F: FnOnce(&mut [MaybeUninit<UnsafeHandle>])
    {
        use self::object::Tuple;
        Tuple::init(object.inner.as_ptr().cast(), len, f)
    }

    pub unsafe fn new_tuple_from_fn<F>
        (&self, into: ScopedHandle, len: usize, f: F)
        where F: FnOnce(&mut [MaybeUninit<UnsafeHandle>])
    {
        let handle = self.alloc_tuple(len);
        Self::init_tuple_from_fn(handle, len, f);
        into.set(handle);
    }
}
