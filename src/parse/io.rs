use crate::{
    combinator::RecursiveState,
    cst::{AssociatedGreenCst, IdSupply},
    green::GreenInterner,
    input::Input,
    parse::ParseEnvironment,
};
use std::{cell::RefCell, fmt::Debug, hash::Hash, rc::Rc};

/// The parse state to be passed and returned by all parser combinators.
pub struct ParseState<'src, C, S: IdSupply> {
    /// The current cursor position.
    pub cursor: C,
    /// A supply of type `S`.
    pub supply: S,
    /// A reference-counted shared interner, allowing for clones to access the same structure.
    ///
    /// Direct usage is discouraged. The interning methods of [`GreenInterner`]
    /// may be accessed via [`ParseState::with_interner`],
    /// which holds the mutable borrow for as short as possible.
    ///
    /// # Panics
    ///
    /// This field comes with the usual pitfalls of [`Rc<RefCell<...>>`];
    /// holding a mutable borrow and attempting subsequent borrowing will cause panicking.
    pub interner: Rc<RefCell<GreenInterner>>,
    /// The current recursive stack.
    ///
    /// Do not discard the recursive stack during parsing. This may lead to undefined behaviour.
    pub recursive_state: RecursiveState<'src, C, S>,
}
impl<'src, C, S: IdSupply> Debug for ParseState<'src, C, S>
where
    C: Debug,
    S: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("ParseState");

        debug.field("cursor", &self.cursor);

        // This is a whole thing because we cannot get an immutable view
        // if the interner is currently mutably-borrowed (results in panic naively).
        // This shouldn't be a problem via API usage of this type,
        // but the field is public (for end-users to be able to reconstruct ParseState)
        // so anything goes.
        // I would rather `Debug` not panic in this scenario.
        match self.interner.try_borrow() {
            Ok(borrowed) => debug.field("interner", &borrowed as &GreenInterner),
            Err(_) => debug.field("interner", &"<borrowed>"),
        };

        debug.field("supply", &self.supply).finish()
    }
}
impl<'src, C: Clone, S: IdSupply> Clone for ParseState<'src, C, S> {
    fn clone(&self) -> Self {
        ParseState {
            cursor: self.cursor.clone(),
            interner: self.interner.clone(),
            supply: self.supply.clone(),
            recursive_state: self.recursive_state.clone(),
        }
    }
}
impl<'src, C, S: IdSupply + Default> ParseState<'src, C, S> {
    /// Creates a new parse state from this input.
    pub fn new<I: Input + ?Sized>(input: &I) -> ParseState<'_, I::Cursor, S> {
        ParseState {
            cursor: input.start(),
            interner: Rc::new(RefCell::new(GreenInterner::new())),
            supply: S::default(),
            recursive_state: RecursiveState::new(),
        }
    }

    /// Executes a closure in which the interner is borrowed mutably.
    ///
    /// Prefer this wherever you may write `self.interner.borrow_mut()` instead. This avoid runtime panics
    /// caused by holding a mutable reference into [`Rc<RefCell<_>>`] while attempting subsequent borrowing.
    pub fn with_interner<F, A>(&self, closure: F) -> A
    where
        F: FnOnce(&mut GreenInterner) -> A,
    {
        (closure)(&mut self.interner.borrow_mut())
    }
}

/// Parse output produced by [`Parser`](crate::parse::Parser) implementors.
pub struct ParseOutput<A, E: ParseEnvironment> {
    pub ast: ParseOutcome<A, E::Failure>,
    pub cst: AssociatedGreenCst<<E::Supply as IdSupply>::Id>,
}
impl<A, E: ParseEnvironment> Debug for ParseOutput<A, E>
where
    A: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParseOutput")
            .field("ast", &self.ast)
            .field("cst", &self.cst)
            .finish()
    }
}
impl<A, E: ParseEnvironment> Clone for ParseOutput<A, E>
where
    A: Clone,
{
    fn clone(&self) -> Self {
        ParseOutput {
            ast: self.ast.clone(),
            cst: self.cst.clone(),
        }
    }
}
impl<A, E: ParseEnvironment> PartialEq for ParseOutput<A, E>
where
    A: PartialEq,
    E::Failure: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.ast == other.ast && self.cst == other.cst
    }
}
impl<A, E: ParseEnvironment> Eq for ParseOutput<A, E>
where
    A: Eq,
    E::Failure: Eq,
{
}

