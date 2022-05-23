use {super::super::ast::*, typed_arena::Arena};

/// Arenas for allocating AST nodes in.
#[allow(missing_docs)]
pub struct Arenas<'a>
{
    pub expressions: &'a Arena<Expression<'a>>,
    pub statements: &'a Arena<Statement<'a>>,
}

impl<'a> Arenas<'a>
{
    /// Create arenas and pass them to the given function.
    pub fn with<R>(f: impl FnOnce(&Arenas) -> R) -> R
    {
        let expressions = &Arena::new();
        let statements = &Arena::new();
        let arenas = Arenas{expressions, statements};
        f(&arenas)
    }

    /// Move a node to a suitable arena.
    pub fn alloc<T>(&self, node: T) -> &'a mut T
        where T: ArenaNode<'a>
    {
        T::arena(self).alloc(node)
    }

    /// Move a collection of nodes to a suitable arena.
    pub fn alloc_extend<I>(&self, nodes: I) -> &'a mut [I::Item]
        where I: IntoIterator
            , I::Item: ArenaNode<'a>
    {
        I::Item::arena(self).alloc_extend(nodes)
    }
}

/// Utility trait for [`Arenas::alloc`].
pub trait ArenaNode<'a>: Sized
{
    /// Return the arena for this type of node.
    fn arena(arenas: &Arenas<'a>) -> &'a Arena<Self>;
}

impl<'a> ArenaNode<'a> for Expression<'a>
{
    fn arena(arenas: &Arenas<'a>) -> &'a Arena<Self>
    {
        arenas.expressions
    }
}

impl<'a> ArenaNode<'a> for Statement<'a>
{
    fn arena(arenas: &Arenas<'a>) -> &'a Arena<Self>
    {
        arenas.statements
    }
}
