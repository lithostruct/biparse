use std::{fmt::Debug, hash::Hash};

use derive_more::{Deref, Into};

use crate::cst::path::CstPath;

/// Association table between an `Id` type and paths to a coupled green forest.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Into, Deref)]
pub(super) struct AssociationTable<Id>
where
    Id: Eq + Hash,
{
    #[into]
    pub(super) inner: im::HashMap<Id, CstPath>,
}
impl<Id> Default for AssociationTable<Id>
where
    Id: Eq + Hash,
{
    fn default() -> Self {
        AssociationTable::new()
    }
}
impl<Id> AssociationTable<Id>
where
    Id: Eq + Hash,
{
    /// Creates an empty association table.
    pub fn new() -> Self {
        AssociationTable {
            inner: im::HashMap::new(),
        }
    }

    /// Immutably borrows the inner [`im::HashMap`].
    pub const fn inner(&self) -> &im::HashMap<Id, CstPath> {
        &self.inner
    }
    /// Converts self into the inner [`im::HashMap`].
    pub fn into_inner(self) -> im::HashMap<Id, CstPath> {
        self.inner
    }
}

impl<Id> AssociationTable<Id>
where
    Id: Debug + Clone + Eq + Hash,
{
    /// Associatively merges with `self` as the left-hand-side, and a given right-hand-side.
    ///
    /// # Panics
    ///
    /// Panics if the [`IdSupply`](crate::cst::supply::IdSupply) protocol is violated,
    /// such that there exists overlapping entries in both origin tables.
    pub fn merge_with(self, mut rhs: Self, left_width: usize) -> Self {
        rhs.inner
            .iter_mut()
            .map(|(_, path)| {
                path.shift_left_mut(left_width);
            })
            .for_each(drop);
        let right_pair = rhs.inner;
        let left_pair = self.inner;
        let merged = left_pair.union_with_key(right_pair, |key, _, _| {
            panic!("Non-unique key {key:?} found in merge operation!")
        });
        AssociationTable { inner: merged }
    }
    /// Updates all paths to be prepended by a new parent, which takes the whole forest as children.
    pub fn with_new_parent(mut self, at_index: usize) -> Self {
        self.inner
            .iter_mut()
            .map(|(_, path)| {
                path.with_new_parent_mut(at_index);
            })
            .for_each(drop);
        self
    }
}
