use std::{
    borrow::Cow,
    fmt::Debug,
    hash::Hash,
    rc::{Rc, Weak},
};

use crate::{
    cst::IdSupply,
    parse::{ParseEnvironment, ParseOutput, ParseStateData},
};

/// The recursive state stored in [`ParseState`](crate::parse::ParseState), which can be cheaply cloned.
///
/// A [`RecursiveState`] must only be used by the parser invocation for which it was created,
/// and must not be retained and reused across unrelated parsing or checking invocations.
/// Violating this requirement may result in panicking.
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
/// throughout the input and output [`ParseState`](crate::parse::ParseState) of their combinators.
///
/// Failure to do so may cause correctness issues or exponential blowup of recursive parsing.
///
/// - **Leaf Combinators:** Return the `RecursiveState` as-is.
///   Do *not* return a [`RecursiveState::new()`] or [`Default::default()`] instance.
/// - **Sequential Combinators:** The `RecursiveState` must be threaded through each step.
///   Do *not* use earlier clones of `RecursiveState` not threaded through each combinator.
/// - **Speculative Combinators:** Clone the `RecursiveState` prior to speculative parsing.
///   Use a fresh clone for each instance of speculative parsing.
///   The `RecursiveState` of the *successful* branch should be returned.
///   `RecursiveState` corresponding to discarded alternatives must *not* be returned, and should be discarded.
/// - **All of the Above:** *Must* hold for *both* parsing and checking modes.
pub struct RecursiveState<'src, C, S: IdSupply, ES: IdSupply> {
    /// The recursion stack + memoization table.
    pub(super) inner: im::HashMap<(RecursiveId, C), RecursiveFrame<'src, C, S, ES>>,

    /// The currently-active recursive combinator invocation.
    pub(super) current_enclosing: Option<(RecursiveId, C)>,
    /// Dependencies for the currently-enclosing recursive combinator.
    pub(super) current_dependencies: im::Vector<(RecursiveId, C, usize)>,
}
impl<'src, C: Clone + Eq + Hash + Debug, S: IdSupply, ES: IdSupply> Debug
    for RecursiveState<'src, C, S, ES>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecursiveState")
            .field("inner", &self.inner)
            .field("current_enclosing", &self.current_enclosing)
            .field("current_dependencies", &self.current_dependencies)
            .finish()
    }
}
impl<'src, C: Clone, S: IdSupply, ES: IdSupply> Default for RecursiveState<'src, C, S, ES> {
    fn default() -> Self {
        RecursiveState {
            inner: im::HashMap::new(),
            current_enclosing: None,
            current_dependencies: im::Vector::new(),
        }
    }
}
impl<'src, C: Clone, S: IdSupply, ES: IdSupply> Clone for RecursiveState<'src, C, S, ES> {
    fn clone(&self) -> Self {
        RecursiveState {
            inner: self.inner.clone(),
            current_enclosing: self.current_enclosing.clone(),
            current_dependencies: self.current_dependencies.clone(),
        }
    }
}
impl<'src, C: Clone, S: IdSupply, ES: IdSupply> RecursiveState<'src, C, S, ES> {
    pub fn new() -> Self {
        Self::default()
    }
}
impl<'src, C: Clone + Hash + Eq, S: IdSupply, ES: IdSupply> RecursiveState<'src, C, S, ES> {
    pub(super) fn get(
        &self,
        id: RecursiveId,
        start_cursor: C,
    ) -> Option<&RecursiveFrame<'src, C, S, ES>>
    where
        C: Eq,
    {
        self.inner.get(&(id, start_cursor))
    }
    pub(super) fn get_mut(
        &mut self,
        id: RecursiveId,
        start_cursor: C,
    ) -> Option<&mut RecursiveFrame<'src, C, S, ES>>
    where
        C: Eq,
    {
        self.inner.get_mut(&(id, start_cursor))
    }
    pub(super) fn push_to_top(
        &mut self,
        id: RecursiveId,
        start_cursor: C,
        frame: RecursiveFrame<'src, C, S, ES>,
    ) {
        self.current_enclosing = Some((id, start_cursor.clone()));
        self.current_dependencies = frame.dependencies.clone();
        self.inner.insert((id, start_cursor), frame);
    }
}

