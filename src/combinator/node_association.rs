use std::marker::PhantomData;

use rowan::SyntaxKind;

use crate::{cst::IdSupply, input::Input, parse::{DefaultParseEnvironment, ParseEnvironment, ParseOutput, ParseResult, ParseState, Parser}};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Node<P> {
    kind: SyntaxKind,
    inner: P,
}
impl<P> Node<P> {
    pub fn new<'src, I: Input + ?Sized, A, E: ParseEnvironment>(kind: SyntaxKind, inner: P) -> Self
    where P : Parser<'src, I, A, E> {
        Node { inner, kind }
    }
}
impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, A, E> for Node<P>
where
    P : Parser<'src, I, A, E>,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (ParseState<'src, I::Cursor, E::Supply>, ParseOutput<A, E>)
    {
        let (state, output) = self.inner.parse(input, state);
        match &output.ast {
            ParseResult::Success(_) => {
                // Take the CST forest and prepend the new predecessor.
                let new_cst = {
                    // Danger zone.
                    let mut interner = state.interner.borrow_mut();
                    output.cst.attach_parent(self.kind, &mut interner)
                };
                (state, ParseOutput {
                    ast: output.ast,
                    cst: new_cst,
                })
            },
            _ => {
                // No point in predecessor creation when the AST output is a (currently unrecovered) failure.
                (state, output)
            }
        }
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (ParseState<'src, I::Cursor, E::Supply>, ParseOutput<(), E>)
    {
        self.inner.check(input, state)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Associate<P, F, A> {
    inner: P,
    closure: F,
    kind: SyntaxKind,
    _marker: PhantomData<fn(A)>
}
impl<P, F, A> Associate<P, F, A> {
    pub fn new<'src, I: Input + ?Sized, E: ParseEnvironment, B>(inner: P, kind: SyntaxKind, closure: F) -> Self
    where Self : Parser<'src, I, B, E> {
        Associate { inner, closure, kind, _marker: PhantomData }
    }
}
impl<'src, I: Input + ?Sized, A, B, E: ParseEnvironment, P, F> Parser<'src, I, B, E> for Associate<P, F, A>
where
    P : Parser<'src, I, A, E>,
    F : Fn(<E::Supply as IdSupply>::Id, A) -> B,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (ParseState<'src, I::Cursor, E::Supply>, ParseOutput<B, E>)
    {
        let (mut state, output) = self.inner.parse(input, state);
        match output.ast {
            ParseResult::Success(payload) => {
                // Take the CST forest and prepend the new predecessor.
                let new_cst = {
                    // Danger zone.
                    let mut interner = state.interner.borrow_mut();
                    output.cst.attach_parent(self.kind, &mut interner)
                };

                // Associate the AST and CST payload via an `Id` taken from the result state.
                let fresh_id = state.supply.next_id();
                let linked_cst = new_cst.associate_root_at(0, fresh_id.clone());
                let new_payload = (self.closure)(fresh_id, payload);

                (state, ParseOutput {
                    ast: ParseResult::Success(new_payload),
                    cst: linked_cst,
                })
            },
            failure => {
                // No point in association when the AST output is a (currently unrecovered) failure.
                (state, ParseOutput {
                    ast: failure.map(|_| unreachable!()),
                    cst: output.cst,
                })
            }
        }
    }
}