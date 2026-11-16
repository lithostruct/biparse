use std::ops::{Bound, RangeBounds};

use crate::{
    cst::AssociatedGreenCst,
    failure::{FailureKind, ParseFailure},
    input::Input,
    parse::{ParseEnvironment, ParseHint, ParseOutcome, ParseOutput, ParseState, Parser},
};

/// Parser combinator that executes a given parser zero, one or more than one times.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Repetition<P> {
    /// The inner parser.
    pub inner: P,
    /// The minimum number of repetitions, inclusive.
    minimum_repetitions: usize,
    /// The maximum number of repetitions, inclusive. [`None`] represents [`Bound::Unbounded`].
    maximum_repetitions: Option<usize>,
}
impl<P> Repetition<P> {
    /// Constructs a new [`Repetition`] parser object.
    ///
    /// `range` describes the minimum and maximum repetitions permitted by this object;
    /// - Below the minimum repetition, speculative failure is propagated.
    /// - Between the minimum and maximum repetitions (inclusive),
    ///   the parser may terminate early via speculative failure of the inner parser.
    /// - No parsing is tried beyond the maximum repetition count.
    ///
    /// An unbounded `range.start_bound()` is normalized as zero minimum repetitions.
    ///
    /// The inner parser must be advancing. Otherwise, a [`ParseOutcome::Fatal`] failure is returned upon parse.
    ///
    /// Returns [`None`] if the range is empty.
    pub fn new<'src, I: Input + ?Sized, A, E: ParseEnvironment>(
        inner: P,
        range: impl RangeBounds<usize>,
    ) -> Option<Self>
    where
        P: Parser<'src, I, A, E>,
    {
        let minimum_repetitions = match range.start_bound() {
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n.saturating_add(1),
            Bound::Unbounded => 0,
        };
        let maximum_repetitions = match range.end_bound() {
            Bound::Included(&n) => Some(n),
            Bound::Excluded(&n) => {
                if n == 0 {
                    return None;
                } else {
                    Some(n - 1)
                }
            }
            Bound::Unbounded => None,
        };
        if maximum_repetitions.is_some_and(|inner| inner < minimum_repetitions) {
            None
        } else {
            Some(Repetition {
                inner,
                minimum_repetitions,
                maximum_repetitions,
            })
        }
    }

    /// Returns the range at which this parser accepts the minimum and maximum number of repetitions.
    ///
    /// Note that this range object is normalized, and may not be identical to the `range`
    /// suppplied to [`Repetition::new`] upon construction.
    pub fn repetition_range(&self) -> (Bound<usize>, Bound<usize>) {
        let maximum_bound = if let Some(maximum) = self.maximum_repetitions {
            Bound::Included(maximum)
        } else {
            Bound::Unbounded
        };
        (Bound::Included(self.minimum_repetitions), maximum_bound)
    }
}

impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, Vec<A>, E>
    for Repetition<P>
where
    P: Parser<'src, I, A, E>,
{
    fn parse(
        &self,
        input: &'src I,
        mut state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<Vec<A>, E>) {
        let mut accumulative = Vec::new();
        let mut cst = AssociatedGreenCst::new();

        let iterator = (1_usize..).take_while(|&u| match self.maximum_repetitions {
            Some(maximum) => u <= maximum,
            None => true,
        });

        for iteration in iterator {
            let (new_state, new_output) = self.inner.parse(input, state.clone());
            match new_output.ast {
                ParseOutcome::Success(element) => {
                    // If no progress made, fatal error.
                    if new_state.cursor <= state.cursor {
                        let failure = E::Failure::failure_at(
                            input,
                            state.cursor,
                            FailureKind::NonGrowingRepetition {
                                inner_hint: self.inner.hint(),
                            },
                        );
                        return (
                            state,
                            ParseOutput {
                                ast: ParseOutcome::Fatal(failure),
                                cst,
                            },
                        );
                    }
                    // Else, combine and continue.
                    accumulative.push(element);
                    state = new_state;
                    cst = cst.merge_with(new_output.cst);
                    continue;
                }
                ParseOutcome::Speculative(failure) => {
                    // Do we have sufficient repetition?
                    // If yes, construct and return.
                    // If not, return a speculative error.
                    let ast = if iteration < self.minimum_repetitions {
                        let supplemental = E::Failure::failure_at(
                            input,
                            new_state.cursor,
                            FailureKind::InsufficientRepetition {
                                minimum_repetitions: self.minimum_repetitions,
                                in_repetition: iteration,
                            },
                        );
                        ParseOutcome::Speculative(failure.supplement(supplemental))
                    } else {
                        ParseOutcome::Success(accumulative)
                    };
                    return (state, ParseOutput { ast, cst });
                }
                ast @ ParseOutcome::Committed(_) | ast @ ParseOutcome::Fatal(_) => {
                    // Propagate committed or fatal failure.
                    return (
                        new_state,
                        ParseOutput {
                            ast: ast.map(|_| unreachable!() as Vec<A>),
                            cst: new_output.cst,
                        },
                    );
                }
            }
        }

        // End of maximum repetitions.
        (
            state,
            ParseOutput {
                ast: ParseOutcome::Success(accumulative),
                cst,
            },
        )
    }
    fn check(
        &self,
        input: &'src I,
        mut state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<(), E>) {
        let mut cst = AssociatedGreenCst::new();
        
        let iterator = (1_usize..).take_while(|&u| match self.maximum_repetitions {
            Some(maximum) => u <= maximum,
            None => true,
        });

        for iteration in iterator {
            let (new_state, new_output) = self.inner.check(input, state.clone());
            match new_output.ast {
                ParseOutcome::Success(()) => {
                    if new_state.cursor <= state.cursor {
                        let failure = E::Failure::failure_at(
                            input,
                            state.cursor,
                            FailureKind::NonGrowingRepetition {
                                inner_hint: self.inner.hint(),
                            },
                        );
                        return (
                            state,
                            ParseOutput {
                                ast: ParseOutcome::Fatal(failure),
                                cst,
                            },
                        );
                    }
                    // Combine and continue
                    state = new_state;
                    cst = cst.merge_with(new_output.cst);
                    continue;
                }
                ParseOutcome::Speculative(failure) => {
                    // Do we have sufficient repetition?
                    // If yes, construct and return.
                    // If not, return a speculative error.
                    let ast = if iteration < self.minimum_repetitions {
                        let supplemental = E::Failure::failure_at(
                            input,
                            new_state.cursor,
                            FailureKind::InsufficientRepetition {
                                minimum_repetitions: self.minimum_repetitions,
                                in_repetition: iteration,
                            },
                        );
                        ParseOutcome::Speculative(failure.supplement(supplemental))
                    } else {
                        ParseOutcome::Success(())
                    };
                    return (state, ParseOutput { ast, cst });
                }
                ParseOutcome::Committed(_) | ParseOutcome::Fatal(_) => {
                    // Propagate committed or fatal failure.
                    return (new_state, new_output);
                }
            }
        }
        // Maximum repetitions reached.
        (
            state,
            ParseOutput {
                ast: ParseOutcome::Success(()),
                cst,
            },
        )
    }
    fn hint(&self) -> ParseHint {
        ParseHint::Repetition {
            minimum: self.minimum_repetitions,
            maximum: self.maximum_repetitions,
            inner: Box::new(self.inner.hint()),
        }
    }
}
