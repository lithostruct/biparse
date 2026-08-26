use std::{borrow::Cow, fmt::Debug, rc::Rc};

use crate::{
    cst::IdSupply,
    parse::{ParseEnvironment, ParseOutput, ParseState},
};

/// The recursive stack stored in `ParseState`, which can be cheaply cloned.
///
/// Internals hidden due to being specific to `Recursive` and no other combinator.
pub struct RecursiveStack<'src, C, S: IdSupply> {
    inner: im::Vector<RecursiveFrame<'src, C, S>>,
}
impl<'src, C: Clone, S: IdSupply> Default for RecursiveStack<'src, C, S> {
    fn default() -> Self {
        RecursiveStack {
            inner: im::Vector::new(),
        }
    }
}
impl<'src, C: Clone, S: IdSupply> Clone for RecursiveStack<'src, C, S> {
    fn clone(&self) -> Self {
        RecursiveStack {
            inner: self.inner.clone(),
        }
    }
}
impl<'src, C: Clone, S: IdSupply> RecursiveStack<'src, C, S> {
    pub fn new() -> Self {
        Self::default()
    }
    pub(in super) fn get_latest(&self, id: RecursiveId, start_cursor: C) -> Option<&RecursiveFrame<'src, C, S>>
    where
        C: Eq,
    {
        self.inner
            .iter()
            .rev()
            .find(|inner| inner.matches(id, start_cursor.clone()))
    }
    pub(in super) fn push_to_top(&self, frame: RecursiveFrame<'src, C, S>) -> Self {
        RecursiveStack {
            inner: self.inner.clone() + im::Vector::unit(frame),
        }
    }
}

pub(super) struct RecursiveFrame<'src, C, S: IdSupply> {
    // Identifying information
    pub(super) id: RecursiveId,
    pub(super) start_cursor: C,

    // Diagnostic information.
    pub(super) description: Option<Cow<'static, str>>,
    pub(super) diagnostic_name: String,

    // Payload.
    /// This cannot be in `erased` becaue `I` does not live for `'static`.
    pub(super) seed_result: Option<(ParseState<'src, C, S>, Rc<dyn ErasedOutput<'src> + 'src>)>,
}
impl<'src, C: Debug, S: IdSupply> Debug for RecursiveFrame<'src, C, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecursiveFrame")
            .field("description", &self.description)
            .field("erased", &"<erased>")
            .finish()
    }
}
impl<'src, C: Clone, S: IdSupply> Clone for RecursiveFrame<'src, C, S> {
    fn clone(&self) -> Self {
        RecursiveFrame {
            id: self.id,
            start_cursor: self.start_cursor.clone(),
            description: self.description.clone(),
            diagnostic_name: self.diagnostic_name.clone(),
            seed_result: self.seed_result.clone(),
        }
    }
}
impl<C, S: IdSupply> RecursiveFrame<'_, C, S> {
    pub fn matches(&self, id: RecursiveId, start_cursor: C) -> bool
    where
        C: Eq,
    {
        self.id == id && self.start_cursor == start_cursor
    }
}

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
