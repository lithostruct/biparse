use std::{borrow::Cow, marker::PhantomData};

use rowan::SyntaxKind;

use crate::{
    cst::IdSupply,
    input::Input,
    parse::{ParseEnvironment, ParseHint, ParseOutcome, ParseOutput, ParseState, Parser},
};

/// Parser combinator for appending a CST node as a parent to an existing green forest.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Node<P> {
    /// The [`SyntaxKind`] of the CST node, which takes the existing forest as its children.
    pub kind: SyntaxKind,
    /// Optional name for this parser.
    ///
    /// Used for better parse errors and diagnostics.
    pub name: Option<Cow<'static, str>>,
    /// The inner parser.
    pub inner: P,
}
impl<P> Node<P> {
    /// Constructs a new [`Node`] parser.
    pub fn new<'src, I: Input + ?Sized, A, E: ParseEnvironment>(kind: SyntaxKind, inner: P) -> Self
    where
        P: Parser<'src, I, A, E>,
    {
        Node {
            inner,
            kind,
            name: None,
        }
    }
    /// Constructs a new [`Node`] parser, with the given name.
    pub fn new_as<'src, I: Input + ?Sized, A, E: ParseEnvironment>(
        name: impl Into<Cow<'static, str>>,
        kind: SyntaxKind,
        inner: P,
    ) -> Self
    where
        P: Parser<'src, I, A, E>,
    {
        Node {
            inner,
            kind,
            name: Some(name.into()),
        }
    }
}
impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, A, E> for Node<P>
where
    P: Parser<'src, I, A, E>,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<A, E>) {
        let (state, output) = self.inner.parse(input, state);
        match &output.ast {
            ParseOutcome::Success(_) => {
                // Take the CST forest and prepend the new predecessor.
                let new_cst =
                    state.with_interner(|interner| output.cst.attach_parent(self.kind, interner));
                (
                    state,
                    ParseOutput {
                        ast: output.ast,
                        cst: new_cst,
                    },
                )
            }
            _ => {
                // No point in predecessor creation when the AST output is a (currently unrecovered) failure.
                (state, output)
            }
        }
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<(), E>) {
        self.inner.check(input, state)
    }
    fn hint(&self) -> ParseHint {
        match &self.name {
            Some(name) => ParseHint::Named {
                name: name.clone().into_owned(),
                inner: Box::new(self.inner.hint()),
            },
            None => self.inner.hint(),
        }
    }
}

/// Parser combinator for the association of AST and CST trees via [`IdSupply::Id`] correspondence.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Associate<P, F, A> {
    /// The inner parser.
    pub inner: P,
    /// The closure receiving an `Id` from `ParseEnvironment::IdSupply` and the inner parser,
    /// returning a payload of type `A`.
    pub closure: F,
    /// The [`SyntaxKind`] of the CST node, which takes the existing forest as its children.
    pub kind: SyntaxKind,
    /// Optional name for this parser.
    ///
    /// Used for better parse errors and diagnostics.
    pub name: Option<Cow<'static, str>>,
    _marker: PhantomData<fn(A)>,
}
impl<P, F, A> Associate<P, F, A> {
    /// Constructs a new [`Associate`] parser.
    pub fn new<'src, I: Input + ?Sized, E: ParseEnvironment, B>(
        inner: P,
        kind: SyntaxKind,
        closure: F,
    ) -> Self
    where
        Self: Parser<'src, I, B, E>,
    {
        Associate {
            inner,
            closure,
            kind,
            name: None,
            _marker: PhantomData,
        }
    }
    /// Constructs a new [`Associate`] parser, with the given name.
    pub fn new_as<'src, I: Input + ?Sized, E: ParseEnvironment, B>(
        name: impl Into<Cow<'static, str>>,
        inner: P,
        kind: SyntaxKind,
        closure: F,
    ) -> Self
    where
        Self: Parser<'src, I, B, E>,
    {
        Associate {
            inner,
            closure,
            kind,
            name: Some(name.into()),
            _marker: PhantomData,
        }
    }
}
impl<'src, I: Input + ?Sized, A, B, E: ParseEnvironment, P, F> Parser<'src, I, B, E>
    for Associate<P, F, A>
where
    P: Parser<'src, I, A, E>,
    F: Fn(<E::Supply as IdSupply>::Id, A) -> B,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<B, E>) {
        let (mut state, output) = self.inner.parse(input, state);
        match output.ast {
            ParseOutcome::Success(payload) => {
                let fresh_id = state.supply.next_id();
                // Take the CST forest and prepend + associate the new parent.
                let new_cst = state.with_interner(|interner| {
                    output
                        .cst
                        .associate_new_parent(self.kind, fresh_id.clone(), interner)
                });

                let new_payload = (self.closure)(fresh_id, payload);

                (
                    state,
                    ParseOutput {
                        ast: ParseOutcome::Success(new_payload),
                        cst: new_cst,
                    },
                )
            }
            failure => {
                // No point in association when the AST output is a (currently unrecovered) failure.
                (
                    state,
                    ParseOutput {
                        ast: failure.map(|_| unreachable!()),
                        cst: output.cst,
                    },
                )
            }
        }
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<(), E>) {
        self.inner.check(input, state)
    }
    fn hint(&self) -> ParseHint {
        match &self.name {
            Some(name) => ParseHint::Named {
                name: name.clone().into_owned(),
                inner: Box::new(self.inner.hint()),
            },
            None => self.inner.hint(),
        }
    }
}
