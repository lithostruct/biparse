use derive_more::From;

use crate::{
    cst::AssociatedGreenCst,
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
                ParseOutcome::Committed(failure) => {
                    // Propagate committed failure.
                    return (
                        new_state,
                        ParseOutput {
                            ast: ParseOutcome::Committed(failure),
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
                ParseOutcome::Committed(failure) => {
                    // Propagate committed failure.
                    return (
                        new_state,
                        ParseOutput {
                            ast: ParseOutcome::Committed(failure),
                            cst,
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
