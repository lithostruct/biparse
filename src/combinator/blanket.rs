use std::{rc::Rc, sync::Arc};

use crate::{
    input::Input,
    parse::{ParseEnvironment, ParseOutput, ParseState, Parser},
};

impl<'src, I: Input, A, E: ParseEnvironment, P> Parser<'src, I, A, E> for &P
where
    P: Parser<'src, I, A, E>,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<I, E::Supply>,
    ) -> (ParseState<I, E::Supply>, ParseOutput<A, E>) {
        (**self).parse(input, state)
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<I, E::Supply>,
    ) -> (ParseState<I, E::Supply>, ParseOutput<(), E>) {
        (**self).check(input, state)
    }
}

impl<'src, I: Input, A, E: ParseEnvironment, P> Parser<'src, I, A, E> for Box<P>
where
    P: Parser<'src, I, A, E>,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<I, E::Supply>,
    ) -> (ParseState<I, E::Supply>, ParseOutput<A, E>) {
        (**self).parse(input, state)
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<I, E::Supply>,
    ) -> (ParseState<I, E::Supply>, ParseOutput<(), E>) {
        (**self).check(input, state)
    }
}

impl<'src, I: Input, A, E: ParseEnvironment, P> Parser<'src, I, A, E> for Arc<P>
where
    P: Parser<'src, I, A, E>,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<I, E::Supply>,
    ) -> (ParseState<I, E::Supply>, ParseOutput<A, E>) {
        (**self).parse(input, state)
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<I, E::Supply>,
    ) -> (ParseState<I, E::Supply>, ParseOutput<(), E>) {
        (**self).check(input, state)
    }
}

impl<'src, I: Input, A, E: ParseEnvironment, P> Parser<'src, I, A, E> for Rc<P>
where
    P: Parser<'src, I, A, E>,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<I, E::Supply>,
    ) -> (ParseState<I, E::Supply>, ParseOutput<A, E>) {
        (**self).parse(input, state)
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<I, E::Supply>,
    ) -> (ParseState<I, E::Supply>, ParseOutput<(), E>) {
        (**self).check(input, state)
    }
}
