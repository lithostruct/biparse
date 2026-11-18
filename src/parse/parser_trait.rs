use std::{borrow::Cow, ops::RangeBounds, rc::Rc};

use either::Either;
use rowan::SyntaxKind;

pub use crate::failure::ParseHint;
use crate::{
    combinator::{
        As, Associate, Commit, Fatal, IntoOwned, Map, Node, Or, Recover, Repetition, Success, Then,
        TryMap, end, success,
    },
    cst::IdSupply,
    failure::ParseFailure,
    input::Input,
    parse::{ParseArtifact, ParseOutput, ParseStateData, TerminalFailure, environment},
};

/// Type alias over [`ParseStateData`] which provides more straightforward type arguments for usage in [`Parser`] implementors.
// Disabling the lint here because `ParseStateData` requires `E: ParseEnvironment` anyways.
#[allow(type_alias_bounds)]
pub type ParseState<'src, I, E: ParseEnvironment> = ParseStateData<'src, <I as Input>::Cursor, E>;

/// The central parsing trait, implemented by all combinators.
/// Heavily based on that of [`chumsky`](https://docs.rs/chumsky/latest/chumsky/trait.Parser.html).
///
/// Parsers take inputs of type `I`, which will implement [`Input`].
/// Refer to the documentation on [`Input`] for examples of common input types.
///
/// It will then attempt to parse them into an AST value of type `A`, which may be just about any type.
/// In doing so, they may encounter failures.
/// These need not be fatal to the parsing process: certain syntactic errors can be recovered from
/// and a valid output may still be generated alongside any errors that were encountered along the way.
/// Refer to [`ParseOutcome`](crate::parse::ParseOutcome) regarding the modes of failure supported.
///
/// The final type parameter, E, is expected to be a [`ParseEnvironment`] implementor.
/// [`ParseEnvironment`] is used to encapsulate the various types a parser uses that are not simply its input and output.
/// Refer to the documentation on [`ParseEnvironment`] for more detail on the contained types.
/// If not provided, it will default to [`environment::Rich`],
/// which retains maximal failure information but may have significant overhead.
///
/// The lifetime of the parser is used for zero-copy output. The input is bound by the lifetime,
/// and returned values or parser state may take advantage of this to borrow tokens or slices of the input and hold on to them,
/// if the input supports this.
pub trait Parser<'src, I: Input + ?Sized, A, E: ParseEnvironment = environment::Rich> {
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

/// The trait describing additional information required by certain parsing constructs.
///
/// This trait should be implemented by zero-sized types.
pub trait ParseEnvironment: 'static {
    /// The type of the failure payload.
    type Failure: ParseFailure;
    /// The type of the supply provided to [`Associate`](crate::combinator::Associate) combinators.
    /// Used for associations.
    type Supply: IdSupply;
    /// The type of the supply provided to [`Recover`](crate::combinator::Recover) combinators.
    /// Used for diagnostics.
    type ErrorSupply: IdSupply;
}

