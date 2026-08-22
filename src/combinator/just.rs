use rowan::SyntaxKind;

use crate::{
    cst::CstOutput,
    failure::{FailureKind, ParseFailure},
    input::{Input, Pattern},
    parse::{ParseEnvironment, ParseOutput, ParseResult, ParseState, Parser},
};

/// Matches on a pattern compatible with the input type.
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
impl<'src, I: Input + ?Sized, P, E: ParseEnvironment> Parser<'src, I, &'src str, E> for Just<P>
where
    P: Pattern<I>,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E::Supply>,
    ) -> (ParseState<'src, I, E::Supply>, ParseOutput<&'src str, E>) {
        match self.pattern.matches_at(input, state.cursor) {
            Ok(end_position) => {
                let old_position = state.cursor;
                let substring = input.slice(old_position, end_position);
                let element = state.intern_token(self.kind, substring).into();

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

    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E::Supply>,
    ) -> (ParseState<'src, I, E::Supply>, ParseOutput<(), E>) {
        match self.pattern.matches_at(input, state.cursor) {
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
