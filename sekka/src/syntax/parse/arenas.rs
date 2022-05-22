use {super::super::ast::Expression, typed_arena::Arena};

/// Arenas for allocating AST nodes in.
#[allow(missing_docs)]
pub struct Arenas<'a>
{
    pub expressions: &'a Arena<Expression<'a>>,
}

impl<'a> Arenas<'a>
{
    /// Create arenas and pass them to the given function.
    pub fn with<R>(f: impl FnOnce(&Arenas) -> R) -> R
    {
        let expressions = Arena::new();
        let arenas = Arenas{expressions: &expressions};
        f(&arenas)
    }

    /// Move a node to a suitable arena.
    pub fn alloc<T>(&self, node: T) -> &'a T
        where T: ArenaNode<'a>
    {
        node.alloc(self)
    }
}

/// Utility trait for [`Arenas::alloc`].
pub trait ArenaNode<'a>
{
    /// Move the node to a suitable arena.
    fn alloc(self, arenas: &Arenas<'a>) -> &'a Self;
}

impl<'a> ArenaNode<'a> for Expression<'a>
{
    fn alloc(self, arenas: &Arenas<'a>) -> &'a Self
    {
        arenas.expressions.alloc(self)
    }
}
