use std::marker::PhantomData;

use crate::{
    failure::ParseFailure,
    input::Input,
    parse::{ParseEnvironment, ParseHint, ParseOutcome, ParseOutput, ParseState, Parser},
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
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<B, E>) {
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
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<(), E>) {
        self.inner.check(input, state)
    }
    fn hint(&self) -> ParseHint {
        self.inner.hint()
    }
}

/// Parser combinator that calls [`ToOwned::to_owned`] onto its payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntoOwned<P, A: ?Sized> {
    inner: P,
    _marker: PhantomData<fn(A)>,
}
impl<P, A: ?Sized> IntoOwned<P, A> {
    pub fn new<'src, I: Input + ?Sized, E: ParseEnvironment>(inner: P) -> Self
    where
        P: Parser<'src, I, &'src A, E>,
        A: ToOwned + 'src,
    {
        IntoOwned {
            inner,
            _marker: PhantomData,
        }
    }
}
impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, A::Owned, E>
    for IntoOwned<P, A>
where
    P: Parser<'src, I, &'src A, E>,
    A: ToOwned + ?Sized + 'src,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<A::Owned, E>) {
        let (state, output) = self.inner.parse(input, state);
        (
            state,
            ParseOutput {
                ast: output.ast.map(|inner| (*inner).to_owned()),
                cst: output.cst,
            },
        )
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

/// Parser combinator for fallible mapping successful AST payloads.
///
/// If conversion fails, signal a committed failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TryMap<P, F, A> {
    inner: P,
    closure: F,
    _marker: PhantomData<fn(A)>,
}
impl<P, F, A> TryMap<P, F, A> {
    pub fn new<'src, I: Input + ?Sized, B, E: ParseEnvironment, Error>(inner: P, closure: F) -> Self
    where
        P: Parser<'src, I, A, E>,
        F: Fn(A) -> Result<B, Error>,
        Error: std::error::Error + 'static,
    {
        TryMap {
            inner,
            closure,
            _marker: PhantomData,
        }
    }
}
impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P, F, B, Error> Parser<'src, I, B, E>
    for TryMap<P, F, A>
where
    P: Parser<'src, I, A, E>,
    F: Fn(A) -> Result<B, Error>,
    Error: std::error::Error + 'static,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<B, E>) {
        let (state, output) = self.inner.parse(input, state);
        let new_ast = match output.ast {
            ParseOutcome::Success(success) => match (self.closure)(success) {
                Ok(payload) => ParseOutcome::Success(payload),
                Err(e) => {
                    ParseOutcome::Committed(E::Failure::custom_failure_at(input, state.cursor, e))
                }
            },
            ParseOutcome::Speculative(_) | ParseOutcome::Committed(_) | ParseOutcome::Fatal(_) => {
                output.ast.map(|_| unreachable!() as B)
            }
        };
        (
            state,
            ParseOutput {
                ast: new_ast,
                cst: output.cst,
            },
        )
    }

    /// This is the default implementation for `check`;
    /// we need to actualize the AST value because subsequent outcome semantics depend on whether the closure fails.
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<(), E>) {
        let (new_state, output) = <Self as Parser<'src, I, B, E>>::parse(self, input, state);
        let erased_output = ParseOutput {
            ast: output.ast.map(|_| ()),
            cst: output.cst,
        };
        (new_state, erased_output)
    }

    fn hint(&self) -> ParseHint {
        self.inner.hint()
    }
}
