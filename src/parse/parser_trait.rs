use std::{borrow::Cow, rc::Rc};

use either::Either;
use rowan::SyntaxKind;

pub use crate::failure::ParseHint;
use crate::{
    combinator::{
        As, Associate, Commit, IntoOwned, Many, Map, Node, Or, Recover, Success, Then, TryMap,
        With, end, success,
    },
    cst::IdSupply,
    input::Input,
    parse::{
        DefaultParseEnvironment, ParseArtifact, ParseEnvironment, ParseOutput, ParseStateData,
        TerminalFailure,
    },
};

/// Type alias over [`ParseStateData`] which provides more straightforward type arguments for usage in [`Parser`] implementors.
// Disabling the lint here because `ParseStateData` requires `E: ParseEnvironment` anyways.
#[allow(type_alias_bounds)]
pub type ParseState<'src, I, E: ParseEnvironment> = ParseStateData<'src, <I as Input>::Cursor, E>;

/// The central parsing trait, implemented by all combinators.
pub trait Parser<'src, I: Input + ?Sized, A, E: ParseEnvironment = DefaultParseEnvironment> {
    /// Parse the input from the given state, returning a result state and output.
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<A, E>);

    /// Checks the input for any errors/diagnostics from the given state.
    /// AST, raw CST and association information may be opportunistically omitted or discarded.
    ///
    /// Implementors are encouraged to override this default implementation;
    /// an adherent implementation must preserve both overall parsing outcome
    /// and the errors or diagnostics generated.
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<(), E>) {
        let (new_state, output) = <Self as Parser<'src, I, A, E>>::parse(self, input, state);
        let erased_output = ParseOutput {
            ast: output.ast.map(|_| ()),
            cst: output.cst,
        };
        (new_state, erased_output)
    }

    /// Returns a parse hint conceptually relating the item that is expected by this parser.
    ///
    /// Implementors are encouraged to override this default implementation.
    fn hint(&self) -> ParseHint {
        ParseHint::Opaque
    }
}

