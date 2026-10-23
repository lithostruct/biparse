use std::{cmp::Ordering, fmt::Debug, marker::PhantomData};

use rowan::SyntaxKind;

use crate::{
    cst::{AssociatedGreenCst, Cst},
    failure::{ParseFailure, ParseFailureKind},
    green::GreenElement,
    input::{Input, InputItem, Pattern},
    parse::{ParseEnvironment, ParseHint, ParseOutcome, ParseOutput, ParseState, Parser},
};

/// Parser combinator that matches on a pattern compatible with the input type.
/// `P` should implement `Pattern<I>` for some `I : Input` for this combinator to be meaningful.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Just<P> {
    pattern: P,
}
impl<P> Just<P> {
    pub fn new<I>(pattern: P) -> Self
    where
        P: Pattern<I>,
        I: Input + ?Sized,
    {
        Just { pattern }
    }
}
impl<'src, I: Input + ?Sized, P, E: ParseEnvironment> Parser<'src, I, P::Payload<'src>, E>
    for Just<P>
where
    P: Pattern<I> + 'src,
{
    /// Actualizes skipped trivia up to the next cursor.
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (
        ParseState<'src, I::Cursor, E::Supply>,
        ParseOutput<P::Payload<'src>, E>,
    ) {
        match self.pattern.matches_at(input, state.cursor) {
            Ok(matched) => {
                let leaf_cst = matched.actualize_leaves::<I, E>(input, &state);
                let new_state = ParseState {
                    cursor: matched.next_cursor,
                    ..state
                };
                (
                    new_state,
                    ParseOutput {
                        ast: ParseOutcome::Success(matched.payload),
                        cst: AssociatedGreenCst::leaves(leaf_cst),
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
