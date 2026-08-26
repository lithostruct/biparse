use std::{fmt::Debug, hash::Hash};

use crate::cst::path::CstPath;

/// Association table between an `Id` type and a CstOutput containing GreenNodes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CstAssociation<Id>
where
    Id: Eq + Hash,
{
    pub(crate) inner: im::HashMap<Id, CstPath>,
}
impl<Id> Default for CstAssociation<Id>
where
    Id: Eq + Hash,
{
    fn default() -> Self {
        CstAssociation {
            inner: im::HashMap::new(),
        }
    }
}
impl<Id> CstAssociation<Id>
where
    Id: Debug + Clone + Eq + Hash,
{
    pub fn new() -> Self {
        CstAssociation::default()
    }
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
        CstAssociation { inner: merged }
    }
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
