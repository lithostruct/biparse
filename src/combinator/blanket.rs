use std::{fmt::Debug, hash::Hash, marker::PhantomData, rc::Rc, sync::Arc};

use crate::{
    input::Input,
    parse::{ParseEnvironment, ParseHint, ParseOutput, ParseState, Parser},
};

impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, A, E> for &P
where
    P: Parser<'src, I, A, E> + ?Sized,
{
    #[inline(always)]
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<A, E>) {
        <P as Parser<'src, I, A, E>>::parse(*self, input, state)
    }
    #[inline(always)]
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<(), E>) {
        <P as Parser<'src, I, A, E>>::check(*self, input, state)
    }
    #[inline(always)]
    fn hint(&self) -> ParseHint {
        <P as Parser<'src, I, A, E>>::hint(self)
    }
}

impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, A, E> for Box<P>
where
    P: Parser<'src, I, A, E> + ?Sized,
{
    #[inline(always)]
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<A, E>) {
        (&**self as &P).parse(input, state)
    }
    #[inline(always)]
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<(), E>) {
        (&**self as &P).check(input, state)
    }
    #[inline(always)]
    fn hint(&self) -> ParseHint {
        <P as Parser<'src, I, A, E>>::hint(self as &P)
    }
}

impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, A, E> for Arc<P>
where
    P: Parser<'src, I, A, E> + ?Sized,
{
    #[inline(always)]
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<A, E>) {
        (&**self as &P).parse(input, state)
    }
    #[inline(always)]
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<(), E>) {
        (&**self as &P).check(input, state)
    }
    #[inline(always)]
    fn hint(&self) -> ParseHint {
        <P as Parser<'src, I, A, E>>::hint(self as &P)
    }
}

impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, A, E> for Rc<P>
where
    P: Parser<'src, I, A, E> + ?Sized,
{
    #[inline(always)]
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<A, E>) {
        (&**self as &P).parse(input, state)
    }
    #[inline(always)]
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<(), E>) {
        (&**self as &P).check(input, state)
    }
    #[inline(always)]
    fn hint(&self) -> ParseHint {
        <P as Parser<'src, I, A, E>>::hint(self as &P)
    }
}

/// Parser combinator wrapper that restricts the inner parser to use the specified [`Parser<'src, I, A, E>`] instance.
/// Usually used to wrap leaf combinators where type information otherwise cannot be propagated.
#[repr(transparent)]
pub struct As<'src, I: ?Sized, A, E, P> {
    /// The inner parser.
    pub inner: P,
    _marker: PhantomData<fn(&'src I, A, E)>,
}
impl<'src, I: ?Sized, A, E, P: Debug> Debug for As<'src, I, A, E, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsParser")
            .field("inner", &self.inner)
            .field("_marker", &self._marker)
            .finish()
    }
}
impl<'src, I: ?Sized, A, E, P: Clone> Clone for As<'src, I, A, E, P> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _marker: self._marker,
        }
    }
}
impl<'src, I: ?Sized, A, E, P: Copy> Copy for As<'src, I, A, E, P> {}
impl<'src, I: ?Sized, A, E, P: PartialEq> PartialEq for As<'src, I, A, E, P> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner && self._marker == other._marker
    }
}
impl<'src, I: ?Sized, A, E, P: Eq> Eq for As<'src, I, A, E, P> {}
impl<'src, I: ?Sized, A, E, P: Hash> Hash for As<'src, I, A, E, P> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
        self._marker.hash(state);
    }
}

impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> From<P> for As<'src, I, A, E, P>
where
    P: Parser<'src, I, A, E>,
{
    fn from(value: P) -> Self {
        As {
            inner: value,
            _marker: PhantomData,
        }
    }
}
impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> As<'src, I, A, E, P>
where
    P: Parser<'src, I, A, E>,
{
    /// Constructs a new [`As`] parser. Defers to [`Self::from`].
    pub fn new(value: P) -> Self {
        As::from(value)
    }
}

impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, A, E>
    for As<'src, I, A, E, P>
where
    P: Parser<'src, I, A, E>,
{
    #[inline(always)]
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<A, E>) {
        self.inner.parse(input, state)
    }
    #[inline(always)]
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<(), E>) {
        self.inner.check(input, state)
    }
    #[inline(always)]
    fn hint(&self) -> ParseHint {
        self.inner.hint()
    }
}
