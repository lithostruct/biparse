use std::{cmp::Ordering, fmt::Debug, marker::PhantomData};

use rowan::SyntaxKind;

use crate::{
    cst::{Cst, AssociatedCst},
    failure::{ParseFailure, ParseFailureKind},
    green::GreenElement,
    input::{Input, InputItem, Pattern},
    parse::{ParseEnvironment, ParseHint, ParseOutput, ParseOutcome, ParseState, Parser},
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

                let mut byte_offset = input.byte_offset(old_position);
                let mut leaf_cst: Cst<GreenElement> = im::Vector::new();
                for cst_span in matched.cst_spans {
                    let (kind, span) = cst_span;
                    match byte_offset.cmp(&span.start) {
                        Ordering::Less => {
                            // The span (byte_offset)..(span.start) contains skipped trivia, which should be included.
                            let Some(trivia_kind) = input.skipped_trivia_kind() else {
                                panic!(
                                    "Byte span ({byte_offset}..{}) not covered by contigious input of type {}!",
                                    span.start,
                                    std::any::type_name::<I>()
                                );
                            };
                            let substring = input.slice_bytes(byte_offset, span.start);
                            let element: GreenElement =
                                state.intern_token(trivia_kind, &substring).into();
                            leaf_cst.push_back(element);
                        }
                        // Fallthrough.
                        Ordering::Equal => (),
                        Ordering::Greater => {
                            // Logically impossible; some `Pattern` implementor messed up.
                            panic!(
                                "Previous end offset {byte_offset} overlaps with next starting offset {}!",
                                span.start
                            );
                        }
                    }
                    // Add the substring of the actual match to the current one.
                    let substring = input.slice_bytes(span.start, span.end);
                    let element: GreenElement = state.intern_token(kind, &substring).into();
                    leaf_cst.push_back(element);
                    // Mutate `byte_offset`.
                    byte_offset = span.end;
                }
                // Check for skipped span between the final span and the next cursor.
                let next_offset = input.byte_offset(matched.next_cursor);
                match byte_offset.cmp(&next_offset) {
                    Ordering::Less => {
                        // The span (byte_offset)..(next_offset) contains skipped trivia, which should be included.
                        let Some(trivia_kind) = input.skipped_trivia_kind() else {
                            panic!(
                                "Byte span ({byte_offset}..{}) not covered by contigious input of type {}!",
                                next_offset,
                                std::any::type_name::<I>()
                            );
                        };
                        let substring = input.slice_bytes(byte_offset, next_offset);
                        let element: GreenElement =
                            state.intern_token(trivia_kind, &substring).into();
                        leaf_cst.push_back(element);
                    }
                    // Fallthrough, do nothing.
                    Ordering::Equal => (),
                    Ordering::Greater => {
                        // Logically unsound; some `Pattern` implementor messed up.
                        panic!(
                            "Last end offset {byte_offset} is greater than the cursor indicating offset {next_offset}!"
                        );
                    }
                }

                let new_state = ParseState {
                    cursor: matched.next_cursor,
                    ..state
                };
                (
                    new_state,
                    ParseOutput {
                        ast: ParseOutcome::Success(matched.payload),
                        cst: AssociatedCst::leaves(leaf_cst),
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
                        cst: AssociatedCst::new(),
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
                        cst: AssociatedCst::new(),
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
                        cst: AssociatedCst::new(),
                    },
                )
            }
        }
    }

    fn hint(&self) -> ParseHint {
        self.pattern.hint()
    }
}

/// Parser combinator that peeks at the current item and conditionally accepts it based on a predicate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Satisfies<F, Success, Failure> {
    kind: SyntaxKind,
    predicate: F,
    _marker: PhantomData<fn(Success, Failure)>,
}
impl<F, Success, Failure> Satisfies<F, Success, Failure> {
    pub fn new<I>(kind: SyntaxKind, predicate: F) -> Self
    where
        I: Input + ?Sized,
        F: Fn(&I, &InputItem<I::Token, I::Cursor>) -> Result<Success, Failure>,
    {
        Satisfies {
            kind,
            predicate,
            _marker: PhantomData,
        }
    }
}
impl<'src, I: Input + ?Sized, E: ParseEnvironment, F, Success, Failure> Parser<'src, I, Success, E>
    for Satisfies<F, Success, Failure>