/// Methods blanket implemented for all implementors of [`Parser<'src, I, A, E>`].
///
/// Enables combinator method chaining,
/// which is the preferred method of construction over direct usage of combinator types.
///
/// Due to additional type restrictions on the payload type `A` (namely being a reference to a [`ToOwned`] implementor),
/// the chaining method for [`IntoOwned`] is separated into trait [`ParserIntoOwnedExt`].
///
/// # Stability
/// - Unlike [`Parser`], this trait is not intended to be implemented directly.
/// - New methods may be added to this trait *without* following semantic versioning.
/// - Existing methods will only be modified in accordance with semantic versioning.
pub trait ParserExt<'src, I: Input + ?Sized, A, E: ParseEnvironment = environment::Rich>:
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
    ///
    /// Resulting parser produces [`(A, B)`](primitive@tuple) on success.
    ///
    /// Note that `a.then(b).then(c)` produces a left-nested [`((A, B), C)`] payload.
    fn then<P, B>(self, next: P) -> Then<Self, P>
    where
        P: Parser<'src, I, B, E>,
        Self: Sized,
    {
        Then::new(self, next)
    }
    /// Chains `self` as an alternative, falling through to the second parser.
    ///
    /// `self` and `other` must produce an AST value of the same type `A` on success.
    fn or<P>(self, other: P) -> Or<Self, P>
    where
        P: Parser<'src, I, A, E>,
        Self: Sized,
    {
        Or::new(self, other)
    }
    /// Chains `self` to transform the success payload via the given closure.
    ///
    /// Resulting parser produces `B` on success, where `B` is the result type of the closure.
    fn map<F, B>(self, closure: F) -> Map<Self, F, A>
    where
        F: Fn(A) -> B,
        Self: Sized,
    {
        Map::new(self, closure)
    }
    /// Chains `self` to add a new parent to the current CST forest.
    fn node(self, kind: impl Into<SyntaxKind>) -> Node<Self>
    where
        Self: Sized,
    {
        Node::new(kind.into(), self)
    }
    /// Chains `self` to add a new parent to the current CST forest.
    ///
    /// Additionally provides a name to the parser object for use in diagnostics.
    fn named(self, name: impl Into<Cow<'static, str>>, kind: impl Into<SyntaxKind>) -> Node<Self>
    where
        Self: Sized,
    {
        Node::new_as(name, kind.into(), self)
    }
    /// Chains `self` to add a new parent to the current CST forest,
    /// and associate it with the fresh `Id` supplied to the closure.
    ///
    /// Resulting parser produces `B` on success, where `B` is the return type of the closure.
    fn associate<F, B>(self, kind: impl Into<SyntaxKind>, closure: F) -> Associate<Self, F, A>
    where
        F: Fn(<E::Supply as IdSupply>::Id, A) -> B,
        Self: Sized,
    {
        Associate::new(self, kind.into(), closure)
    }
    /// Chains `self` to add a new parent to the current CST forest,
    /// and associate it with the fresh `Id` supplied to the closure.
    ///
    /// Resulting parser produces `B` on success, where `B` is the return type of the closure.
    ///
    /// Additionally provides a name to the parser object for use in diagnostics.
    fn associate_as<F, B>(
        self,
        name: impl Into<Cow<'static, str>>,
        kind: impl Into<SyntaxKind>,
        closure: F,
    ) -> Associate<Self, F, A>
    where
        F: Fn(<E::Supply as IdSupply>::Id, A) -> B,
        Self: Sized,
    {
        Associate::new_as(name, self, kind.into(), closure)
    }

    /// Chains `self` to repeat for the minimum and maximum bounds described by `range`,
    /// returning a [`Vec<A>`] with the number of elements bounded to `range`.
    ///
    /// Refer to [`Repetition::new`] regarding the handling of the [`RangeBounds`] value supplied.
    ///
    /// `self` must consume input. Otherwise, attempting a parse results in
    /// [`ParseOutcome::Fatal`](crate::parse::ParseOutcome::Fatal) failure.
    ///
    /// # Panics
    ///
    /// Panics if `range` contains no elements.
    fn repeat_by(self, range: impl RangeBounds<usize>) -> Repetition<Self>
    where
        Self: Sized,
    {
        Repetition::new(self, range).expect("range provided must be non-empty")
    }
    /// Chains `self` to repeat *exactly* `n` times, returning a [`Vec<A>`] with exactly `n` elements.
    ///
    /// `self` must consume input. Otherwise, attempting a parse results in
    /// [`ParseOutcome::Fatal`](crate::parse::ParseOutcome::Fatal) failure.
    fn repeat(self, n: usize) -> Repetition<Self>
    where
        Self: Sized,
    {
        Repetition::new(self, n..=n).expect("Range valid!")
    }

    /// Chains `self` to repeat zero or more times, producing a [`Vec<A>`] on success.
    ///
    /// `self` must consume input. Otherwise, attempting a parse results in
    /// [`ParseOutcome::Fatal`](crate::parse::ParseOutcome::Fatal) failure.
    fn many(self) -> Repetition<Self>
    where
        Self: Sized,
    {
        Repetition::new(self, 0..).expect("Range valid!")
    }
    /// Chains `self` to repeat one or more times, producing a non-empty [`Vec<A>`] on success.
    ///
    /// `self` must consume input. Otherwise, attempting a parse results in
    /// [`ParseOutcome::Fatal`](crate::parse::ParseOutcome::Fatal) failure.
    fn some(self) -> Repetition<Self>
    where
        Self: Sized,
    {
        Repetition::new(self, 1..).expect("Range valid!")
    }
    /// Chains `self` to match on zero or one occurrence, returning [`Option<A>`].
    fn optional(self) -> derived::Optional<'src, Self, I, A, E>
    where
        Self: Sized,
    {
        self.either_or(success()).map(Either::left)
    }

    /// Chains `self` to attempt fallible mapping on the success payload,
    /// signalling a committed failure if conversion fails.
    ///
    /// Resulting parser produces `B` upon conversion success.
    fn try_map<F, B, Error>(self, closure: F) -> TryMap<Self, F, A>
    where
        F: Fn(A) -> Result<B, Error>,
        Error: std::error::Error + 'static,
        Self: Sized,
    {
        TryMap::new(self, closure)
    }
    /// Chains `self` to commit, promoting speculative failure into
    /// [`ParseOutcome::Committed`](crate::parse::ParseOutcome::Committed) failure.
    ///
    /// Committed failure follows exception semantics, and is propagated outwards
    /// until explicitly handled or recovered from,
    /// typically via [`ParserExt::recover`].
    fn commit(self) -> Commit<Self>
    where
        Self: Sized,
    {
        Commit::new(self)
    }
    /// Chains `self`, promoting speculative and committed failure into
    /// [`ParseOutcome::Fatal`](crate::parse::ParseOutcome::Fatal) failure.
    ///
    /// Fatal failures are unrecoverable.
    fn fatal(self) -> Fatal<Self>
    where
        Self: Sized,
    {
        Fatal::new(self)
    }
    /// Chains `self` to a recovery strategy, handling and transforming
    /// [`ParseOutcome::Committed`](crate::parse::ParseOutcome::Committed) failure
    /// into a diagnostic upon success.
    ///
    /// Common recovery strategies include parsing up to a set of recognized synchronization characters or end-of-input,
    /// where regular parsing can then resume.
    ///
    /// # Note on Recovery Parser Failure
    ///
    /// Failures surfacing from the recovery parser will result in
    /// resuming propagation of the failure from the fallible `self` parser.
    /// Hence, it is recommended to keep recovery simple relative to the fallible parser.
    fn recover<R, B, C>(
        self,
        with: R,
        closure: C,
        kind: impl Into<SyntaxKind>,
    ) -> Recover<Self, R, B, C>
    where
        Self: Sized,
        R: Parser<'src, I, B, E>,
        C: Fn(<E::ErrorSupply as IdSupply>::Id, B) -> A,
    {
        Recover::new(self, with, closure, kind)
    }

    /// Discards the current AST payload, producing the unit value [`()`](primitive@unit) on success.
    fn ignore(self) -> derived::Ignore<Self, A>
    where
        Self: Sized,
    {
        self.map(|_| ())
    }
    /// Sequentially executes the next parser, discarding the next parser's payload.
    fn then_ignore<P, B>(self, next: P) -> derived::ThenIgnore<Self, P, A, B>
    where
        P: Parser<'src, I, B, E>,
        Self: Sized,
    {
        self.then(next).map(|(first, _)| first)
    }
    /// Sequentially executes the next parser, discarding `self`'s payload.
    fn ignore_then<P, B>(self, next: P) -> derived::IgnoreThen<Self, P, A, B>
    where
        P: Parser<'src, I, B, E>,
        Self: Sized,
    {
        self.then(next).map(|(_, second)| second)
    }

    /// Speculatively parses `self` and the given parser.
    ///
    /// Unlike [`ParserExt::or`], the payloads of `self` and `other` may be different, producing an [`Either<A, B>`] payload.
    ///
    /// The first parsing path from parser `a` is represented by [`Either::Left`],
    /// and the second parsing path from `b` is [`Either::Right`].
    ///
    /// Note that `a.either_or(b).either_or(c)` produces a left-nested [`Either<Either<A, B>, C>`] payload.
    fn either_or<P, B>(self, other: P) -> derived::EitherOr<Self, P, A, B>
    where
        P: Parser<'src, I, B, E>,
        Self: Sized,
    {
        self.map(Either::Left as fn(A) -> Either<A, B>)
            .or(other.map(Either::Right))
    }

    /// Establishes a commitment point where subsequent speculative failure in the `other` parser
    /// is promoted to a [`ParseOutcome::Committed`](crate::parse::ParseOutcome::Committed) failure.
    ///
    /// Resembles a Prolog-like cut boundary.
    ///
    /// Note that chaining `a.then_commit_to(b).then(c)` will only cause `b` to commit, but not `c`.
    fn then_commit_to<P, B>(self, other: P) -> derived::ThenCommitTo<Self, P>
    where
        P: Parser<'src, I, B, E>,
        Self: Sized,
    {
        self.then(other.commit())
    }
}
impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, T> ParserExt<'src, I, A, E> for T where
    T: Parser<'src, I, A, E>
{
}

