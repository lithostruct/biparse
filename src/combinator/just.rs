use std::fmt::Debug;

use crate::{
    cst::AssociatedGreenCst,
    failure::{ParseFailure, ParseFailureKind},
    input::{Input, Pattern},
    parse::{ParseEnvironment, ParseHint, ParseOutcome, ParseOutput, ParseState, Parser},
};

/// Parser combinator that matches on a pattern compatible with the input type.
/// `P` should implement `Pattern<I>` for some `I : Input` for this combinator to be meaningful.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Just<P> {
    pattern: P,
}
impl<P> Just<P> {
    pub fn new<'src, I>(pattern: P) -> Self
    where
        P: Pattern<'src, I>,
        I: Input + ?Sized + 'src,
    {
        Just { pattern }
    }
}
impl<'src, I: Input + ?Sized, P, E: ParseEnvironment> Parser<'src, I, P::Payload, E> for Just<P>
where
    P: Pattern<'src, I> + 'src,
{
    /// Actualizes skipped trivia up to the next cursor.
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (
        ParseState<'src, I::Cursor, E::Supply>,
        ParseOutput<P::Payload, E>,
    ) {
        match self.pattern.matches_at(input, state.cursor) {
            Ok(matched) => {
                let new_state = ParseState {
                    cursor: matched.next_cursor,
                    ..state
                };
                let new_cst = new_state.with_interner(|interner| {
                    AssociatedGreenCst::leaves(
                        &matched.cst_spans,
                        state.cursor,
                        matched.next_cursor,
                        input,
                        interner,
                    )
                });
                (
                    new_state,
                    ParseOutput {
                        ast: ParseOutcome::Success(matched.payload),
                        cst: new_cst,
                    },
                )
            }
            Err(error_position) => {
                let failure = E::Failure::failure_at(
                    input,
                    state.cursor,
                    ParseFailureKind::ExpectedFound {
                        hint: self.pattern.hint(),
                        difference_at: error_position,
                    },
                );
                (
                    state,
                    ParseOutput {
                        ast: ParseOutcome::Speculative(failure),
                        cst: AssociatedGreenCst::new(),
                    },
                )
            }
        }
    }

    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (ParseState<'src, I::Cursor, E::Supply>, ParseOutput<(), E>) {
        match self.pattern.matches_at(input, state.cursor) {
            Ok(matched) => {
                let new_state = ParseState {
                    cursor: matched.next_cursor,
                    ..state
                };
                (
                    new_state,
                    ParseOutput {
                        ast: ParseOutcome::Success(()),
                        cst: AssociatedGreenCst::new(),
                    },
                )
            }
            Err(error_position) => {
                let failure = E::Failure::failure_at(
                    input,
                    state.cursor,
                    ParseFailureKind::ExpectedFound {
                        hint: self.pattern.hint(),
                        difference_at: error_position,
                    },
                );
                (
                    state,
                    ParseOutput {
                        ast: ParseOutcome::Speculative(failure),
                        cst: AssociatedGreenCst::new(),
                    },
                )
            }
        }
    }

    fn hint(&self) -> ParseHint {
        self.pattern.hint()
    }
}
