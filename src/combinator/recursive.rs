use std::borrow::Cow;

use crate::{
    cst::IdSupply,
    input::Input,
    parse::{ParseEnvironment, ParseOutput, ParseState, Parser},
};

#[derive(Debug, Clone)]
pub struct Recursive<F> {
    closure: F,
    description: Cow<'static, str>,
}
impl<'src, I: Input, A, E: ParseEnvironment, F, P> Parser<'src, I, A, E> for Recursive<F>
where
    F: Fn(&Self) -> P,
    P: Parser<'src, I, A, E>,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<I, E::Supply>,
    ) -> (ParseState<I, E::Supply>, ParseOutput<A, E>) {
        todo!()
    }
}
