use crate::{
    combinator::success,
    cst::AssociatedCst,
    input::Input,
    parse::{ParseEnvironment, ParseHint, ParseOutcome, ParseOutput, ParseState, Parser},
};

/// Parser combinator that executes a given parser zero, one or more than one times.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Many<P> {
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
        mut state: ParseState<'src, I::Cursor, <E as ParseEnvironment>::Supply>,
    ) -> (
        ParseState<'src, I::Cursor, <E as ParseEnvironment>::Supply>,
        ParseOutput<Vec<A>, E>,
    ) {
        let mut accumulative = Vec::new();
        let mut cst = AssociatedCst::new();
        loop {
            let (new_state, new_result) = self.inner.parse(input, state.clone());
            match new_result.ast {
                ParseOutcome::Success(element) => {
                    // Combine and continue.
                    accumulative.push(element);
                    state = new_state;
                    cst = cst.merge_with(new_result.cst);
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
                            cst: new_result.cst,
                        },
                    );
                }
            }
        }
    }
    fn check(
        &self,
        input: &'src I,
        mut state: ParseState<'src, I::Cursor, <E as ParseEnvironment>::Supply>,
    ) -> (
        ParseState<'src, I::Cursor, <E as ParseEnvironment>::Supply>,
        ParseOutput<(), E>,
    ) {
        let mut cst = success::<'src, I, E>().check(input, state.clone()).1.cst;
        loop {
            let (new_state, new_result) = self.inner.check(input, state.clone());
            cst = cst.merge_with(new_result.cst);
            match new_result.ast {
                ParseOutcome::Success(()) => {
                    state = new_state;
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
