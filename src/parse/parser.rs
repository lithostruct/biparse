use crate::{
    combinator::{Map, Or, Then, ToOwned, WithEnv},
    cst::CstOutput,
    input::Input,
    parse::{DefaultParseEnvironment, ParseEnvironment, ParseOutput, ParseState},
};

/// The central parsing trait, implemented by all combinators.
pub trait Parser<'src, I: Input + ?Sized, A, E: ParseEnvironment = DefaultParseEnvironment> {
    /// Parse the input.
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (ParseState<'src, I::Cursor, E::Supply>, ParseOutput<A, E>);

    /// Checks the input for any errors.
    ///
    /// AST, CST and origin information is discarded. CST information *must* be empty (equivalent to [`CstOutput::new`]).
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I::Cursor, E::Supply>,
    ) -> (ParseState<'src, I::Cursor, E::Supply>, ParseOutput<(), E>) {
        let (new_state, output) = self.parse(input, state);
        let erased_output = ParseOutput {
            ast: output.ast.map(|_| ()),
            cst: CstOutput::new(),
        };
        (new_state, erased_output)
    }
}

/// Convenience methods available to all implementors of [`Parser<'src, I, A, E>`]. Enables combinator method chaining.
///
/// This is separated from the main [`Parser`] to allow [`Parser`] to be dyn-compatible.
pub trait ParserExt<'src, I: Input + ?Sized, A, E: ParseEnvironment = DefaultParseEnvironment>:
    Parser<'src, I, A, E>
{
    fn parse_from_start(
        &self,
        input: &'src I,
    ) -> (ParseState<'src, I::Cursor, E::Supply>, ParseOutput<A, E>) {
        self.parse(input, ParseState::<'_, I::Cursor, E::Supply>::new(input))
    }

    fn boxed<'p>(self) -> WithEnv<E, Box<dyn Parser<'src, I, A, E> + 'p>>
    where
        Self: Sized + 'p,
    {
        let boxed: Box<dyn Parser<'src, I, A, E> + 'p> = Box::new(self);
        WithEnv::<E, Box<dyn Parser<'src, I, A, E> + 'p>>::new::<'src, I, A>(boxed)
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
        Self: Sized,
    {
        WithEnv::new(Map::new(self, closure))
    }
    fn to_owned(self) -> WithEnv<E, ToOwned<Self, A>>
    where
        A: std::borrow::ToOwned + 'src,
        Self: Sized,
    {
        WithEnv::new(ToOwned::new(self))
    }
}
impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, T> ParserExt<'src, I, A, E> for T where
    T: Parser<'src, I, A, E>
{
}
