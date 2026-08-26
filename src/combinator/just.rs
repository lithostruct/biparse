use rowan::SyntaxKind;

use crate::{
    cst::CstOutput,
    failure::{FailureKind, ParseFailure},
    input::{Input, Pattern},
    parse::{ParseEnvironment, ParseOutput, ParseResult, ParseState, Parser},
};

/// Parser combinator that matches on a pattern compatible with the input type.
/// `P` should implement `Pattern<I>` for some `I : Input` for this combinator to be meaningful.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Just<P> {
    kind: SyntaxKind,
    pattern: P,
}
impl<P> Just<P> {
    pub fn new<I>(kind: SyntaxKind, pattern: P) -> Self
    where
        P: Pattern<I>,
        I: Input + ?Sized,
    {
        Just { kind, pattern }
    }
}
impl<'src, I: Input + ?Sized, P, E: ParseEnvironment> Parser<'src, I, P::Payload<'src>, E> for Just<P>
where
    P: Pattern<I> + 'src,
{
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
                let old_position = state.cursor;
                let substring = input.slice(old_position, matched.advanced_cursor);
                let element = state.intern_token(self.kind, &substring).into();

                let new_state = ParseState {
                    cursor: matched.advanced_cursor,
                    ..state
                };
                (
                    new_state,
                    ParseOutput {
                        ast: ParseResult::Success(matched.payload),
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

    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (ParseState<'src, I::Cursor, E::Supply>, ParseOutput<(), E>) {
        match self.pattern.matches_at(input, state.cursor) {
            Ok(matched) => {
                let new_state = ParseState {
                    cursor: matched.advanced_cursor,
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
