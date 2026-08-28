use std::rc::Rc;

use either::Either;
use rowan::SyntaxKind;

use crate::{
    combinator::{AsParser, Associate, IntoOwned, Many, Map, Node, Or, Success, Then, WithEnv, end, success}, cst::{CstOutput, IdSupply}, input::Input, parse::{
        DefaultParseEnvironment, ParseEnvironment, ParseOutput, ParseState, TopLevelResult,
        diagnostics::ParseHint,
    },
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

    /// Returns the diagnostics of `self`:
    ///
    /// Diagnostics are an approximation of the expected input of self,
    /// and are used for error reporting.
    fn hint(&self) -> ParseHint {
        ParseHint::Opaque
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

    /// Creates a type-erased [`Box`]-ed parser from `self`.
    fn boxed<'p>(self) -> WithEnv<E, Box<dyn Parser<'src, I, A, E> + 'p>>
    where
        Self: Sized + 'p,
    {
        let boxed: Box<dyn Parser<'src, I, A, E> + 'p> = Box::new(self);
        WithEnv::<E, Box<dyn Parser<'src, I, A, E> + 'p>>::new::<'src, I, A>(boxed)
    }
    /// Creates a type-erased shared parser from `self` using [`Rc`].
    fn shared<'p>(self) -> WithEnv<E, Rc<dyn Parser<'src, I, A, E> + 'p>>
    where
        Self: Sized + 'p,
    {
        let rc: Rc<dyn Parser<'src, I, A, E> + 'p> = Rc::new(self);
        WithEnv::<E, Rc<dyn Parser<'src, I, A, E> + 'p>>::new::<'src, I, A>(rc)
    }

    /// Chains `self` to then sequentially execute the second parser.
    fn then<P, B>(self, next: P) -> WithEnv<E, Then<Self, P>>
    where
        P: Parser<'src, I, B, E>,
        Self: Sized,
    {
        WithEnv::new(Then::new(self, next))
    }
    /// Chains `self` as an alternative, falling through to the second parser.
    fn or<P, B>(self, other: P) -> WithEnv<E, Or<Self, P>>
    where
        P: Parser<'src, I, B, E>,
        Self: Sized,
    {
        WithEnv::new(Or::new(self, other))
    }
    /// Chains `self` to execute `map` on the success payload.
    fn map<F, B>(self, closure: F) -> WithEnv<E, Map<Self, F, A>>
    where
        F: Fn(A) -> B,
        Self: Sized,
    {
        WithEnv::new(Map::new(self, closure))
    }
    /// Chains `self` to add a new parent to the current CST forest.
    fn node(self, kind: SyntaxKind) -> WithEnv<E, Node<Self>>
    where
        Self: Sized,
    {
        WithEnv::new(Node::new(kind, self))
    }
    /// Chains `self` to add a new parent to the current CST forest,
    /// and associate it with the fresh `Id` supplied to the closure.
    fn associate<F, B>(self, kind: SyntaxKind, closure: F) -> WithEnv<E, Associate<Self, F, A>>
    where
        F: Fn(<E::Supply as IdSupply>::Id, A) -> B,
        Self: Sized,
    {
        WithEnv::new(Associate::new(self, kind, closure))
    }

    fn many(self) -> WithEnv<E, Many<Self>>
    where
        Self: Sized,
    {
        WithEnv::new(Many::new(self))
    }
    fn some(self) -> composite::Some<E, Self, A>
    where
        Self: Sized + Clone,
    {
        self.clone().many().then(self).map(|(mut rest, last)| {
            rest.push(last);
            rest
        })
    }
    fn optional(self) -> composite::Optional<'src, I, A, E, Self>
    where
        Self: Sized,
    {
        self.or(success()).map(|either| match either {
            Either::Left(a) => Some(a),
            Either::Right(()) => None,
        })
    }

    /// Discards the current AST payload.
    fn ignore(self) -> composite::Ignore<E, Self, A>
    where
        Self: Sized,
    {
        self.map(|_| ())
    }
    /// Sequentially executes the next parser, discarding the next parser's payload.
    fn then_ignore<P, B>(self, next: P) -> composite::ThenIgnore<E, Self, P, A, B>
    where
        P: Parser<'src, I, B, E>,
        Self: Sized,
    {
        self.then(next).map(|(first, _)| first)
    }
    /// Sequentially executes the next parser, discarding `self`'s payload.
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
pub mod composite {
    use super::*;

    pub type Some<E, P, A> =
        WithEnv<E, Map<
            WithEnv<E, Then<WithEnv<E, Many<P>>, P>>,
            fn((Vec<A>, A)) -> Vec<A>,
            (Vec<A>, A)
        >>;
    pub type Optional<'src, I, A, E, P> =
        WithEnv<E, Map<
            WithEnv<E, Or<P, AsParser<'src, I, (), E, Success>>>,
            fn(Either<A, ()>) -> Option<A>,
            Either<A, ()>
        >>;

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
    /// Chains `self` by turning an AST payload reference into owned data.
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