/// A frame containing recursive information.
pub(super) struct RecursiveFrame<'src, C, S: IdSupply, ES: IdSupply> {
    // IDENTIFYING INFORMATION
    /// The user-given name for this recursive combinator.
    pub(super) description: Option<Cow<'static, str>>,
    /// Diagnostic name, for use in debugging. Typically a string representation of the pointer address.
    pub(super) diagnostic_name: String,

    // SAFETY INFORMATION
    /// A weak pointer to this recursive combinator.
    ///
    /// Unsafe code in [`Recursive`](crate::combinator::Recursive) assumes the critical invariant that
    /// the identical parser that produces this frame is the one that reads and downcasts the payload.
    /// This may be invalid in cases where the pointer address of the original combinator
    /// is overwritten by another of a different payload type,
    /// leading to UB when downcasting to type or lifetime mismatch.
    ///
    /// This field is such that while this frame remains live in the [`RecursiveState`],
    /// the underlying allocation store to the combinator remains live,
    /// even if the inner combinator is dropped,
    /// thus occupying the pointer address and not allowing for overwriting.
    pub(super) allocation_store: Box<dyn ErasedWeak + 'src>,

    // PAYLOAD INFORMATION
    /// The seed result of this recursion. [`None`] represents an unevaluated recursion (or unbounded recursion),
    /// [`Some`] means that there exists a seed state that can be recovered.
    /// Payload erased via [`ErasedOutput`], which is recovered via unsafe code in [`Recursive`](crate::combinator::Recursive).
    #[allow(clippy::type_complexity)]
    pub(super) seed_result: Option<(ParseStateData<'src, C, S, ES>, Rc<dyn ErasedOutput>)>,

    /// Informs whether this frame is currently involved in fixpoint seed growing.
    pub(super) active_fixpoint: bool,

    /// Returns the generation of this fixpoint. This is incremented every seed-growing iteration.
    /// Used to determine whether cached results can be reused.
    pub(super) generation: usize,

    /// Dependencies used to calculate this seed result.
    /// Used for cache invalidation.
    pub(super) dependencies: im::Vector<(RecursiveId, C, usize)>,
}
impl<'src, C: Clone + Debug, S: IdSupply, ES: IdSupply> Debug for RecursiveFrame<'src, C, S, ES> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecursiveFrame")
            .field("description", &self.description)
            .field("diagnostic_name", &self.diagnostic_name)
            .field(
                "seed_result",
                &if self.seed_result.is_none() {
                    "None"
                } else {
                    "<erased>"
                },
            )
            .field("active_fixpoint", &self.active_fixpoint)
            .field("generation", &self.generation)
            .field("dependencies", &self.dependencies)
            .finish()
    }
}
impl<'src, C: Clone, S: IdSupply, ES: IdSupply> Clone for RecursiveFrame<'src, C, S, ES> {
    fn clone(&self) -> Self {
        RecursiveFrame {
            description: self.description.clone(),
            diagnostic_name: self.diagnostic_name.clone(),
            allocation_store: self.allocation_store.clone_erased(),
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
pub(super) trait ErasedOutput {
    fn clone_erased(&self) -> Rc<dyn ErasedOutput>;
    fn as_ptr(&self) -> *const ();
    fn type_id(&self) -> std::any::TypeId;
}
impl<A, E: ParseEnvironment> ErasedOutput for ParseOutput<A, E>
where
    A: Clone + 'static,
{
    fn clone_erased(&self) -> Rc<dyn ErasedOutput> {
        Rc::new((*self).clone())
    }
    fn as_ptr(&self) -> *const () {
        self as *const Self as *const ()
    }
    fn type_id(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
    }
}

pub(super) trait ErasedWeak {
    fn clone_erased<'a>(&self) -> Box<dyn ErasedWeak + 'a>
    where
        Self: 'a;
}
impl<P: ?Sized> ErasedWeak for Weak<P> {
    fn clone_erased<'a>(&self) -> Box<dyn ErasedWeak + 'a>
    where
        Self: 'a,
    {
        Box::new(self.clone())
    }
}
