use std::{marker::PhantomData, rc::Rc, sync::Arc};

use derive_more::{From, Into};

use crate::{
    input::Input,
    parse::{ParseEnvironment, ParseOutput, ParseState, Parser},
};

impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, A, E> for &P
where
    P: Parser<'src, I, A, E>,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E::Supply>,
    ) -> (ParseState<'src, I, E::Supply>, ParseOutput<A, E>) {
        (**self).parse(input, state)
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E::Supply>,
    ) -> (ParseState<'src, I, E::Supply>, ParseOutput<(), E>) {
        (**self).check(input, state)
    }
}

impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, A, E> for Box<P>
where
    P: Parser<'src, I, A, E>,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E::Supply>,
    ) -> (ParseState<'src, I, E::Supply>, ParseOutput<A, E>) {
        (**self).parse(input, state)
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E::Supply>,
    ) -> (ParseState<'src, I, E::Supply>, ParseOutput<(), E>) {
        (**self).check(input, state)
    }
}

impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, A, E> for Arc<P>
where
    P: Parser<'src, I, A, E>,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E::Supply>,
    ) -> (ParseState<'src, I, E::Supply>, ParseOutput<A, E>) {
        (**self).parse(input, state)
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E::Supply>,
    ) -> (ParseState<'src, I, E::Supply>, ParseOutput<(), E>) {
        (**self).check(input, state)
    }
}

impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, A, E> for Rc<P>
where
    P: Parser<'src, I, A, E>,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E::Supply>,
    ) -> (ParseState<'src, I, E::Supply>, ParseOutput<A, E>) {
        (**self).parse(input, state)
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E::Supply>,
    ) -> (ParseState<'src, I, E::Supply>, ParseOutput<(), E>) {
        (**self).check(input, state)
    }
}

/// Minimal wrapper that allows for a parser combinator to carry its associated `ParseEnvironment`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, From, Into)]
#[repr(transparent)]
pub struct WithEnv<E, P> {
    #[from]
    pub inner: P,
    _marker: PhantomData<fn(E)>
}
impl<E: ParseEnvironment, P> WithEnv<E, P> {
    pub fn new<'src, I: Input + ?Sized, A>(inner: P) -> Self
    where P: Parser<'src, I, A, E> {
        WithEnv { inner, _marker: PhantomData }
    }
}
impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, A, E> for WithEnv<E, P>
where P : Parser<'src, I, A, E> {
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, <E as ParseEnvironment>::Supply>,
    ) -> (ParseState<'src, I, <E as ParseEnvironment>::Supply>, ParseOutput<A, E>)
    {
        self.inner.parse(input, state)
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, <E as ParseEnvironment>::Supply>,
    ) -> (ParseState<'src, I, <E as ParseEnvironment>::Supply>, ParseOutput<(), E>)
    {
        self.inner.check(input, state)
    }
}