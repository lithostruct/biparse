use std::marker::PhantomData;

use crate::{
    input::Input,
    parse::{ParseEnvironment, ParseOutput, ParseState, Parser},
};

/// Parser combinator for mapping successful AST payloads via the given closure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Map<P, F, A> {
    inner: P,
    closure: F,
    _marker: PhantomData<fn(A)>,
}
impl<P, F, A> Map<P, F, A> {
    pub fn new<'src, I: Input + ?Sized, B, E: ParseEnvironment>(inner: P, closure: F) -> Self
    where
        P: Parser<'src, I, A, E>,
        F: Fn(A) -> B,
    {
        Map {
            inner,
            closure,
            _marker: PhantomData,
        }
    }
}
impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P, F, B> Parser<'src, I, B, E>
    for Map<P, F, A>
where
    P: Parser<'src, I, A, E>,
    F: Fn(A) -> B,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (ParseState<'src, I::Cursor, E::Supply>, ParseOutput<B, E>) {
        let (state, output) = self.inner.parse(input, state);
        (
            state,
            ParseOutput {
                ast: output.ast.map(&self.closure),
                cst: output.cst,
            },
        )
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (ParseState<'src, I::Cursor, E::Supply>, ParseOutput<(), E>) {
        self.inner.check(input, state)
    }
}

/// Parser combinator that calls [`ToOwned::to_owned`](std::borrow::ToOwned::to_owned) onto its payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToOwned<P, A> {
    inner: P,
    _marker: PhantomData<fn(A)>,
}
impl<P, A> ToOwned<P, A> {
    pub fn new<'src, I: Input + ?Sized, E: ParseEnvironment>(inner: P) -> Self
    where
        P: Parser<'src, I, A, E>,
        A: std::borrow::ToOwned + 'src,
    {
        ToOwned {
            inner,
            _marker: PhantomData,
        }
    }
}
impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, A::Owned, E>
    for ToOwned<P, A>
where
    P: Parser<'src, I, A, E>,
    A: std::borrow::ToOwned + 'src,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (
        ParseState<'src, I::Cursor, E::Supply>,
        ParseOutput<A::Owned, E>,
    ) {
        let (state, output) = self.inner.parse(input, state);
        (
            state,
            ParseOutput {
                ast: output
                    .ast
                    .map(|inner| std::borrow::ToOwned::to_owned(&inner)),
                cst: output.cst,
            },
        )
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (ParseState<'src, I::Cursor, E::Supply>, ParseOutput<(), E>) {
        self.inner.check(input, state)
    }
}
