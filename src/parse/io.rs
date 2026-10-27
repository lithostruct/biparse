pub use crate::combinator::RecursiveState;
use crate::{
    cst::{AssociatedGreenCst, IdSupply},
    green::GreenInterner,
    input::Input,
    parse::ParseEnvironment,
};
use std::{cell::RefCell, fmt::Debug, hash::Hash, rc::Rc};

/// The parse state to be passed and returned by all parser combinators.
///
/// Prefer the usage of the [`ParseState`](crate::parse::ParseState) alias where applicable,
/// which is equivalent but has more suitable associated types for use in [`Parser`](crate::parse::Parser) implementors.
pub struct ParseStateData<'src, C, S: IdSupply, ES: IdSupply> {
    /// The current cursor position.
    pub cursor: C,
    /// Id supply for associations.
    pub supply: S,
    /// Id supply for recovered errors/diagnostics.
    pub error_supply: ES,
    /// A reference-counted shared interner, allowing for clones to access the same structure.
    ///
    /// Direct usage is discouraged. The interning methods of [`GreenInterner`]
    /// may be accessed via [`Self::with_interner`],
    /// which holds the mutable borrow for as short as possible.
    ///
    /// # Panics
    ///
    /// This field comes with the usual pitfalls of [`Rc<RefCell<...>>`];
    /// holding a mutable borrow and attempting subsequent borrowing will cause panicking.
    pub interner: Rc<RefCell<GreenInterner>>,
    /// The current recursive stack.
    ///
    /// Do not discard the recursive stack during a parse operation,
    /// or retain the recursive stack across independent parse operations.
    /// This may lead to undefined behaviour.
    pub recursive_state: RecursiveState<'src, C, S, ES>,
}
impl<'src, C, S: IdSupply, ES: IdSupply> Debug for ParseStateData<'src, C, S, ES>
where
    C: Debug,
    S: Debug,
    ES: Debug,
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

        debug
            .field("supply", &self.supply)
            .field("error_supply", &self.error_supply)
            .finish()
    }
}
impl<'src, C: Clone, S: IdSupply, ES: IdSupply> Clone for ParseStateData<'src, C, S, ES> {
    fn clone(&self) -> Self {
        ParseStateData {
            cursor: self.cursor.clone(),
            interner: self.interner.clone(),
            supply: self.supply.clone(),
            error_supply: self.error_supply.clone(),
            recursive_state: self.recursive_state.clone(),
        }
    }
}
impl<'src, C, S: IdSupply + Default, ES: IdSupply + Default> ParseStateData<'src, C, S, ES> {
    /// Creates a new parse state from this input.
    pub fn new<I: Input + ?Sized>(input: &I) -> ParseStateData<'src, I::Cursor, S, ES> {
        ParseStateData {
            cursor: input.start(),
            interner: Rc::new(RefCell::new(GreenInterner::new())),
            supply: S::default(),
            error_supply: ES::default(),
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
    pub cst: AssociatedGreenCst<
        <E::Supply as IdSupply>::Id,
        <E::ErrorSupply as IdSupply>::Id,
        E::Failure,
    >,
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

impl<A, E: ParseEnvironment> ParseOutput<A, E> {
    /// Consumes `self`, lifting parse failure out of the AST component into a [`Result`] value.
    /// Speculative and commited failure are promoted to [`TerminalFailure`].
    ///
    /// This is typically used for top-level parser output;
    /// see [`ParserExt::parse_all`](crate::parse::ParserExt::parse_all)
    /// and [`ParserExt::check_all`](crate::parse::ParserExt::check_all).
    pub fn into_lifted(self) -> Result<ParseArtifact<A, E>, TerminalFailure<E::Failure>> {
        match self.ast.into_result() {
            Ok(ast) => Ok(ParseArtifact { ast, cst: self.cst }),
            Err(terminating) => Err(TerminalFailure {
                terminating,
                supplemental: self.cst.into_raw_diagnostics(),
            }),
        }
    }
}

/// Intermediate outcome of a parse on the AST payload.
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
    /// Converts `self` into a [`Result`], erasing the outcome distinction between speculative and committed failure.
    ///
    /// This is typically used for top-level parser output.
    pub fn into_result(self) -> Result<A, F> {
        match self {
            ParseOutcome::Success(success) => Ok(success),
            ParseOutcome::Speculative(failure) | ParseOutcome::Committed(failure) => Err(failure),
        }
    }
    /// Transforms the [`Self::Success`] payload via the given closure.
    pub fn map<B>(self, f: impl FnOnce(A) -> B) -> ParseOutcome<B, F> {
        match self {
            ParseOutcome::Success(inner) => ParseOutcome::Success(f(inner)),
            ParseOutcome::Speculative(failure) => ParseOutcome::Speculative(failure),
            ParseOutcome::Committed(failure) => ParseOutcome::Committed(failure),
        }
    }
    /// Transforms the [`Self::Speculative`] and [`Self::Committed`] failure payloads via the given closure.
    pub fn map_err<F2>(self, f: impl FnOnce(F) -> F2) -> ParseOutcome<A, F2> {
        match self {
            ParseOutcome::Success(inner) => ParseOutcome::Success(inner),
            ParseOutcome::Speculative(failure) => ParseOutcome::Speculative(f(failure)),
            ParseOutcome::Committed(failure) => ParseOutcome::Committed(f(failure)),
        }
    }
}

/// The top-level error of a failed parse.
/// Represents irrevocable and premature termination of parsing.
///
/// Converted from [`ParseOutput::into_result`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TerminalFailure<T> {
    /// The main parse failure that caused termination of parsing.
    pub terminating: T,
    /// Supplemental diagnostics from normal parsing prior to termination.
    pub supplemental: Vec<T>,
}

