use derive_more::From;

use crate::{
    cst::AssociatedGreenCst,
    failure::{FailureKind, ParseFailure},
    input::Input,
    parse::{ParseEnvironment, ParseHint, ParseOutcome, ParseOutput, ParseState, Parser},
};

/// Parser combinator that executes a given parser zero, one or more than one times.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, From)]
pub struct Many<P> {
    #[from]
    inner: P,
}
impl<P> Many<P> {
    pub fn new<'src, I: Input + ?Sized, A, E: ParseEnvironment>(inner: P) -> Self
    where
        P: Parser<'src, I, A, E>,
    {
        Many { inner }
    }
}

impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P> Parser<'src, I, Vec<A>, E> for Many<P>
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
        loop {
            let (new_state, new_output) = self.inner.parse(input, state.clone());
            match new_output.ast {
                ParseOutcome::Success(element) => {
                    // If no progress made, terminal error.
                    if new_state.cursor == state.cursor {
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
                    // Combine and continue.
                    accumulative.push(element);
                    state = new_state;
                    cst = cst.merge_with(new_output.cst);
                    continue;
                }
                ParseOutcome::Speculative(_) => {
                    // We've reached the end.
                    // Construct and return.
                    return (
                        state,
                        ParseOutput {
                            ast: ParseOutcome::Success(accumulative),
                            cst,
                        },
                    );
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
    }
    fn check(
        &self,
        input: &'src I,
        mut state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<(), E>) {
        let mut cst = AssociatedGreenCst::new();
        loop {
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
                ParseOutcome::Speculative(_) => {
                    // We've reached the end.
                    // Construct and return.
                    return (
                        state,
                        ParseOutput {
                            ast: ParseOutcome::Success(()),
                            cst,
                        },
                    );
                }
                ast @ ParseOutcome::Committed(_) | ast @ ParseOutcome::Fatal(_) => {
                    // Propagate committed or fatal failure.
                    return (
                        new_state,
                        ParseOutput {
                            ast,
                            cst: new_output.cst,
                        },
                    );
                }
            }
        }
    }
    fn hint(&self) -> ParseHint {
        ParseHint::Many(Box::new(self.inner.hint()))
    }
}
