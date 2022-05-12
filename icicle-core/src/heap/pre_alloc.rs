use {super::{Block, Heap, UnsafeRef}, std::{cell::Cell, ptr::NonNull}};

macro_rules! pre_alloc
{
    { $($name:ident $what:literal $size:literal |$ptr:ident| $init:expr,)* } => {

        /// Pre-allocated objects.
        ///
        /// Small objects with little structure are pre-allocated.
        /// Think of objects like undef, Booleans, and small integers.
        /// There is no need to allocate these over and over again.
        /// Available through [`Heap::pre_alloc`].
        pub struct PreAlloc<'h>
        {
            // These must be cells because they are initialized separately.
            // They cannot be initialized immediately due to lifetime issues.
            $($name: Cell<UnsafeRef<'h>>,)*
        }

        // SAFETY: Not mutated anymore after call to init.
        unsafe impl<'h> Send for PreAlloc<'h> { }
        unsafe impl<'h> Sync for PreAlloc<'h> { }

        impl<'h> PreAlloc<'h>
        {
            /// Prepare for allocating the objects.
            pub (super) fn dangling() -> Self
            {
                Self{
                    $($name: Cell::new(UnsafeRef::new(NonNull::dangling())),)*
                }
            }

            /// Allocate and initialize the objects.
            ///
            /// # Safety
            ///
            /// This must be called exactly once during heap construction.
            pub (super) unsafe fn init(&self, heap: &'h Heap<'h>)
            {
                const BLOCK_SIZE: usize = 24;
                let mut block = Block::with_capacity(heap, BLOCK_SIZE);

                $({
                    const ERR: &str = "Cannot pre-allocate object";
                    let $ptr = block.try_alloc($size).expect(ERR);
                    $init;
                    self.$name.set(UnsafeRef::new($ptr));
                })*

                heap.add_block(block);
            }

            $(
                #[doc = concat!("The pre-allocated ", $what, " object.")]
                pub fn $name(&self) -> UnsafeRef<'h>
                {
                    self.$name.get()
                }
            )*
        }

    };
}

pre_alloc!
{
    // TODO: Properly initialize the values.
    undef         "undef"         1 |ptr| {},
    boolean_false "Boolean false" 1 |ptr| {},
    boolean_true  "Boolean true"  1 |ptr| {},
}
