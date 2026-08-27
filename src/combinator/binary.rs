use std::fmt::Debug;

use crate::{
    failure::ParseFailure,
    input::Input,
    parse::{ParseEnvironment, ParseOutput, ParseResult, ParseState, Parser},
};
use either::Either;

/// Parser combinator that sequentially executes two parsers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Then<First, Second> {
    first: First,
    second: Second,
}
impl<First, Second> Then<First, Second> {
    pub fn new<'src, I: Input + ?Sized, A, B, E: ParseEnvironment>(
        first: First,
        second: Second,
    ) -> Self
    where
        First: Parser<'src, I, A, E>,
        Second: Parser<'src, I, B, E>,
    {
        Then { first, second }
    }
}
impl<'src, I: Input + ?Sized, First, Second, A, B, E: ParseEnvironment> Parser<'src, I, (A, B), E>
    for Then<First, Second>
where
    First: Parser<'src, I, A, E>,
    Second: Parser<'src, I, B, E>,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (
        ParseState<'src, I::Cursor, E::Supply>,
        ParseOutput<(A, B), E>,
    ) {
        let (state, first_result) = self.first.parse(input, state);
        let payload_a = match first_result.ast {
            ast @ ParseResult::Speculative(_) | ast @ ParseResult::Committed(_) => {
                // Do not continue, exit with error.
                let typed_result = ast.map(|_| unreachable!("In failure!") as (A, B));
                return (
                    state,
                    ParseOutput {
                        ast: typed_result,
                        cst: first_result.cst,
                    },
                );
            }
            ParseResult::Success(payload_a) => payload_a,
        };

        let (state, second_result) = self.second.parse(input, state);
        let cst_union = first_result.cst.merge_with(second_result.cst);
        match second_result.ast {
            ast @ ParseResult::Speculative(_) | ast @ ParseResult::Committed(_) => {
                // Discard payload_a, exit with error.
                let typed_result = ast.map(|_| unreachable!("In failure!") as (A, B));
                (
                    state,
                    ParseOutput {
                        ast: typed_result,
                        cst: cst_union,
                    },
                )
            }
            ParseResult::Success(payload_b) => {
                // Complete success!
                (
                    state,
                    ParseOutput {
                        ast: ParseResult::Success((payload_a, payload_b)),
                        cst: cst_union,
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
        let (state, first_result) = self.first.check(input, state);
        match &first_result.ast {
            ParseResult::Speculative(_) | ParseResult::Committed(_) => {
                return (state, first_result);
            }
            ParseResult::Success(()) => (),
        }
        let (state, second_result) = self.second.check(input, state);
        (state, second_result)
    }
}

/// Parser combinator that returns the first viable alternative, preferring the first result wherever possible.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Or<First, Second> {
    first: First,
    second: Second,
}
impl<First, Second> Or<First, Second> {
    pub fn new<'src, I: Input + ?Sized, A, B, E: ParseEnvironment>(
        first: First,
        second: Second,
    ) -> Self
    where
        First: Parser<'src, I, A, E>,
        Second: Parser<'src, I, B, E>,
    {
        Or { first, second }
    }
}
impl<'src, I: Input + ?Sized, First, Second, A, B, E: ParseEnvironment>
    Parser<'src, I, Either<A, B>, E> for Or<First, Second>
where
    First: Parser<'src, I, A, E>,
    Second: Parser<'src, I, B, E>,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (
        ParseState<'src, I::Cursor, E::Supply>,
        ParseOutput<Either<A, B>, E>,
    ) {
        let (first_state, first_result) = self.first.parse(input, state.clone());
        let first_speculative_error = match first_result.ast {
            ast @ ParseResult::Success(_) | ast @ ParseResult::Committed(_) => {
                // Early termination.
                return (
                    first_state,
                    ParseOutput {
                        ast: ast.map(Either::Left),
                        cst: first_result.cst,
                    },
                );
            }
            // Continue.
            ParseResult::Speculative(error) => error,
        };

        let (second_state, second_result) = self.second.parse(input, state);
        let second_speculative_error = match second_result.ast {
            ast @ ParseResult::Success(_) | ast @ ParseResult::Committed(_) => {
                // Termination, discard first speculative error.
                return (
                    second_state,
                    ParseOutput {
                        ast: ast.map(Either::Right),
                        cst: second_result.cst,
                    },
                );
            }
            // Continue.
            ParseResult::Speculative(error) => error,
        };

        let overall_speculation = first_speculative_error.merge(second_speculative_error);
        (
            second_state,
            ParseOutput {
                ast: ParseResult::Speculative(overall_speculation),
                cst: second_result.cst,
            },
        )
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (ParseState<'src, I::Cursor, E::Supply>, ParseOutput<(), E>) {
        let (first_state, mut first_result) = self.first.check(input, state.clone());
        let first_speculative_error = match first_result.ast {
            ast @ ParseResult::Success(_) | ast @ ParseResult::Committed(_) => {
                // Early termination.
                first_result.ast = ast;
                return (first_state, first_result);
            }
            // Continue.
            ParseResult::Speculative(error) => error,
        };

        let (second_state, mut second_result) = self.second.check(input, state);
        let second_speculative_error = match second_result.ast {
            ast @ ParseResult::Success(_) | ast @ ParseResult::Committed(_) => {
                // Termination, discard first speculative error.
                second_result.ast = ast;
                return (second_state, second_result);
            }
            // Continue.
            ParseResult::Speculative(error) => error,
        };

        let overall_speculation = first_speculative_error.merge(second_speculative_error);
        (
            second_state,
            ParseOutput {
                ast: ParseResult::Speculative(overall_speculation),
                cst: second_result.cst,
            },
        )
    }
}
