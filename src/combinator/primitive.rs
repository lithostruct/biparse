use crate::{
    cst::AssociatedGreenCst,
    failure::{Descriptor, FailureKind, ParseFailure},
    input::Input,
    parse::{ParseEnvironment, ParseHint, ParseOutcome, ParseOutput, ParseState, Parser},
};
use std::{borrow::Cow, fmt::Debug};

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
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<(), E>) {
        (
            state,
            ParseOutput {
                ast: ParseOutcome::Success(()),
                cst: AssociatedGreenCst::new(),
            },
        )
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<(), E>) {
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
pub struct Fail(Option<Cow<'static, str>>);
impl Fail {
    pub fn new() -> Self {
        Self(None)
    }
    pub fn new_with(msg: impl Into<Cow<'static, str>>) -> Self {
        Self(Some(msg.into()))
    }
}
impl<'src, I: Input + ?Sized, E: ParseEnvironment> Parser<'src, I, (), E> for Fail {
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<(), E>) {
        let failure = E::Failure::failure_at(
            input,
            state.cursor,
            FailureKind::FailPrimitive {
                message: self.0.clone(),
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
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<(), E>) {
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
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<(), E>) {
        let cursor = state.cursor;
        let cst = if state.encountered_end {
            // Previously encountered end, leading span materialized for all-trivia input.
            // Do nothing.
            AssociatedGreenCst::new()
        } else {
            state.with_interner(|interner| {
                AssociatedGreenCst::leaves([], cursor, cursor, input, interner)
            })
        };

        (
            ParseState::<I, E> {
                encountered_end: true,
                ..state
            },
            ParseOutput {
                ast: input.peek(cursor).map_or(ParseOutcome::Success(()), |_| {
                    let failure = E::Failure::failure_at(
                        input,
                        cursor,
                        FailureKind::ExpectedFound {
                            hint: <Self as Parser<'src, I, (), E>>::hint(self),
                            difference_at: Some(cursor),
                        },
                    );
                    ParseOutcome::Speculative(failure)
                }),
                cst,
            },
        )
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<(), E>) {
        self.parse(input, state)
    }
    fn hint(&self) -> ParseHint {
        ParseHint::Expectation(Descriptor::EndOfInput)
    }
}
