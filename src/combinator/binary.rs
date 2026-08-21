use crate::{
    cst::IdSupply,
    input::Input,
    parse::{ParseEnvironment, ParseOutput, ParseResult, ParseState, Parser},
};

pub struct Then<First, Second> {
    first: First,
    second: Second,
}
impl<'src, I: Input, First, Second, A, B, E: ParseEnvironment> Parser<'src, I, (A, B), E>
    for Then<First, Second>
where
    First: Parser<'src, I, A, E>,
    Second: Parser<'src, I, B, E>,
{
    fn parse<S: IdSupply<Id = <E as ParseEnvironment>::Id>>(
        &self,
        input: &'src I,
        state: ParseState<I, S>,
    ) -> (
        ParseState<I, S>,
        ParseOutput<(A, B), <E as ParseEnvironment>::Failure, <E as ParseEnvironment>::Id>,
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
    fn check<S: IdSupply<Id = <E as ParseEnvironment>::Id>>(
        &self,
        input: &'src I,
        state: ParseState<I, S>,
    ) -> (
        ParseState<I, S>,
        ParseOutput<(), <E as ParseEnvironment>::Failure, <E as ParseEnvironment>::Id>,
    ) {
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

pub struct Or<First, Second> {
    first: First,
    second: Second,
}
impl<'src, I: Input, First, Second, A, B, E: ParseEnvironment> Parser<'src, I, (A, B), E>
    for Or<First, Second>
where
    First: Parser<'src, I, A, E>,
    Second: Parser<'src, I, B, E>,
{
    fn parse<S: IdSupply<Id = <E as ParseEnvironment>::Id>>(
        &self,
        input: &'src I,
        state: ParseState<I, S>,
    ) -> (
        ParseState<I, S>,
        ParseOutput<(A, B), <E as ParseEnvironment>::Failure, <E as ParseEnvironment>::Id>,
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
}
