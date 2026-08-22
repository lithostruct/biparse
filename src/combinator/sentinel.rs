use rowan::SyntaxKind;

use crate::{
    cst::CstOutput,
    failure::{FailureKind, ParseFailure},
    input::{Input, Pattern},
    parse::{ParseEnvironment, ParseOutput, ParseResult, ParseState, Parser},
};
use std::fmt::Debug;

/// Consumes no input and succeeds.
///
/// The zero identity for sequential operations.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Empty;
impl Empty {
    pub fn new() -> Self {
        Self
    }
}
impl<'src, I: Input + ?Sized, E: ParseEnvironment> Parser<'src, I, (), E> for Empty {
    fn parse(
        &self,
        _input: &'src I,
        state: ParseState<'src, I, E::Supply>,
    ) -> (ParseState<'src, I, E::Supply>, ParseOutput<(), E>) {
        (
            state,
            ParseOutput {
                ast: ParseResult::Success(()),
                cst: CstOutput::new(),
            },
        )
    }
}

/// Consumes no input and produces a speculative failure.
///
/// The zero identity for choices.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Fail;
impl Fail {
    pub fn new() -> Self {
        Self
    }
}
impl<'src, I: Input + ?Sized, E: ParseEnvironment> Parser<'src, I, (), E> for Fail {
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E::Supply>,
    ) -> (ParseState<'src, I, E::Supply>, ParseOutput<(), E>) {
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
}

/// Checks for the end-of-file.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct End;
impl End {
    pub fn new() -> Self {
        Self
    }
}
impl<'src, I: Input + ?Sized, E: ParseEnvironment> Parser<'src, I, (), E> for End {
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, <E as ParseEnvironment>::Supply>,
    ) -> (
        ParseState<'src, I, <E as ParseEnvironment>::Supply>,
        ParseOutput<(), E>,
    ) {
        let cursor = state.cursor;
        (
            state,
            ParseOutput {
                ast: input.peek(cursor).map_or(ParseResult::Success(()), |item| {
                    let failure = E::Failure::failure_at::<_, E::Failure>(
                        input,
                        cursor,
                        FailureKind::ExpectedEnd { found: item.token },
                    );
                    ParseResult::Speculative(failure)
                }),
                cst: CstOutput::new(),
            },
        )
    }
}
