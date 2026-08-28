use std::cmp::Ordering;

use rowan::SyntaxKind;

use crate::{
    cst::{Cst, CstAssociation, CstOutput},
    failure::{FailureKind, ParseFailure},
    green::GreenElement,
    input::{Input, Pattern},
    parse::{ParseEnvironment, ParseOutput, ParseResult, ParseState, Parser},
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
                        ast: ParseResult::Success(matched.payload),
                        cst: CstOutput {
                            green: leaf_cst,
                            origin: CstAssociation::new(),
                        },
                    },
                )
            }
            Err(error_position) => {
                let failure = E::Failure::failure_at::<_, E::Failure>(
                    input,
                    state.cursor,
                    FailureKind::ExpectedFound {
                        expected: self.pattern.describe(),
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
                    cursor: matched.next_cursor,
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
                let failure = E::Failure::failure_at::<_, E::Failure>(
                    input,
                    state.cursor,
                    FailureKind::ExpectedFound {
                        expected: self.pattern.describe(),
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
