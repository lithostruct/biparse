use std::{fmt::Debug, hash::Hash, marker::PhantomData, rc::Rc, sync::Arc};

use crate::{
    input::Input,
    parse::{ParseEnvironment, ParseHint, ParseOutput, ParseState, Parser},
};

impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, A, E> for &P
where
    P: Parser<'src, I, A, E> + ?Sized,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<A, E>) {
        <P as Parser<'src, I, A, E>>::parse(*self, input, state)
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<(), E>) {
        <P as Parser<'src, I, A, E>>::check(*self, input, state)
    }
    fn hint(&self) -> ParseHint {
        <P as Parser<'src, I, A, E>>::hint(self)
    }
}

impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, A, E> for Box<P>
where
    P: Parser<'src, I, A, E> + ?Sized,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<A, E>) {
        (&**self as &P).parse(input, state)
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<(), E>) {
        (&**self as &P).check(input, state)
    }
    fn hint(&self) -> ParseHint {
        <P as Parser<'src, I, A, E>>::hint(self as &P)
    }
}

impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, A, E> for Arc<P>
where
    P: Parser<'src, I, A, E> + ?Sized,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<A, E>) {
        (&**self as &P).parse(input, state)
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<(), E>) {
        (&**self as &P).check(input, state)
    }
    fn hint(&self) -> ParseHint {
        <P as Parser<'src, I, A, E>>::hint(self as &P)
    }
}

impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, A, E> for Rc<P>
where
    P: Parser<'src, I, A, E> + ?Sized,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<A, E>) {
        (&**self as &P).parse(input, state)
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<(), E>) {
        (&**self as &P).check(input, state)
    }
    fn hint(&self) -> ParseHint {
        <P as Parser<'src, I, A, E>>::hint(self as &P)
    }
}

/// Parser combinator wrapper that allows for a parser combinator to carry its associated [`ParseEnvironment`].
/// Used to wrap combinators, where the associated `ParseEnvironment` information is likely to be lost for type inference.
///
/// See [`As<'src, I, A, E>`] for a stronger typed variant.
#[repr(transparent)]
pub struct With<E, P> {
    pub inner: P,
    _marker: PhantomData<fn(E)>,
}
impl<E, P: Debug> Debug for With<E, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WithEnv")
            .field("inner", &self.inner)
            .field("_marker", &self._marker)
            .finish()
    }
}
impl<E, P: Clone> Clone for With<E, P> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _marker: self._marker,
        }
    }
}
impl<E, P: Copy> Copy for With<E, P> {}
impl<E, P: PartialEq> PartialEq for With<E, P> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner && self._marker == other._marker
    }
}
impl<E, P: Eq> Eq for With<E, P> {}
impl<E, P: Hash> Hash for With<E, P> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
        self._marker.hash(state);
    }
}

impl<E: ParseEnvironment, P> With<E, P> {
    pub fn new<'src, I: Input + ?Sized, A>(inner: P) -> Self
    where
        P: Parser<'src, I, A, E>,
    {
        With {
            inner,
            _marker: PhantomData,
        }
    }
}
impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, A, E> for With<E, P>
where
    P: Parser<'src, I, A, E>,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<A, E>) {
        self.inner.parse(input, state)
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<(), E>) {
        self.inner.check(input, state)
    }
    fn hint(&self) -> ParseHint {
        self.inner.hint()
    }
}

/// Parser combinator wrapper that restricts the inner parser to use the specified [`Parser<'src, I, A, E>`] instance.
/// Usually used to wrap leaf combinators where type information otherwise cannot be propagated.
///
/// See [`With`] for a weaker version that only requires proof of the [`Parser`] instance at construction, then forgets it.
#[repr(transparent)]
pub struct As<'src, I: ?Sized, A, E, P> {
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
impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, A, E>
    for As<'src, I, A, E, P>
where
    P: Parser<'src, I, A, E>,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<A, E>) {
        self.inner.parse(input, state)
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<(), E>) {
        self.inner.check(input, state)
    }
    fn hint(&self) -> ParseHint {
        self.inner.hint()
    }
}
