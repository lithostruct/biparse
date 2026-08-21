use rowan::SyntaxKind;

use crate::{
    cst::{CstOutput, IdSupply},
    failure::{FailureKind, ParseFailure},
    input::{Input, Pattern},
    parse::{ParseEnvironment, ParseOutput, ParseResult, ParseState, Parser},
};
use std::fmt::Debug;

/// Consumes no input and succeeds.
///
/// The zero identity for sequential operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Empty;
impl<'src, I: Input, E: ParseEnvironment> Parser<'src, I, (), E> for Empty {
    fn parse<S: IdSupply<Id = <E as ParseEnvironment>::Id>>(
        &self,
        _input: &'src I,
        state: ParseState<I, S>,
    ) -> (
        ParseState<I, S>,
        ParseOutput<(), <E as ParseEnvironment>::Failure, <E as ParseEnvironment>::Id>,
    ) {
        (
            state,
            ParseOutput {
                ast: ParseResult::Success(()),
                cst: CstOutput::new(),
            },
        )
    }
    fn check<S: IdSupply<Id = <E as ParseEnvironment>::Id>>(
        &self,
        input: &'src I,
        state: ParseState<I, S>,
    ) -> (
        ParseState<I, S>,
        ParseOutput<(), <E as ParseEnvironment>::Failure, <E as ParseEnvironment>::Id>,
    ) {
        Parser::<'src, I, (), E>::parse(self, input, state)
    }
}

/// Consumes no input and produces a speculative failure.
///
/// The zero identity for choices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fail;
impl<'src, I: Input, E: ParseEnvironment> Parser<'src, I, (), E> for Fail {
    fn parse<S: IdSupply<Id = <E as ParseEnvironment>::Id>>(
        &self,
        input: &'src I,
        state: ParseState<I, S>,
    ) -> (
        ParseState<I, S>,
        ParseOutput<(), <E as ParseEnvironment>::Failure, <E as ParseEnvironment>::Id>,
    ) {
        let failure = E::Failure::failure_at(
            input,
            state.cursor,
            FailureKind::FailPrimitive::<I, E::Failure>,
        );
        (
            state,
            ParseOutput {
                ast: ParseResult::Speculative(failure),
                cst: CstOutput::new(),
            },
        )
    }

    fn check<S: IdSupply<Id = <E as ParseEnvironment>::Id>>(
        &self,
        input: &'src I,
        state: ParseState<I, S>,
    ) -> (
        ParseState<I, S>,
        ParseOutput<(), <E as ParseEnvironment>::Failure, <E as ParseEnvironment>::Id>,
    ) {
        Parser::<'src, I, (), E>::parse(self, input, state)
    }
}

/// Matches on a pattern compatible with the input type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Just<P> {
    kind: SyntaxKind,
    pattern: P,
}
impl<'src, I: Input, P, E: ParseEnvironment> Parser<'src, I, &'src str, E> for Just<P>
where
    P: Pattern<I> + Debug,
{
    fn parse<S: IdSupply<Id = <E as ParseEnvironment>::Id>>(
        &self,
        input: &'src I,
        mut state: ParseState<I, S>,
    ) -> (
        ParseState<I, S>,
        ParseOutput<&'src str, <E as ParseEnvironment>::Failure, <E as ParseEnvironment>::Id>,
    ) {
        match self.pattern.matches_at(input, state.cursor) {
            Ok(end_position) => {
                let old_position = state.cursor;
                let substring = input.slice(old_position, end_position);
                let element = state.interner.token(self.kind, substring).into();

                let new_state = ParseState {
                    cursor: end_position,
                    ..state
                };
                (
                    new_state,
                    ParseOutput {
                        ast: ParseResult::Success(substring),
                        cst: CstOutput::unit(element),
                    },
                )
            }
            Err(error_position) => {
                let peek_ahead = input.peek(error_position);
                let failure = E::Failure::failure_at::<_, E::Failure>(
                    input,
                    state.cursor,
                    FailureKind::ExpectedFound {
                        expected: self.pattern.describe(),
                        found: peek_ahead.map(|item| item.token),
                        difference_at: error_position,
                    },
                );
                (
                    state,
                    ParseOutput {
                        ast: ParseResult::Speculative(failure),
                        cst: CstOutput::new(),
                    },
                )
            }
        }
    }

    fn check<S: IdSupply<Id = <E as ParseEnvironment>::Id>>(
        &self,
        input: &'src I,
        state: ParseState<I, S>,
    ) -> (
        ParseState<I, S>,
        ParseOutput<(), <E as ParseEnvironment>::Failure, <E as ParseEnvironment>::Id>,
    ) {
        match self.pattern.matches_at(input, state.cursor.clone()) {
            Ok(end_position) => {
                let new_state = ParseState {
                    cursor: end_position,
                    ..state
                };
                (
                    new_state,
                    ParseOutput {
                        ast: ParseResult::Success(()),
                        cst: CstOutput::new(),
                    },
                )
            }
            Err(error_position) => {
                let peek_ahead = input.peek(error_position);
                let failure = E::Failure::failure_at::<_, E::Failure>(
                    input,
                    state.cursor,
                    FailureKind::ExpectedFound {
                        expected: self.pattern.describe(),
                        found: peek_ahead.map(|item| item.token),
                        difference_at: error_position,
                    },
                );
                (
                    state,
                    ParseOutput {
                        ast: ParseResult::Speculative(failure),
                        cst: CstOutput::new(),
                    },
                )
            }
        }
    }
}
