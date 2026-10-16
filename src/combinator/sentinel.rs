use crate::{
    cst::AssociatedCst,
    failure::{ParseFailure, ParseFailureKind},
    input::Input,
    parse::{
        Expectation, ParseEnvironment, ParseHint, ParseOutcome, ParseOutput, ParseState, Parser,
    },
};
use std::fmt::Debug;

/// Parser combinator that consumes no input and succeeds.
///
/// The zero identity for sequential operations.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Success;
impl Success {
    pub fn new() -> Self {
        Self
    }
}
impl<'src, I: Input + ?Sized, E: ParseEnvironment> Parser<'src, I, (), E> for Success {
    fn parse(
        &self,
        _input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (ParseState<'src, I::Cursor, E::Supply>, ParseOutput<(), E>) {
        (
            state,
            ParseOutput {
                ast: ParseOutcome::Success(()),
                cst: AssociatedCst::new(),
            },
        )
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>,
    ) -> (
        ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>,
        ParseOutput<(), E>,
    ) {
        self.parse(input, state)
    }
    fn hint(&self) -> ParseHint {
        ParseHint::Empty
    }
}

/// Parser combinator that onsumes no input and produces a speculative failure.
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
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (ParseState<'src, I::Cursor, E::Supply>, ParseOutput<(), E>) {
        let failure = E::Failure::failure_at(input, state.cursor, ParseFailureKind::FailPrimitive);
        (
            state,
            ParseOutput {
                ast: ParseOutcome::Speculative(failure),
                cst: AssociatedCst::new(),
            },
        )
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>,
    ) -> (
        ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>,
        ParseOutput<(), E>,
    ) {
        self.parse(input, state)
    }
    fn hint(&self) -> ParseHint {
        ParseHint::Fails
    }
}

/// Parser combinator that checks for the end-of-file.
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
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (ParseState<'src, I::Cursor, E::Supply>, ParseOutput<(), E>) {
        let cursor = state.cursor;
        (
            state,
            ParseOutput {
                ast: input.peek(cursor).map_or(ParseOutcome::Success(()), |_| {
                    let failure = E::Failure::failure_at(
                        input,
                        cursor,
                        ParseFailureKind::ExpectedFound {
                            hint: <Self as Parser<'src, I, (), E>>::hint(self),
                            difference_at: Some(cursor),
                        },
                    );
                    ParseOutcome::Speculative(failure)
                }),
                cst: AssociatedCst::new(),
            },
        )
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>,
    ) -> (
        ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>,
        ParseOutput<(), E>,
    ) {
        self.parse(input, state)
    }
    fn hint(&self) -> ParseHint {
        ParseHint::Expectation(Expectation::EndOfInput)
    }
}
