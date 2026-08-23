use std::{marker::PhantomData, rc::Rc, sync::Arc};

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

/// Parser combinator wrapper that allows for a parser combinator to carry its associated `ParseEnvironment`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, From, Into)]
#[repr(transparent)]
pub struct WithEnv<E, P> {
    #[from]
    pub inner: P,
    _marker: PhantomData<fn(E)>,
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
    ) -> (
        ParseState<'src, I::Cursor, E::Supply>,
        ParseOutput<A, E>,
    ) {
        self.inner.parse(input, state)
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (
        ParseState<'src, I::Cursor, E::Supply>,
        ParseOutput<(), E>,
    ) {
        self.inner.check(input, state)
    }
}
