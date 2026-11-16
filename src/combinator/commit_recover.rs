use std::marker::PhantomData;

use derive_more::{AsMut, AsRef};
use rowan::SyntaxKind;

use crate::{
    cst::IdSupply,
    failure::ParseFailure,
    input::Input,
    parse::{ParseEnvironment, ParseHint, ParseOutcome, ParseOutput, ParseState, Parser},
};

/// Combinator that transforms an upstream speculative failure into a [`ParseOutcome::Committed`] failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, AsRef, AsMut)]
#[non_exhaustive]
pub struct Commit<P> {
    #[as_ref]
    #[as_mut]
    /// The inner parser.
    pub inner: P,
}
impl<P> Commit<P> {
    /// Constructs a new [`Commit`] parser.
    pub fn new<'src, I: Input + ?Sized, A, E: ParseEnvironment>(inner: P) -> Self
    where
        P: Parser<'src, I, A, E>,
    {
        Commit { inner }
    }
}

impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, A, E> for Commit<P>
where
    P: Parser<'src, I, A, E>,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<A, E>) {
        let (state, output) = self.inner.parse(input, state);
        match output.ast {
            ParseOutcome::Success(_) | ParseOutcome::Committed(_) | ParseOutcome::Fatal(_) => {
                (state, output)
            }
            ParseOutcome::Speculative(failure) => (
                state,
                ParseOutput {
                    ast: ParseOutcome::Committed(failure),
                    cst: output.cst,
                },
            ),
        }
    }

    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<(), E>) {
        let (state, output) = self.inner.check(input, state);
        match output.ast {
            ParseOutcome::Success(_) | ParseOutcome::Committed(_) | ParseOutcome::Fatal(_) => {
                (state, output)
            }
            ParseOutcome::Speculative(failure) => (
                state,
                ParseOutput {
                    ast: ParseOutcome::Committed(failure),
                    cst: output.cst,
                },
            ),
        }
    }

    fn hint(&self) -> ParseHint {
        self.inner.hint()
    }
}

/// Combinator that transforms an upstream speculative or committed failure into a [`ParseOutcome::Fatal`] failure.
///
/// *Fatal failures are unrecoverable.* Use this combinator sparingly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, AsRef, AsMut)]
#[non_exhaustive]
pub struct Fatal<P> {
    #[as_ref]
    #[as_mut]
    /// The inner parser.
    pub inner: P,
}
impl<P> Fatal<P> {
    /// Constructs a new [`Fatal`] parser.
    pub fn new<'src, I: Input + ?Sized, A, E: ParseEnvironment>(inner: P) -> Self
    where
        P: Parser<'src, I, A, E>,
    {
        Fatal { inner }
    }
}

impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, A, E> for Fatal<P>
where
    P: Parser<'src, I, A, E>,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<A, E>) {
        let (state, output) = self.inner.parse(input, state);
        match output.ast {
            ParseOutcome::Success(_) | ParseOutcome::Fatal(_) => (state, output),
            ParseOutcome::Speculative(failure) | ParseOutcome::Committed(failure) => (
                state,
                ParseOutput {
                    ast: ParseOutcome::Fatal(failure),
                    cst: output.cst,
                },
            ),
        }
    }

    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<(), E>) {
        let (state, output) = self.inner.check(input, state);
        match output.ast {
            ParseOutcome::Success(_) | ParseOutcome::Fatal(_) => (state, output),
            ParseOutcome::Speculative(failure) | ParseOutcome::Committed(failure) => (
                state,
                ParseOutput {
                    ast: ParseOutcome::Fatal(failure),
                    cst: output.cst,
                },
            ),
        }
    }
    fn hint(&self) -> ParseHint {
        self.inner.hint()
    }
}

/// Combinator that allows for a recovery parser to attempt recovery from committed failure,
/// generating a diagnostic upon successful recovery.
///
/// Does not interact with speculative failure.
///
/// # Note on Recovery Parser Failure
///
/// Speculative or committed failure in the recovery parser surfacing during recovery
/// *will* cause the committed failure to propagate outwards.
/// For this reason, it is recommended to keep the recovery parser as simple as possible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Recover<F, R, B, C> {
    /// The fallible parser, assumed to potentially produce a [`ParseOutcome::Committed`] failure from which to recover from.
    pub fallible: F,
    /// The recovery parser, from which recovery is attempted.
    /// Produces an AST payload of type `B`.
    pub recovery: R,
    /// The recovery closure, receiving an `Id` from [`ParseEnvironment::ErrorSupply`] and a successful recovery parse,
    /// producing the corresponding payload type `A` expected from the fallible parser.
    pub closure: C,
    /// The [`SyntaxKind`] of the CST node constructed upon successful recovery.
    pub kind: SyntaxKind,
    _marker: PhantomData<fn(B)>,
}
impl<F, R, B, C> Recover<F, R, B, C> {
    /// Constructs a new [`Recover`] parser.
    pub fn new<'src, I: Input + ?Sized, A, E: ParseEnvironment>(
        fallible: F,
        recovery: R,
        closure: C,
        kind: impl Into<SyntaxKind>,
    ) -> Self
    where
        F: Parser<'src, I, A, E>,
        R: Parser<'src, I, B, E>,
        C: Fn(<E::ErrorSupply as IdSupply>::Id, B) -> A,
    {
        let kind = kind.into();
        Recover {
            fallible,
            recovery,
            closure,
            kind,
            _marker: PhantomData,
        }
    }
}
impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, B, F, R, C> Parser<'src, I, A, E>
    for Recover<F, R, B, C>
where
    F: Parser<'src, I, A, E>,
    R: Parser<'src, I, B, E>,
    C: Fn(<E::ErrorSupply as IdSupply>::Id, B) -> A,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<A, E>) {
        let (state, output) = self.fallible.parse(input, state);
        // Catching committed failure.
        let failure = match output.ast {
            ParseOutcome::Success(_) | ParseOutcome::Speculative(_) | ParseOutcome::Fatal(_) => {
                // Short-circuit; return.
                return (state, output);
            }
            ParseOutcome::Committed(failure) => failure,
        };

        let (mut recovery_state, recovery_output) = self.recovery.parse(input, state);
        match recovery_output.ast {
            ParseOutcome::Success(recovery_success) => {
                // Recovery success!
                let error_id = recovery_state.error_supply.next_id();
                let recovered_cst = recovery_state.with_interner(|interner| {
                    recovery_output.cst.diagnose_new_parent(
                        self.kind,
                        error_id.clone(),
                        failure,
                        interner,
                    )
                });
                let recovery_payload = (self.closure)(error_id, recovery_success);
                (
                    recovery_state,
                    ParseOutput {
                        ast: ParseOutcome::Success(recovery_payload),
                        cst: output.cst.merge_with(recovered_cst),
                    },
                )
            }
            ParseOutcome::Speculative(recovery_failure)
            | ParseOutcome::Committed(recovery_failure) => {
                // Nested failure!
                (
                    recovery_state,
                    ParseOutput {
                        ast: ParseOutcome::Committed(failure.supplement(recovery_failure)),
                        cst: recovery_output.cst,
                    },
                )
            }
            ParseOutcome::Fatal(recovery_failure) => {
                // Bigger fish to fry. Report this error as principal.
                (
                    recovery_state,
                    ParseOutput {
                        ast: ParseOutcome::Fatal(recovery_failure.supplement(failure)),
                        cst: recovery_output.cst,
                    },
                )
            }
        }
    }
    fn hint(&self) -> ParseHint {
        self.fallible.hint()
    }
}
