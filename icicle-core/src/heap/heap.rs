use {
    super::{Block, Mutator, PreAlloc, UnsafeRef},
    non_zero_ext::NonZeroExt,
    std::{
        collections::{HashMap, HashSet},
        marker::{PhantomData, PhantomPinned},
        num::NonZeroU64,
        ptr::NonNull,
        sync::Mutex,
    },
};

/// Ensure that `'h` is an invariant lifetime.
pub (super) type HeapId<'h> = PhantomData<fn(&'h ()) -> &'h ()>;

/// Garbage-collected heap.
///
/// The `'h` parameter identifies the heap at the type level.
/// This prevents objects from pointing to objects on different heaps,
/// which would cause the garbage collector to crash horribly.
/// The `'h` parameter can also be used as a lifetime for the heap.
pub struct Heap<'h>
{
    /// Uniquely identifies this heap.
    _heap_id: HeapId<'h>,

    /// Heaps are referenced all over the place.
    _pinned: PhantomPinned,

    /// Pre-allocated objects.
    pub pre_alloc: PreAlloc<'h>,

    /// Non-allocator blocks that constitute the heap.
    ///
    /// Note that more blocks can be found in the mutators.
    blocks: Mutex<Vec<Block<'h>>>,

    /// Tracks the existence of each mutator.
    ///
    /// Each mutator must be known to the garbage collector,
    /// so that the garbage collector can see its stack root batches.
    /// The entries in this set are automatically maintained
    /// by [`Mutator::new`] and [`Mutator::drop`].
    mutators: Mutex<HashSet<NonNull<Mutator<'h>>>>,

    /// Tracks the existence of each pinned root.
    ///
    /// This map stores for each object how many pinned roots reference it.
    /// If an object exists in this map, the garbage collector
    /// is prohibited from moving or garbage collecting the object.
    /// The entries in this map are automatically maintained
    /// by [`PinnedRoot::new`] and [`PinnedRoot::drop`].
    ///
    /// [`PinnedRoot::new`]: `super::PinnedRoot::new`
    /// [`PinnedRoot::drop`]: `super::PinnedRoot::drop`
    pinned_roots: Mutex<HashMap<UnsafeRef<'h>, NonZeroU64>>,
}

impl<'h> Heap<'h>
{
    /// Create a heap with a unique `'h` parameter.
    ///
    /// The heap is passed to the given function.
    /// When the function returns or panics, the heap is destroyed.
    pub fn with<F, R>(f: F) -> R
        // NOTE: Using Self here would allow the caller to choose 'h.
        //       That could result in multiple heaps with the same 'h.
        // NOTE: The heap must be behind a reference.
        //       Otherwise the given function could move it.
        //       This must be prevented, because heaps are referenced
        //       in several places the borrow checker is unaware of.
        where F: for<'i> FnOnce(&'i Heap<'i>) -> R
    {
        let heap = Heap{
            _heap_id: PhantomData,
            _pinned: PhantomPinned,
            pre_alloc: PreAlloc::dangling(),
            blocks: Mutex::new(Vec::new()),
            mutators: Mutex::new(HashSet::new()),
            pinned_roots: Mutex::new(HashMap::new()),
        };

        // SAFETY: Called exactly once during heap construction.
        unsafe { heap.pre_alloc.init(&heap); }

        f(&heap)
    }

    /// Add a block to the heap.
    pub (super) fn add_block(&self, block: Block<'h>)
    {
        let mut blocks = self.blocks.lock().unwrap();
        blocks.push(block);
    }

    /// Register a mutator with the heap.
    ///
    /// # Safety
    ///
    /// Must only be called by [`Mutator::new`].
    pub (super) unsafe fn register_mutator(
        &'h self,
        mutator: NonNull<Mutator<'h>>,
    )
    {
        let mut set = self.mutators.lock().unwrap();
        set.insert(mutator);
    }

    /// Unregister a mutator with the heap.
    ///
    /// # Safety
    ///
    /// Must only be called by [`Mutator::drop`].
    pub (super) unsafe fn unregister_mutator(
        &'h self,
        mutator: NonNull<Mutator<'h>>,
    )
    {
        let mut set = self.mutators.lock().unwrap();
        set.take(&mutator).expect("Use-after-drop of mutator");
    }

    /// Increment the pinned root count for an object.
    ///
    /// # Safety
    ///
    /// Must only be called by [`PinnedRoot::new`].
    ///
    /// [`PinnedRoot::new`]: `super::PinnedRoot::new`
    pub (super) unsafe fn retain_pinned_root(&self, object: UnsafeRef<'h>)
    {
        const ERR: &str = "Too many pinned roots for object";
        let mut pinned_roots = self.pinned_roots.lock().unwrap();
        pinned_roots.entry(object)
            .and_modify(|n| *n = n.checked_add(1).expect(ERR))
            .or_insert(NonZeroU64::ONE);
    }

    /// Decrement the pinned root count for an object.
    ///
    /// # Safety
    ///
    /// Must only be called by [`PinnedRoot::drop`].
    ///
    /// [`PinnedRoot::drop`]: `super::PinnedRoot::drop`
    pub (super) unsafe fn release_pinned_root(&self, object: UnsafeRef<'h>)
    {
        use std::collections::hash_map::Entry::*;
        let mut pinned_roots = self.pinned_roots.lock().unwrap();
        match pinned_roots.entry(object) {
            Occupied(mut entry) =>
                match NonZeroU64::new(entry.get().get() - 1) {
                    Some(n) => { entry.insert(n); },
                    None    => { entry.remove_entry(); },
                },
            Vacant(..) =>
                unreachable!("Use-after-drop of pinned root"),
        }
    }
}