pub type ParseResult<A, E> = Result<ParseArtifact<A, E>, <E as ParseEnvironment>::Failure>;
impl<A, E: ParseEnvironment> ParseOutput<A, E> {
    /// Converts self into a [`Result<ParseArtifact<A, E>, ...>`],
    /// erasing the distinction between speculative and committed failure.
    ///
    /// This is typically used for top-level parser output;
    /// see [`ParserExt::parse_all`](crate::parse::ParserExt::parse_all)
    /// and [`ParserExt::check_all`](crate::parse::ParserExt::check_all).
    pub fn into_result(self) -> ParseResult<A, E> {
        self.ast
            .into_result()
            .map(|ast| ParseArtifact { ast, cst: self.cst })
    }
}

/// The top-level payload of a successful parse.
/// Converted from and has the same parse information as `ParseOutput`,
/// with accessor functions for end-use.
pub struct ParseArtifact<A, E: ParseEnvironment> {
    ast: A,
    cst: AssociatedGreenCst<<E::Supply as IdSupply>::Id>,
}

impl<A, E: ParseEnvironment> Debug for ParseArtifact<A, E>
where
    A: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParseReport")
            .field("ast", &self.ast)
            .field("cst", &self.cst)
            .finish()
    }
}
impl<A, E: ParseEnvironment> Clone for ParseArtifact<A, E>
where
    A: Clone,
{
    fn clone(&self) -> Self {
        ParseArtifact {
            ast: self.ast.clone(),
            cst: self.cst.clone(),
        }
    }
}
impl<A, E: ParseEnvironment> PartialEq for ParseArtifact<A, E>
where
    A: PartialEq,
    E::Failure: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.ast == other.ast && self.cst == other.cst
    }
}
impl<A, E: ParseEnvironment> Eq for ParseArtifact<A, E>
where
    A: Eq,
    E::Failure: Eq,
{
}

impl<A, E: ParseEnvironment> ParseArtifact<A, E> {
    /// Provides an immutable view into the AST payload only.
    pub fn ast_only(&self) -> &A {
        &self.ast
    }
    /// Consumes `self` into the AST payload only.
    ///
    /// To obtain both AST and CST components, use [`Self::into_parts`].
    pub fn into_ast_only(self) -> A {
        self.ast
    }
    /// Provides an immutable view into the CST payload.
    pub fn cst(&self) -> &AssociatedGreenCst<<E::Supply as IdSupply>::Id> {
        &self.cst
    }
    /// Consumes `self` into the CST payload.
    ///
    /// To obtain both AST and CST components, use [`Self::into_parts`].
    pub fn into_cst(self) -> AssociatedGreenCst<<E::Supply as IdSupply>::Id> {
        self.cst
    }
    /// Consumes `self` and returns its constituent parts.
    ///
    /// Note that diagnostic information is contained within the CST component
    /// due to coupling with CST path-based finding and resolution.
    pub fn into_parts(self) -> (A, AssociatedGreenCst<<E::Supply as IdSupply>::Id>) {
        (self.ast, self.cst)
    }
}

/// Status of parse result of the AST payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParseOutcome<A, F> {
    /// A successful parse producing an AST payload.
    Success(A),
    /// Speculative failure.
    /// The combinator system is free to try out parsing alternatives,
    /// and a later success will cause this value to be discarded.
    Speculative(F),
    /// Committed failure.
    /// The combinator system may not try other alternatives.
    /// This failure is propagated outwards until explicitly handled.
    Committed(F),
}
impl<A, F> ParseOutcome<A, F> {
    /// Converts `self` into a [`Result`], erasing the parser-internal distinction between speculative and committed failure.
    ///
    /// This is typically used for top-level parser output.
    pub fn into_result(self) -> Result<A, F> {
        match self {
            ParseOutcome::Success(success) => Ok(success),
            ParseOutcome::Speculative(failure) | ParseOutcome::Committed(failure) => Err(failure),
        }
    }
    /// Maps the [`Self::Success`] payload to a different type.
    pub fn map<B>(self, f: impl FnOnce(A) -> B) -> ParseOutcome<B, F> {
        match self {
            ParseOutcome::Success(inner) => ParseOutcome::Success(f(inner)),
            ParseOutcome::Speculative(failure) => ParseOutcome::Speculative(failure),
            ParseOutcome::Committed(failure) => ParseOutcome::Committed(failure),
        }
    }
    /// Maps the [`Self::Speculative`] and [`Self::Committed`] failure payloads to a different type.
    pub fn map_err<F2>(self, f: impl FnOnce(F) -> F2) -> ParseOutcome<A, F2> {
        match self {
            ParseOutcome::Success(inner) => ParseOutcome::Success(inner),
            ParseOutcome::Speculative(failure) => ParseOutcome::Speculative(f(failure)),
            ParseOutcome::Committed(failure) => ParseOutcome::Committed(f(failure)),
        }
    }
}
