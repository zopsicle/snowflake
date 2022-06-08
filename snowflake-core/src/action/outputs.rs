/// Information about the outputs of an action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outputs<T>
{
    /// The action has outputs described by `T`.
    ///
    /// `T` may be empty or zero, in which case the action is always [pruned].
    /// To prevent an action from being pruned, use [`Lint`][`Self::Lint`].
    ///
    /// [pruned]: `super::ActionGraph::prune`
    Outputs(T),

    /// The action is a lint action.
    ///
    /// Lint actions don't produce outputs;
    /// they are performed for errors and warnings only.
    /// Unlike regular actions with zero outputs,
    /// lint actions are never [pruned].
    ///
    /// [pruned]: `super::ActionGraph::prune`
    Lint,
}

impl<T> Outputs<T>
{
    /// Borrow the outputs, if any.
    pub fn as_ref(&self) -> Outputs<&T>
    {
        match self {
            Self::Outputs(outputs) => Outputs::Outputs(outputs),
            Self::Lint => Outputs::Lint,
        }
    }

    /// Modify the outputs, if any.
    ///
    /// ```
    /// # use snowflake_core::action::Outputs;
    /// let a: Outputs<i32> = Outputs::Outputs(1);
    /// let b: Outputs<i32> = Outputs::Lint;
    /// assert_eq!(a.map(|x| x + 1), Outputs::Outputs(2));
    /// assert_eq!(b.map(|x| x + 1), Outputs::Lint);
    /// ```
    pub fn map<F, U>(self, f: F) -> Outputs<U>
        where F: FnOnce(T) -> U
    {
        match self {
            Self::Outputs(outputs) => Outputs::Outputs(f(outputs)),
            Self::Lint => Outputs::Lint,
        }
    }
}
