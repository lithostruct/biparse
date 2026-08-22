use crate::{
    combinator::{Map, Or, Then, ToOwned, WithEnv}, cst::CstOutput, input::Input, parse::{DefaultParseEnvironment, ParseEnvironment, ParseOutput, ParseState},
};

/// The central parsing trait, implemented by all combinators.
pub trait Parser<'src, I: Input + ?Sized, A, E: ParseEnvironment = DefaultParseEnvironment>:
    Sized
{
    /// Parse the input.
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E::Supply>,
    ) -> (ParseState<'src, I, E::Supply>, ParseOutput<A, E>);

    /// Checks the input for any errors.
    ///
    /// AST, CST and origin information is discarded. CST information *must* be empty (equivalent to [`CstOutput::new`]).
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E::Supply>,
    ) -> (ParseState<'src, I, E::Supply>, ParseOutput<(), E>) {
        let (new_state, output) = self.parse(input, state);
        let erased_output = ParseOutput {
            ast: output.ast.map(|_| ()),
            cst: CstOutput::new(),
        };
        (new_state, erased_output)
    }

    fn parse_from_start(
        &self,
        input: &'src I,
    ) -> (ParseState<'src, I, E::Supply>, ParseOutput<A, E>) {
        self.parse(input, ParseState::new(input))
    }

    fn then<P, B>(self, next: P) -> WithEnv<E, Then<Self, P>>
    where
        P: Parser<'src, I, B, E>,
        Self: Sized,
    {
        WithEnv::new(Then::new(self, next))
    }
    fn or<P, B>(self, other: P) -> WithEnv<E, Or<Self, P>>
    where
        P: Parser<'src, I, B, E>,
        Self: Sized,
    {
        WithEnv::new(Or::new(self, other))
    }
    fn map<F, B>(self, closure: F) -> WithEnv<E, Map<Self, F, A>>
    where
        F: Fn(A) -> B,
    {
        WithEnv::new(Map::new(self, closure))
    }
    fn to_owned(self) -> WithEnv<E, ToOwned<Self, A>>
    where
        A: std::borrow::ToOwned + 'src,
    {
        WithEnv::new(ToOwned::new(self))
    }
}