/// Convenience methods available to all implementors of [`Parser<'src, I, A, E>`]. Enables combinator method chaining.
///
/// This is separated from the main [`Parser`] to allow [`Parser`] to be dyn-compatible.
///
/// Due to additional type restrictions on the payload type `A` (namely being a reference to a [`ToOwned`] implementor),
/// the chaining method for [`IntoOwned`] is separated into trait [`ParserOwnedExt`].
pub trait ParserExt<'src, I: Input + ?Sized, A, E: ParseEnvironment = DefaultParseEnvironment>:
    Parser<'src, I, A, E>
{
    /// Parses from the start of input to the end, converting the output to a top-level result.
    ///
    /// - [`Ok`] indicates a successful top-level parse, returning [`ParseArtifact`].
    /// - [`Err`] indicates a terminal parse failure ([`TerminalFailure`]).
    fn parse_all(
        &self,
        input: &'src I,
    ) -> Result<ParseArtifact<A, E>, TerminalFailure<E::Failure>> {
        self.then_ignore(end())
            .parse(input, ParseState::<I, E>::new(input))
            .1
            .into_lifted()
    }
    /// Checks from the start of input to the end, converting the output to a top-level result.
    ///
    /// - [`Ok`] indicates a successful top-level parse, which may contain diagnostics generated from recovery.
    /// - [`Err`] indicates a terminal parse failure ([`TerminalFailure`]).
    fn check_all(&self, input: &'src I) -> Result<Vec<E::Failure>, TerminalFailure<E::Failure>> {
        self.then_ignore(end())
            .check(input, ParseState::<I, E>::new(input))
            .1
            .into_lifted()
            .map(|artifact| artifact.into_diagnostics())
    }

    /// Creates a type-erased [`Box`]-ed parser from `self`.
    fn boxed<'p>(self) -> Box<dyn Parser<'src, I, A, E> + 'p>
    where
        Self: Sized + 'p,
    {
        Box::new(self) as Box<dyn Parser<'src, I, A, E>>
    }
    /// Creates a type-erased shared parser from `self` using [`Rc`].
    fn shared<'p>(self) -> Rc<dyn Parser<'src, I, A, E> + 'p>
    where
        Self: Sized + 'p,
    {
        Rc::new(self) as Rc<dyn Parser<'src, I, A, E>>
    }

    /// Chains `self` to then sequentially execute the second parser.
    fn then<P, B>(self, next: P) -> With<E, Then<Self, P>>
    where
        P: Parser<'src, I, B, E>,
        Self: Sized,
    {
        With::new(Then::new(self, next))
    }
    /// Chains `self` as an alternative, falling through to the second parser.
    fn or<P, B>(self, other: P) -> With<E, Or<Self, P>>
    where
        P: Parser<'src, I, B, E>,
        Self: Sized,
    {
        With::new(Or::new(self, other))
    }
    /// Chains `self` to execute `map` on the success payload.
    fn map<F, B>(self, closure: F) -> With<E, Map<Self, F, A>>
    where
        F: Fn(A) -> B,
        Self: Sized,
    {
        With::new(Map::new(self, closure))
    }
    /// Chains `self` to add a new parent to the current CST forest.
    fn node(self, kind: impl Into<SyntaxKind>) -> With<E, Node<Self>>
    where
        Self: Sized,
    {
        With::new(Node::new(kind.into(), self))
    }
    /// Chains `self` to add a new parent to the current CST forest.
    /// Additionally provides a name to the parser object for use in diagnostics.
    fn named(
        self,
        name: impl Into<Cow<'static, str>>,
        kind: impl Into<SyntaxKind>,
    ) -> With<E, Node<Self>>
    where
        Self: Sized,
    {
        With::new(Node::new_as(name, kind.into(), self))
    }
    /// Chains `self` to add a new parent to the current CST forest,
    /// and associate it with the fresh `Id` supplied to the closure.
    fn associate<F, B>(
        self,
        kind: impl Into<SyntaxKind>,
        closure: F,
    ) -> With<E, Associate<Self, F, A>>
    where
        F: Fn(<E::Supply as IdSupply>::Id, A) -> B,
        Self: Sized,
    {
        With::new(Associate::new(self, kind.into(), closure))
    }
    /// Chains `self` to add a new parent to the current CST forest,
    /// and associate it with the fresh `Id` supplied to the closure.
    /// Additionally provides a name to the parser object for use in diagnostics.
    fn associate_as<F, B>(
        self,
        name: impl Into<Cow<'static, str>>,
        kind: impl Into<SyntaxKind>,
        closure: F,
    ) -> With<E, Associate<Self, F, A>>
    where
        F: Fn(<E::Supply as IdSupply>::Id, A) -> B,
        Self: Sized,
    {
        With::new(Associate::new_as(name, self, kind.into(), closure))
    }

    /// Chains `self` to repeat zero or more times, returning [`Vec<A>`].
    fn many(self) -> With<E, Many<Self>>
    where
        Self: Sized,
    {
        With::new(Many::new(self))
    }
    /// Chains `self` to repeat one or more times, returning a non-empty [`Vec<A>`].
    fn some(self) -> composite::Some<E, Self, A>
    where
        Self: Sized + Clone,
    {
        self.clone().then(self.many()).map(|(first, mut rest)| {
            rest.insert(0, first);
            rest
        })
    }
    /// Chains `self` to match on zero or one occurence, returning [`Option<A>`].
    fn optional(self) -> composite::Optional<'src, I, A, E, Self>
    where
        Self: Sized,
    {
        self.or(success()).map(Either::left)
    }

    /// Chains `self` to attempt fallible mapping on the success payload,
    /// signalling a committed failure if conversion fails.
    fn try_map<F, B, Error>(self, closure: F) -> With<E, TryMap<Self, F, A>>
    where
        F: Fn(A) -> Result<B, Error>,
        Error: std::error::Error,
        Self: Sized,
    {
        With::new(TryMap::new(self, closure))
    }
    /// Chains `self` to commit, promoting speculative failure into committed failure.
    fn commit(self) -> With<E, Commit<Self>>
    where
        Self: Sized,
    {
        With::new(Commit::new(self))
    }
    /// Chains `self` to a recovery strategy, handling and transforming committed failure
    /// into an associated diagnostic upon success.
    ///
    /// Common recovery strategies include parsing up to a set of recognized "safe characters",
    /// where regular parsing can then resume.
    ///
    /// # Note on Recovery Parser Failure
    ///
    /// Speculative or committed failure surfacing from the recovery parser will result in
    /// propagation of the failure from the fallible `self` parser.
    /// It is recommended to keep recovery simple relative to the fallible parser.
    fn recover<R, B, C>(
        self,
        with: R,
        closure: C,
        kind: impl Into<SyntaxKind>,
    ) -> With<E, Recover<Self, R, B, C>>
    where
        Self: Sized,
        R: Parser<'src, I, B, E>,
        C: Fn(<E::ErrorSupply as IdSupply>::Id, B) -> A,
    {
        With::new(Recover::new(self, with, closure, kind))
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

    /// Speculatively parses `self` and the given parser.
    /// Both parsers are of the same type,
    /// and the resulting [`Either<A, A>`] is converted to an `A` payload.
    fn or_same<P>(self, other: P) -> composite::OrSame<E, Self, P, A>
    where
        P: Parser<'src, I, A, E>,
        Self: Sized,
    {
        self.or(other).map(Either::into_inner)
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
        With<E, Map<With<E, Then<P, With<E, Many<P>>>>, fn((A, Vec<A>)) -> Vec<A>, (A, Vec<A>)>>;
    pub type Optional<'src, I, A, E, P> = With<
        E,
        Map<
            With<E, Or<P, As<'src, I, (), E, Success>>>,
            fn(Either<A, ()>) -> Option<A>,
            Either<A, ()>,
        >,
    >;

    pub type Ignore<E, P, A> = With<E, Map<P, fn(A) -> (), A>>;
    pub type ThenIgnore<E, L, R, A, B> = With<E, Map<With<E, Then<L, R>>, fn((A, B)) -> A, (A, B)>>;
    pub type IgnoreThen<E, L, R, A, B> = With<E, Map<With<E, Then<L, R>>, fn((A, B)) -> B, (A, B)>>;

    pub type OrSame<E, L, R, A> =
        With<E, Map<With<E, Or<L, R>>, fn(Either<A, A>) -> A, Either<A, A>>>;
}

/// Convenience trait for [`.into_owned()`](Self::into_owned) method chaining.
pub trait ParserOwnedExt<'src, I: Input + ?Sized, A, E: ParseEnvironment = DefaultParseEnvironment>:
    Parser<'src, I, &'src A, E>
where
    A: ToOwned + ?Sized + 'src,
{
    /// Chains `self` by turning an AST payload reference into owned data.
    fn into_owned(self) -> With<E, IntoOwned<Self, A>>
    where
        Self: Sized,
    {
        With::new(IntoOwned::new(self))
    }
}
impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, T> ParserOwnedExt<'src, I, A, E> for T
where
    T: Parser<'src, I, &'src A, E>,
    A: ToOwned + ?Sized + 'src,
{
}