where
    F: Fn(&I, &InputItem<I::Token, I::Cursor>) -> Result<Success, Failure>,
    Failure: std::error::Error,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, <E as ParseEnvironment>::Supply>,
    ) -> (
        ParseState<'src, I::Cursor, <E as ParseEnvironment>::Supply>,
        ParseOutput<Success, E>,
    ) {
        match input.peek(state.cursor) {
            Some(item) => {
                match (self.predicate)(input, &item) {
                    Ok(payload) => {
                        // Predicate succeeded, match!
                        let mut leaf_cst = im::Vector::new();

                        let substring = input.slice_bytes(item.span.start, item.span.end);
                        let token = state.intern_token(self.kind, &substring);
                        leaf_cst.push_back(token.into());

                        let next_offset = input.byte_offset(item.next_cursor);
                        let end_offset = item.span.end;
                        match end_offset.cmp(&next_offset) {
                            Ordering::Less => {
                                // The span (end_offset)..(next_offset) contains skipped trivia, which should be included.
                                let Some(trivia_kind) = input.skipped_trivia_kind() else {
                                    panic!(
                                        "Byte span ({}..{}) not covered by contigious input of type {}!",
                                        end_offset,
                                        next_offset,
                                        std::any::type_name::<I>()
                                    );
                                };
                                let substring = input.slice_bytes(end_offset, next_offset);
                                let trivia_element: GreenElement =
                                    state.intern_token(trivia_kind, &substring).into();
                                leaf_cst.push_back(trivia_element);
                            }
                            // Fallthrough, do nothing.
                            Ordering::Equal => (),
                            Ordering::Greater => {
                                // Logically unsound; some `Pattern` implementor messed up.
                                panic!(
                                    "Last end offset {end_offset} is greater than the cursor indicating offset {next_offset}!"
                                );
                            }
                        }

                        let new_state = ParseState {
                            cursor: item.next_cursor,
                            ..state
                        };
                        (
                            new_state,
                            ParseOutput {
                                ast: ParseOutcome::Success(payload),
                                cst: AssociatedCst::leaves(leaf_cst),
                            },
                        )
                    }
                    Err(error) => {
                        let failure = E::Failure::custom_failure_at(input, state.cursor, error);
                        (
                            state,
                            ParseOutput {
                                ast: ParseOutcome::Speculative(failure),
                                cst: AssociatedCst::new(),
                            },
                        )
                    }
                }
            }
            None => {
                let failure = E::Failure::failure_at(
                    input,
                    state.cursor,
                    ParseFailureKind::ExpectedFound {
                        hint: <Self as Parser<'src, I, _, E>>::hint(self),
                        difference_at: Some(state.cursor),
                    },
                );
                (
                    state,
                    ParseOutput {
                        ast: ParseOutcome::Speculative(failure),
                        cst: AssociatedCst::new(),
                    },
                )
            }
        }
    }

    /// There isn't much of a point to custom-implementing this,
    /// since `peek` is relatively cheap and the cost is basically just the construction of one/two CST nodes.
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, <E as ParseEnvironment>::Supply>,
    ) -> (
        ParseState<'src, I::Cursor, <E as ParseEnvironment>::Supply>,
        ParseOutput<(), E>,
    ) {
        let (new_state, output) = <Self as Parser<'src, I, _, E>>::parse(self, input, state);
        let erased_output = ParseOutput {
            ast: output.ast.map(|_| ()),
            cst: AssociatedCst::new(),
        };
        (new_state, erased_output)
    }

    fn hint(&self) -> ParseHint {
        ParseHint::Opaque
    }
}
