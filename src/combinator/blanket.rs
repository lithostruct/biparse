use std::{fmt::Debug, hash::Hash, marker::PhantomData, rc::Rc, sync::Arc};

use derive_more::{From, Into};

use crate::{
    input::Input,
    parse::{ParseEnvironment, ParseOutput, ParseState, Parser},
};

impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, A, E> for &P
where
    P: Parser<'src, I, A, E> + ?Sized,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (ParseState<'src, I::Cursor, E::Supply>, ParseOutput<A, E>) {
        <P as Parser<'src, I, A, E>>::parse(*self, input, state)
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (ParseState<'src, I::Cursor, E::Supply>, ParseOutput<(), E>) {
        <P as Parser<'src, I, A, E>>::check(*self, input, state)
    }
}

impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, A, E> for Box<P>
where
    P: Parser<'src, I, A, E> + ?Sized,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (ParseState<'src, I::Cursor, E::Supply>, ParseOutput<A, E>) {
        (&**self as &P).parse(input, state)
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (ParseState<'src, I::Cursor, E::Supply>, ParseOutput<(), E>) {
        (&**self as &P).check(input, state)
    }
}

impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, A, E> for Arc<P>
where
    P: Parser<'src, I, A, E> + ?Sized,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (ParseState<'src, I::Cursor, E::Supply>, ParseOutput<A, E>) {
        (&**self as &P).parse(input, state)
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (ParseState<'src, I::Cursor, E::Supply>, ParseOutput<(), E>) {
        (&**self as &P).check(input, state)
    }
}

impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, A, E> for Rc<P>
where
    P: Parser<'src, I, A, E> + ?Sized,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (ParseState<'src, I::Cursor, E::Supply>, ParseOutput<A, E>) {
        (&**self as &P).parse(input, state)
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (ParseState<'src, I::Cursor, E::Supply>, ParseOutput<(), E>) {
        (&**self as &P).check(input, state)
    }
}

/// Parser combinator wrapper that allows for a parser combinator to carry its associated [`ParseEnvironment`].
#[derive(From, Into)]
#[repr(transparent)]
pub struct WithEnv<E, P> {
    #[from]
    pub inner: P,
    _marker: PhantomData<fn(E)>,
}
impl<E, P: Debug> Debug for WithEnv<E, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WithEnv")
            .field("inner", &self.inner)
            .field("_marker", &self._marker)
            .finish()
    }
}
impl<E, P: Clone> Clone for WithEnv<E, P> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone(), _marker: self._marker.clone() }
    }
}
impl<E, P: Copy> Copy for WithEnv<E, P> {}
impl<E, P: PartialEq> PartialEq for WithEnv<E, P> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner && self._marker == other._marker
    }
}
impl<E, P: Eq> Eq for WithEnv<E, P> {}
impl<E, P: Hash> Hash for WithEnv<E, P> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
        self._marker.hash(state);
    }
}

impl<E: ParseEnvironment, P> WithEnv<E, P> {
    pub fn new<'src, I: Input + ?Sized, A>(inner: P) -> Self
    where
        P: Parser<'src, I, A, E>,
    {
        WithEnv {
            inner,
            _marker: PhantomData,
        }
    }
}
impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, A, E> for WithEnv<E, P>
where
    P: Parser<'src, I, A, E>,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (ParseState<'src, I::Cursor, E::Supply>, ParseOutput<A, E>) {
        self.inner.parse(input, state)
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (ParseState<'src, I::Cursor, E::Supply>, ParseOutput<(), E>) {
        self.inner.check(input, state)
    }
}

/// Parser combinator wrapper that restricts the inner parser to use the specified [`Parser<'src, I, A, E>`] instance.
#[repr(transparent)]
pub struct AsParser<'src, I: ?Sized, A, E, P> {
    pub inner: P,
    _marker: PhantomData<fn(&'src I, A, E)>,
}
impl<'src, I: ?Sized, A, E, P: Debug> Debug for AsParser<'src, I, A, E, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsParser")
            .field("inner", &self.inner)
            .field("_marker", &self._marker)
            .finish()
    }
}
impl<'src, I: ?Sized, A, E, P: Clone> Clone for AsParser<'src, I, A, E, P> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone(), _marker: self._marker.clone() }
    }
}
impl<'src, I: ?Sized, A, E, P: Copy> Copy for AsParser<'src, I, A, E, P> {}
impl<'src, I: ?Sized, A, E, P: PartialEq> PartialEq for AsParser<'src, I, A, E, P> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner && self._marker == other._marker
    }
}
impl<'src, I: ?Sized, A, E, P: Eq> Eq for AsParser<'src, I, A, E, P> {}
impl<'src, I: ?Sized, A, E, P: Hash> Hash for AsParser<'src, I, A, E, P> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
        self._marker.hash(state);
    }
}

impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> From<P> for AsParser<'src, I, A, E, P>
where
    P: Parser<'src, I, A, E>,
{
    fn from(value: P) -> Self {
        AsParser {
            inner: value,
            _marker: PhantomData,
        }
    }
}
impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, A, E>
    for AsParser<'src, I, A, E, P>
where
    P: Parser<'src, I, A, E>,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>,
    ) -> (
        ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>,
        ParseOutput<A, E>,
    ) {
        self.inner.parse(input, state)
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>,
    ) -> (
        ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>,
        ParseOutput<(), E>,
    ) {
        self.inner.check(input, state)
    }
}