/// Types of composite and/or derived parsers formed by [`ParserExt`],
/// expressed in terms of the primitive parser combinators.
pub mod derived {
    use super::*;

    /// The derived parser type of [`ParserExt::optional`].
    pub type Optional<'src, SELF, I, A, E> = Map<
        derived::EitherOr<SELF, As<'src, I, (), E, Success>, A, ()>,
        fn(Either<A, ()>) -> Option<A>,
        Either<A, ()>,
    >;

    /// The derived parser type of [`ParserExt::ignore`].
    pub type Ignore<P, A> = Map<P, fn(A) -> (), A>;
    /// The derived parser type of [`ParserExt::then_ignore`].
    pub type ThenIgnore<L, R, A, B> = Map<Then<L, R>, fn((A, B)) -> A, (A, B)>;
    /// The derived parser type of [`ParserExt::ignore_then`].
    pub type IgnoreThen<L, R, A, B> = Map<Then<L, R>, fn((A, B)) -> B, (A, B)>;

    /// The derived parser type of [`ParserExt::either_or`].
    pub type EitherOr<L, R, A, B> =
        Or<Map<L, fn(A) -> Either<A, B>, A>, Map<R, fn(B) -> Either<A, B>, B>>;
    /// The derived parser type of [`ParserExt::then_commit_to`].
    pub type ThenCommitTo<L, R> = Then<L, Commit<R>>;
}

/// Convenience trait for [`.into_owned()`](Self::into_owned) method chaining.
pub trait ParserIntoOwnedExt<'src, I: Input + ?Sized, A, E: ParseEnvironment = environment::Rich>:
    Parser<'src, I, &'src A, E>
where
    A: ToOwned + ?Sized + 'src,
{
    /// Chains `self` by transforming an AST payload reference into owned data.
    fn into_owned(self) -> IntoOwned<Self, A>
    where
        Self: Sized,
    {
        IntoOwned::new(self)
    }
}
impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, T> ParserIntoOwnedExt<'src, I, A, E> for T
where
    T: Parser<'src, I, &'src A, E>,
    A: ToOwned + ?Sized + 'src,
{
}