/// The top-level payload of a successful parse.
///
/// Converted from [`ParseOutput::into_result`] and contains near-similar parse information,
/// with easier-to-use accessor functions.
pub struct ParseArtifact<A, E: ParseEnvironment> {
    ast: A,
    cst: AssociatedGreenCst<
        <E::Supply as IdSupply>::Id,
        <E::ErrorSupply as IdSupply>::Id,
        E::Failure,
    >,
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
    /// Materializes an immutable view into the AST payload and raw diagnostics.
    ///
    /// To view only the AST component, ignoring diagnostics, use [`Self::ast_only`].
    ///
    /// To view only diagnostics, use [`Self::diagnostics`].
    pub fn ast(&self) -> (&A, Vec<&E::Failure>) {
        (&self.ast, self.diagnostics())
    }
    /// Provides an immutable view into the AST payload only.
    ///
    /// This method discards diagnostics.
    /// To view both the AST component *and* diagnostics, use [`Self::ast`].
    pub fn ast_only(&self) -> &A {
        &self.ast
    }
    /// Materializes immutable access into raw diagnostics.
    pub fn diagnostics(&self) -> Vec<&E::Failure> {
        self.cst().diagnostics().values().map(|(f, _)| f).collect()
    }
    /// Consumes `self` to produce the AST payload and diagnostics.
    ///
    /// To obtain only the AST component, discarding diagnostics, use [`Self::into_ast_only`].
    ///
    /// To obtain only diagnostics, use [`Self::into_diagnostics`].
    ///
    /// To obtain both AST and CST components, use [`Self::into_parts`].
    pub fn into_ast(self) -> (A, Vec<E::Failure>) {
        (self.ast, self.cst.into_raw_diagnostics())
    }
    /// Consumes `self` into the AST payload only.
    ///
    /// This method discards diagnostics. To obtain both the AST component *and* diagnostics, use [`Self::into_ast`].
    ///
    /// To obtain both AST and CST components, use [`Self::into_parts`].
    pub fn into_ast_only(self) -> A {
        self.ast
    }
    /// Consumes `self` into raw diagnostics.
    pub fn into_diagnostics(self) -> Vec<E::Failure> {
        self.cst.into_raw_diagnostics()
    }

    /// Provides an immutable view into the CST payload.
    pub fn cst(
        &self,
    ) -> &AssociatedGreenCst<
        <E::Supply as IdSupply>::Id,
        <E::ErrorSupply as IdSupply>::Id,
        E::Failure,
    > {
        &self.cst
    }
    /// Consumes `self` into the CST payload.
    ///
    /// To obtain both AST and CST components, use [`Self::into_parts`].
    pub fn into_cst(
        self,
    ) -> AssociatedGreenCst<<E::Supply as IdSupply>::Id, <E::ErrorSupply as IdSupply>::Id, E::Failure>
    {
        self.cst
    }

    /// Consumes `self` and returns its constituent AST and CST components.
    ///
    /// Note that diagnostic information is contained within the CST component
    /// due to coupling with CST path-based finding and resolution.
    #[allow(clippy::type_complexity)]
    pub fn into_parts(
        self,
    ) -> (
        A,
        AssociatedGreenCst<
            <E::Supply as IdSupply>::Id,
            <E::ErrorSupply as IdSupply>::Id,
            E::Failure,
        >,
    ) {
        (self.ast, self.cst)
    }
}
