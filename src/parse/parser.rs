use std::rc::Rc;

use rowan::SyntaxKind;

use crate::{
    combinator::{Associate, IntoOwned, Map, Node, Or, Then, WithEnv, end},
    cst::{CstOutput, IdSupply},
    input::Input,
    parse::{DefaultParseEnvironment, ParseEnvironment, ParseOutput, ParseState, TopLevelResult},
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
///
/// Due to additional type restrictions on the payload type `A` (namely being a reference to a [`ToOwned`] implementor),
/// the [.into_owned()](crate::parse::parser::ParserOwnedExt::into_owned) method is separated into trait [`ParserOwnedExt`].
pub trait ParserExt<'src, I: Input + ?Sized, A, E: ParseEnvironment = DefaultParseEnvironment>:
    Parser<'src, I, A, E>
{
    /// Parses from the start of input, returning the end state and output.
    ///
    /// This parse may succeed without fully consuming the input.
    fn parse_from_start(
        &self,
        input: &'src I,
    ) -> (ParseState<'src, I::Cursor, E::Supply>, ParseOutput<A, E>) {
        self.parse(input, ParseState::<'_, I::Cursor, E::Supply>::new(input))
    }
    /// Parses from the start of input to the end, converting the output to a top-level result.
    fn parse_all(&self, input: &'src I) -> TopLevelResult<A, E> {
        self.then_ignore(end())
            .parse(input, ParseState::<'_, I::Cursor, E::Supply>::new(input))
            .1
            .into_result()
    }

    /// Checks from the start of input, returning the end state and output.
    ///
    /// This check may succeed without fully consuming the input.
    fn check_from_start(
        &self,
        input: &'src I,
    ) -> (ParseState<'src, I::Cursor, E::Supply>, ParseOutput<(), E>) {
        self.check(input, ParseState::<'_, I::Cursor, E::Supply>::new(input))
    }
    /// Checks from the start of input to the end, converting the output to a top-level result.
    fn check_all(&self, input: &'src I) -> TopLevelResult<(), E> {
        self.then_ignore(end())
            .check(input, ParseState::<'_, I::Cursor, E::Supply>::new(input))
            .1
            .into_result()
    }

    fn boxed<'p>(self) -> WithEnv<E, Box<dyn Parser<'src, I, A, E> + 'p>>
    where
        Self: Sized + 'p,
    {
        let boxed: Box<dyn Parser<'src, I, A, E> + 'p> = Box::new(self);
        WithEnv::<E, Box<dyn Parser<'src, I, A, E> + 'p>>::new::<'src, I, A>(boxed)
    }
    fn shared<'p>(self) -> WithEnv<E, Rc<dyn Parser<'src, I, A, E> + 'p>>
    where
        Self: Sized + 'p,
    {
        let rc: Rc<dyn Parser<'src, I, A, E> + 'p> = Rc::new(self);
        WithEnv::<E, Rc<dyn Parser<'src, I, A, E> + 'p>>::new::<'src, I, A>(rc)
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
    fn node(self, kind: SyntaxKind) -> WithEnv<E, Node<Self>>
    where
        Self: Sized,
    {
        WithEnv::new(Node::new(kind, self))
    }
    fn associate<F, B>(self, kind: SyntaxKind, closure: F) -> WithEnv<E, Associate<Self, F, A>>
    where
        F: Fn(<E::Supply as IdSupply>::Id, A) -> B,
        Self: Sized,
    {
        WithEnv::new(Associate::new(self, kind, closure))
    }

    fn ignore(self) -> composite::Ignore<E, Self, A>
    where
        Self: Sized,
    {
        self.map(|_| ())
    }
    fn then_ignore<P, B>(self, next: P) -> composite::ThenIgnore<E, Self, P, A, B>
    where
        P: Parser<'src, I, B, E>,
        Self: Sized,
    {
        self.then(next).map(|(first, _)| first)
    }
    fn ignore_then<P, B>(self, next: P) -> composite::IgnoreThen<E, Self, P, A, B>
    where
        P: Parser<'src, I, B, E>,
        Self: Sized,
    {
        self.then(next).map(|(_, second)| second)
    }
}
impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, T> ParserExt<'src, I, A, E> for T where
    T: Parser<'src, I, A, E>
{
}

/// Types of composite and/or derived parsers formed by `ParserExt`,
/// expressed in terms of the primitive parser combinators.
///
/// These are here mostly due to `clippy::type_complexity`.
/// They are what you expect from the corresponding value definitions,
/// with closures type-erased to function pointers.
mod composite {
    use super::*;
    pub type Ignore<E, P, A> = WithEnv<E, Map<P, fn(A) -> (), A>>;
    pub type ThenIgnore<E, L, R, A, B> =
        WithEnv<E, Map<WithEnv<E, Then<L, R>>, fn((A, B)) -> A, (A, B)>>;
    pub type IgnoreThen<E, L, R, A, B> =
        WithEnv<E, Map<WithEnv<E, Then<L, R>>, fn((A, B)) -> B, (A, B)>>;
}

/// Convenience trait for [`.into_owned()`](Self::into_owned) method chaining.
pub trait ParserOwnedExt<'src, I: Input + ?Sized, A, E: ParseEnvironment = DefaultParseEnvironment>:
    Parser<'src, I, &'src A, E>
where
    A: ToOwned + ?Sized + 'src,
{
    fn into_owned(self) -> WithEnv<E, IntoOwned<Self, A>>
    where
        Self: Sized,
    {
        WithEnv::new(IntoOwned::new(self))
    }
}
impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, T> ParserOwnedExt<'src, I, A, E> for T
where
    T: Parser<'src, I, &'src A, E>,
    A: ToOwned + ?Sized + 'src,
{
}
