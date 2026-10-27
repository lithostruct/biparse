use std::marker::PhantomData;

use rowan::SyntaxKind;

use crate::{
    cst::IdSupply,
    failure::ParseFailure,
    input::Input,
    parse::{ParseEnvironment, ParseHint, ParseOutcome, ParseOutput, ParseState, Parser},
};

/// Combinator that transforms an upstream speculative failure into a committed failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Commit<P>(P);
impl<P> Commit<P> {
    pub fn new<'src, I: Input + ?Sized, A, E: ParseEnvironment>(parser: P) -> Self
    where
        P: Parser<'src, I, A, E>,
    {
        Commit(parser)
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
        let (state, output) = self.0.parse(input, state);
        match output.ast {
            ParseOutcome::Success(_) | ParseOutcome::Committed(_) => (state, output),
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
        let (state, output) = self.0.check(input, state);
        match output.ast {
            ParseOutcome::Success(_) | ParseOutcome::Committed(_) => (state, output),
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
        self.0.hint()
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
    fallible: F,
    recovery: R,
    closure: C,
    kind: SyntaxKind,
    _marker: PhantomData<fn(B)>,
}
impl<F, R, B, C> Recover<F, R, B, C> {
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
            ParseOutcome::Success(_) | ParseOutcome::Speculative(_) => return (state, output),
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
                        cst: recovered_cst,
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
        }
    }
    fn hint(&self) -> ParseHint {
        self.fallible.hint()
    }
}
