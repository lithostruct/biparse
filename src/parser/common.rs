use rowan::SyntaxKind;

use crate::{
    green::green_interner::{GreenElement, GreenInterner}, input::Input, parser::{error::{self, ParserFailure}},
};
use std::{collections::VecDeque, fmt::Debug, hash::Hash, marker::PhantomData, ops::Add};

/// A protocol for supplying unique `Id`'s to a parser combinator.
pub trait IdSupply : Clone {
    type Id : Debug + Clone + Eq + Hash;
    fn next_id(&mut self) -> Self::Id;
}
/// A [`Supply`] implementor that wraps a newtype around a monotonically-increasing [`usize`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewtypeWrapper<T>(usize, PhantomData<T>);
impl<T: From<usize>> Default for NewtypeWrapper<T> {
    fn default() -> Self {
        NewtypeWrapper(0, PhantomData)
    }
}
impl<T: From<usize>> Iterator for NewtypeWrapper<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        let output = self.0;
        self.0 += 1;
        Some(T::from(output))
    }
}
impl<T: From<usize> + Debug + Clone + Eq + Hash> IdSupply for NewtypeWrapper<T> {
    type Id = T;
    fn next_id(&mut self) -> Self::Id {
        let output = self.0;
        self.0 += 1;
        T::from(output)
    }
}

pub struct ParseState<I: Input, S : IdSupply> {
    pub cursor: I::Cursor,
    pub interner: GreenInterner,
    pub supply : S,
}
impl<I: Input, S : IdSupply + Default> ParseState<I, S> {
    pub fn new(input: &I) -> Self {
        ParseState {
            cursor: input.start(),
            interner: GreenInterner::new(),
            supply: S::default(),
        }
    }
}

pub type Cst<T> = im::Vector<T>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseOutput<A, F, Id>
where
    Id: Debug + Clone + Eq + Hash,
{
    pub ast: ParseResult<A, F>,
    pub cst: CstOutput<Id>,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParseResult<A, F> {
    Success(A),
    Speculative(F),
    Propagating(F),
}
impl<A, F> ParseResult<A, F> {
    pub fn into_result(self) -> Result<A, F> {
        match self {
            ParseResult::Success(success) => Ok(success),
            ParseResult::Speculative(failure)
            | ParseResult::Propagating(failure) => Err(failure),
        }
    }
    pub fn map<B>(self, f: impl FnOnce(A) -> B) -> ParseResult<B, F> {
        match self {
            ParseResult::Success(inner) => ParseResult::Success(f(inner)),
            ParseResult::Speculative(failure) => ParseResult::Speculative(failure),
            ParseResult::Propagating(failure) => ParseResult::Propagating(failure),
        }
    }
    pub fn map_err<F2>(self, f: impl FnOnce(F) -> F2) -> ParseResult<A, F2> {
        match self {
            ParseResult::Success(inner) => ParseResult::Success(inner),
            ParseResult::Speculative(failure) => ParseResult::Speculative(f(failure)),
            ParseResult::Propagating(failure) => ParseResult::Propagating(f(failure)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CstOutput<Id>
where
    Id: Eq + Hash,
{
    pub cst: Cst<GreenElement>,
    pub origin: Association<Id>,
}
impl<Id: Debug + Clone + Eq + Hash> CstOutput<Id> {
    pub fn new() -> Self {
        CstOutput { cst: Cst::new(), origin: Association::new() }
    }
    pub fn unit(value : GreenElement) -> Self {
        CstOutput { cst: Cst::unit(value), origin: Association::new() }
    }

    pub fn merge_with(self, rhs: Self) -> Self {
        let merged_origin = self.origin.merge_with(rhs.origin, self.cst.len());
        let merged_forest = self.cst + rhs.cst;
        CstOutput { cst: merged_forest, origin: merged_origin }
    }
    pub fn attach_parent(self, kind: SyntaxKind, interner: &mut GreenInterner) -> Self {
        let new_origin = self.origin.with_new_parent(0);
        let new_node = interner.node(kind, self.cst);
        CstOutput { cst: Cst::unit(new_node.into()), origin: new_origin }
    }

    pub fn associate_root_at(mut self, index: usize, with_id: Id) -> Self {
        if index >= self.cst.len() {
            panic!("Cannot find node {index}; forest is of length {}!", self.cst.len())
        }
        match self.origin.inner.insert(with_id.clone(), CstPath::here_at(index)) {
            None => (),
            Some(existing) => panic!("IdSupply invariant invalidated: {with_id:?} has existing mapping to {existing:?}!"),
        }
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CstPath(usize, VecDeque<usize>);
impl CstPath {
    pub fn here_at(index: usize) -> Self {
        CstPath(index, VecDeque::new())
    }
    pub fn with_new_parent(mut self, at: usize) -> Self {
        self.1.push_front(self.0);
        self.0 = at;
        self
    }
    pub fn with_new_parent_mut(&mut self, at: usize) {
        self.1.push_front(self.0);
        self.0 = at;
    }
    pub fn shift_left_by(mut self, width: usize) -> Self {
        self.0 += width;
        self
    }
    pub fn shift_left_mut(&mut self, width: usize) {
        self.0 += width;
    }
}

/// Struct controlling the association between an Id type and a C containing GreenNodes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Association<Id>
where
    Id: Eq + Hash,
{
    pub(crate) inner: im::HashMap<Id, CstPath>,
}
impl<Id> Default for Association<Id>
where
    Id: Eq + Hash,
{
    fn default() -> Self {
        Association {
            inner: im::HashMap::new(),
        }
    }
}
impl<Id> Association<Id>
where
    Id: Debug + Clone + Eq + Hash,
{
    pub fn new() -> Self {
        Association::default()
    }
    pub fn merge_with(self, mut rhs: Self, left_width: usize) -> Self {
        rhs.inner.iter_mut()
            .map(|(_, path)| {
                path.shift_left_mut(left_width);
            })
            .for_each(drop);
        let right_pair = rhs.inner;
        let left_pair = self.inner;
        let merged = left_pair.union_with_key(right_pair, 
            |key, _, _| panic!("Non-unique key {key:?} found in merge operation!"));
        Association { inner: merged }
    }
    pub fn with_new_parent(mut self, at_index: usize) -> Self {
        self.inner.iter_mut()
            .map(|(_, path)| {
                path.with_new_parent_mut(at_index);
            })
            .for_each(drop);
        self
    }
}

pub trait ParseEnvironment {
    type Failure: ParserFailure;
    type Id: Debug + Clone + Eq + Hash;
}
pub struct DefaultParseEnvironment;
impl ParseEnvironment for DefaultParseEnvironment {
    type Failure = error::Simple;
    type Id = u64;
}

pub trait Parser<'src, I : Input, A, E : ParseEnvironment = DefaultParseEnvironment> {
    fn parse<S : IdSupply<Id = E::Id>>(
        &self,
        input: &'src I,
        state: ParseState<I, S>,
    ) -> (ParseState<I, S>, ParseOutput<A, E::Failure, E::Id>);
    fn check<S : IdSupply<Id = E::Id>>(
        &self,
        input: &'src I,
        state: ParseState<I, S>,
    ) -> (ParseState<I, S>, ParseOutput<(), E::Failure, E::Id>) {
        let (new_state, output) = self.parse(input, state);
        let erased_output = ParseOutput {
            ast: output.ast.map(|_| ()),
            cst: CstOutput::new(),
        };
        (new_state, erased_output)
    }
}
