pub use self::stack_root::*;

use {
    super::{DEFAULT_BLOCK_SIZE, Block, Heap},
    scope_exit::scope_exit,
    std::{
        cell::{RefCell, UnsafeCell},
        marker::PhantomPinned,
        mem::{ManuallyDrop, replace},
        pin::Pin,
        ptr::NonNull,
    },
};

mod stack_root;

/// Thread-local state regarding garbage-collected heaps.
///
/// Locking a mutex for every single operation is too slow.
/// Mutators provide thread-local state for many operations,
/// which is synchronized with the heap only during
/// [safe points] and other infrequent events.
///
/// [safe points]: `Self::safe_point`
pub struct Mutator<'h>
{
    /// The heap to which this mutator belongs.
    pub heap: &'h Heap<'h>,

    /// Mutators are referenced by heaps.
    _pinned: PhantomPinned,

    /// Block on which new small objects are allocated.
    allocator: ManuallyDrop<UnsafeCell<Block<'h>>>,

    /// Active stack root batches maintained by [`with_stack_roots`].
    ///
    /// [`with_stack_roots`]: `Self::with_stack_roots`
    stack_root_batches: RefCell<Vec<*const [StackRoot<'h>]>>,
}

impl<'h> Mutator<'h>
{
    /// Create a mutator for a heap.
    ///
    /// Creating a mutator is not a zero-cost operation.
    /// Please create one such state per thread and keep it around.
    pub fn new(heap: &'h Heap<'h>) -> Pin<Box<Self>>
    {
        let this = Self{
            heap,
            _pinned: PhantomPinned,
            allocator: ManuallyDrop::new(UnsafeCell::new(Block::new(heap))),
            stack_root_batches: RefCell::new(Vec::new()),
        };

        let this = Box::into_pin(Box::new(this));
        let ptr = NonNull::from(this.as_ref().get_ref());

        // SAFETY: Called from Mutator::new.
        unsafe { heap.register_mutator(ptr) };

        this
    }

    /// Enter a safe point.
    ///
    /// Once a garbage collection cycle is planned,
    /// the garbage collector must ensure all mutators
    /// have entered a safe point, during which they cannot mutate.
    /// This method blocks until the planned garbage collection cycle finishes.
    /// This approach is known as "stop the world".
    ///
    /// If no garbage collection cycle is planned,
    /// this method returns immediately.
    pub fn safe_point(&self)
    {
        // SAFETY: The passed function does nothing.
        unsafe { self.safe_point_with(|| ()); }
    }

    /// Enter a safe point but don't block immediately.
    ///
    /// A safe point is entered, and the given function is called immediately.
    /// The function will run in parallel with the garbage collector.
    /// This is similar to [`safe_point`][`Self::safe_point`],
    /// but blocking does not occur until the function returns.
    /// The main purpose of this method is to allow a safe point to exist
    /// (and hence garbage collections to proceed) during FFI calls.
    ///
    /// # Safety
    ///
    /// The given function must not perform allocations,
    /// mutate objects, or read unpinned objects.
    pub unsafe fn safe_point_with<F, R>(&self, f: F) -> R
        where F: FnOnce() -> R
    {
        // TODO: Implement the safe point logic.
        f()
    }

    /// Allocate memory for an object.
    ///
    /// # Safety
    ///
    /// The caller must initialize the allocated memory
    /// before the next garbage collection cycle.
    pub unsafe fn alloc(&self, size: usize) -> NonNull<()>
    {
        if size > DEFAULT_BLOCK_SIZE {
            return self.alloc_large(size);
        }

        if let Some(ptr) = self.alloc_small_fast(size) {
            return ptr;
        }

        self.alloc_small_slow(size)
    }

    /// Allocate an ad-hoc block for this one value.
    #[inline(never)]
    unsafe fn alloc_large(&self, size: usize) -> NonNull<()>
    {
        let mut block = Block::with_capacity(self.heap, size);
        let ptr = block.try_alloc(size)
            .expect("Block should have sufficient space");
        self.heap.add_block(block);
        return ptr;
    }

    /// Try a pointer bump allocation for the value.
    ///
    /// If the value does not fit within the allocator,
    /// this method returns [`None`] and the caller must try elsewhere.
    #[inline(always)]
    unsafe fn alloc_small_fast(&self, size: usize) -> Option<NonNull<()>>
    {
        let block = self.allocator.get();
        (*block).try_alloc(size)
    }

    /// Allocate a new block and allocate the value in there.
    ///
    /// The new block becomes the new allocator for this mutator.
    #[inline(never)]
    unsafe fn alloc_small_slow(&self, size: usize) -> NonNull<()>
    {
        let block = self.allocator.get();

        let mut new_block = Block::new(self.heap);
        let ptr = new_block.try_alloc(size)
            .expect("Block should have sufficient space");

        let old_block = replace(&mut *block, new_block);
        self.heap.add_block(old_block);

        ptr
    }

    /// Allocate stack space for roots.
    ///
    /// A stack root ensures the referenced object won't be garbage collected.
    /// Stack roots are so called because they always live on the Rust stack.
    /// Calling `with_stack_roots::<N>` is more efficient
    /// than working with `N` individual [pinned roots],
    /// but the usage of stack roots is restricted to subsequent stack frames.
    /// The efficiency of stack roots originates from the following factors:
    ///
    ///  - Stack roots are immovable, because they are always behind a `&`.
    ///    Therefore the garbage collector can track their locations,
    ///    and they do not inhibit moving of objects by the garbage collector.
    ///  - The garbage collector keeps track of stack roots in batches,
    ///    rather than keeping track of every individual root separately.
    ///  - Stack root batches are always created and destroyed in LIFO order,
    ///    because of the "with" interface presented by this method.
    ///    This simplifies bookkeeping (using a vec instead of a hash map).
    ///
    /// The stack roots are passed to the given function.
    /// When the function returns or panics, the stack roots are destroyed.
    /// The function may freely [`set`] the stack roots;
    /// the garbage collector will notice their new objects.
    /// Each stack root is initialized to a reference to undef.
    ///
    /// [pinned roots]: `super::PinnedRoot`
    /// [`set`]: `StackRoot::set`
    pub fn with_stack_roots<const N: usize, F, R>(&self, f: F) -> R
        where F: FnOnce(&[StackRoot<'h>; N]) -> R
    {
        // Initialize the stack root batch with undefs.
        // StackRoot doesn't impl Copy so we can't use [StackRoot{..}; N].
        let undef = self.heap.pre_alloc.undef();
        let new_root = |()| unsafe { StackRoot::new(undef) };
        let batch = [(); N].map(new_root);

        // Add the batch to the stack that the garbage collector traverses.
        let mut batches = self.stack_root_batches.borrow_mut();
        batches.push(&batch);
        drop(batches);

        // Remove the batch from the stack when f returns or panics.
        scope_exit! {
            let mut batches = self.stack_root_batches.borrow_mut();
            batches.pop().expect("stack_root_batches should not be empty");
        }

        // Call the given function with the batch.
        f(&batch)
    }
}

impl<'h> Drop for Mutator<'h>
{
    fn drop(&mut self)
    {
        // Make sure the allocator is not dropped,
        // by transferring ownership of it to the heap.
        // SAFETY: Allocator is not used anymore after.
        let allocator = unsafe { ManuallyDrop::take(&mut self.allocator) };
        self.heap.add_block(allocator.into_inner());

        // SAFETY: Called from Mutator::drop.
        unsafe { self.heap.unregister_mutator(NonNull::from(self)); }
    }
}
