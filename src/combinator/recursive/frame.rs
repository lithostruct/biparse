use std::{borrow::Cow, fmt::Debug, hash::Hash, rc::Rc};

use crate::{
    cst::IdSupply,
    parse::{ParseEnvironment, ParseOutput, ParseState},
};

/// The recursive state stored in [`ParseState`], which can be cheaply cloned.
///
/// # Note / Warning to [`Parser`](crate::parse::Parser) Implementors
/// 
/// This data type encapsulates a lot of the invariants and information
/// that makes [`Recursive`](crate::combinator::Recursive) work, including but not limited to:
/// - Left-recursion and seed-growing,
/// - Memoization and deduplication of work,
/// - Dependency tracking between recursive combinator invocations.
/// 
/// Implementors *must* observe the following invariants when it comes to threading [`RecursiveState`]
/// throughout the input and output [`ParseState`] of their combinators.
/// 
/// Failure to do so may cause correctness issues or exponential blowup of recursive parsing.
/// 
/// - **Leaf Combinators:** Return the `RecursiveState` as-is.
///   Do *not* return a [`RecursiveState::new()`] or [`Default::default()`] instance.
/// - **Sequential Combinators:** The `RecursiveState` must be threaded through each step.
///   Do *not* use earlier clones of `RecursiveState` not threaded through each combinator.
/// - **Speculative Combinators:** Clone the `RecursiveState` prior to speculative parsing.
///   Use a fresh clone for each instance of speculative parsing.
///   The `RecursiveState` of the successful branch should be returned.
///   `RecursiveState` corresponding to discarded alternatives must *not* be returned, and should also be discarded.
/// - **All of the Above:** *Must* hold for *both* parsing and checking modes.
pub struct RecursiveState<'src, C, S: IdSupply> {
    /// The recursion stack + memoization table.
    pub(super) inner: im::HashMap<(RecursiveId, C), RecursiveFrame<'src, C, S>>,

    /// The currently-active recursive combinator invocation.
    pub(super) current_enclosing: Option<(RecursiveId, C)>,
    /// Dependencies for the currently-enclosing recursive combinator.
    pub(super) current_dependencies: im::Vector<(RecursiveId, C, usize)>,
}
impl<'src, C: Clone + Eq + Hash + Debug, S: IdSupply> Debug for RecursiveState<'src, C, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecursiveState")
            .field("inner", &self.inner)
            .field("current_enclosing", &self.current_enclosing)
            .field("current_dependencies", &self.current_dependencies)
            .finish()
    }
}
impl<'src, C: Clone, S: IdSupply> Default for RecursiveState<'src, C, S> {
    fn default() -> Self {
        RecursiveState {
            inner: im::HashMap::new(),
            current_enclosing: None,
            current_dependencies: im::Vector::new(),
        }
    }
}
impl<'src, C: Clone, S: IdSupply> Clone for RecursiveState<'src, C, S> {
    fn clone(&self) -> Self {
        RecursiveState {
            inner: self.inner.clone(),
            current_enclosing: self.current_enclosing.clone(),
            current_dependencies: self.current_dependencies.clone(),
        }
    }
}
impl<'src, C: Clone, S: IdSupply> RecursiveState<'src, C, S> {
    pub fn new() -> Self {
        Self::default()
    }
}
impl<'src, C: Clone + Hash + Eq, S: IdSupply> RecursiveState<'src, C, S> {
    pub(super) fn get(&self, id: RecursiveId, start_cursor: C) -> Option<&RecursiveFrame<'src, C, S>>
    where
        C: Eq,
    {
        self.inner
            .get(&(id, start_cursor))
    }
    pub(super) fn get_mut(&mut self, id: RecursiveId, start_cursor: C) -> Option<&mut RecursiveFrame<'src, C, S>>
    where
        C: Eq,
    {
        self.inner
            .get_mut(&(id, start_cursor))
    }
    pub(super) fn push_to_top(&mut self, id: RecursiveId, start_cursor: C, frame: RecursiveFrame<'src, C, S>) {
        self.current_enclosing = Some((id, start_cursor.clone()));
        self.current_dependencies = frame.dependencies.clone();
        self.inner.insert((id, start_cursor), frame);
    }
}

pub(super) struct RecursiveFrame<'src, C, S: IdSupply> {
    // Safety information
    pub(super) id: RecursiveId,

    // Diagnostic information.
    pub(super) description: Option<Cow<'static, str>>,
    pub(super) diagnostic_name: String,

    // Payload information.
    pub(super) seed_result: Option<(ParseState<'src, C, S>, Rc<dyn ErasedOutput<'src> + 'src>)>,
    pub(super) active_fixpoint: bool,
    pub(super) generation: usize,
    pub(super) dependencies: im::Vector<(RecursiveId, C, usize)>,
}
impl<'src, C: Clone + Debug, S: IdSupply> Debug for RecursiveFrame<'src, C, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecursiveFrame")
            .field("id", &self.id)
            .field("description", &self.description)
            .field("diagnostic_name", &self.diagnostic_name)
            .field("seed_result", &if self.seed_result.is_none() {"None"} else {"<erased>"})
            .field("active_fixpoint", &self.active_fixpoint)
            .field("generation", &self.generation)
            .field("dependencies", &self.dependencies)
            .finish()
    }
}
impl<'src, C: Clone, S: IdSupply> Clone for RecursiveFrame<'src, C, S> {
    fn clone(&self) -> Self {
        RecursiveFrame {
            id: self.id,
            description: self.description.clone(),
            diagnostic_name: self.diagnostic_name.clone(),
            seed_result: self.seed_result.clone(),
            active_fixpoint: self.active_fixpoint,
            generation: self.generation,
            dependencies: self.dependencies.clone(),
        }
    }
}

/// Newtype around the pointer address of a recursive combinator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct RecursiveId(pub(super) *const ());

/// Trait erasure for storing heterogeneous [`ParseOutput<A, E>`] within [`RecursiveFrame`].
pub(super) trait ErasedOutput<'src> {
    fn clone_erased(&self) -> Rc<dyn ErasedOutput<'src> + 'src>;
    fn as_ptr(&self) -> *const ();
}
impl<'src, A, E: ParseEnvironment> ErasedOutput<'src> for ParseOutput<A, E>
where
    A: Clone + 'src,
{
    fn clone_erased(&self) -> Rc<dyn ErasedOutput<'src> + 'src> {
        Rc::new((*self).clone())
    }
    fn as_ptr(&self) -> *const () {
        self as *const Self as *const ()
    }
}