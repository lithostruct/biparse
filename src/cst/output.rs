use std::{fmt::Debug, hash::Hash};

use rowan::SyntaxKind;

use crate::{
    cst::{CstAssociation, Cst, CstPath},
    green::{GreenElement, GreenInterner},
};

/// The output of CST construction.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CstOutput<Id>
where
    Id: Eq + Hash,
{
    /// The forest of CST nodes.
    pub green: Cst<GreenElement>,
    /// The association table between `Id` and paths into `self.cst` to get those nodes.
    pub origin: CstAssociation<Id>,
}
impl<Id: Debug + Clone + Eq + Hash> Default for CstOutput<Id> {
    fn default() -> Self {
        Self::new()
    }
}
impl<Id: Debug + Clone + Eq + Hash> CstOutput<Id> {
    pub fn new() -> Self {
        CstOutput {
            green: Cst::new(),
            origin: CstAssociation::new(),
        }
    }
    pub fn unit(value: GreenElement) -> Self {
        CstOutput {
            green: Cst::unit(value),
            origin: CstAssociation::new(),
        }
    }

    pub fn merge_with(self, rhs: Self) -> Self {
        let merged_origin = self.origin.merge_with(rhs.origin, self.green.len());
        let merged_forest = self.green + rhs.green;
        CstOutput {
            green: merged_forest,
            origin: merged_origin,
        }
    }
    pub fn attach_parent(self, kind: SyntaxKind, interner: &mut GreenInterner) -> Self {
        let new_origin = self.origin.with_new_parent(0);
        let new_node = interner.node(kind, self.green);
        CstOutput {
            green: Cst::unit(new_node.into()),
            origin: new_origin,
        }
    }

    pub fn associate_root_at(mut self, index: usize, with_id: Id) -> Self {
        if index >= self.green.len() {
            panic!(
                "Cannot find node {index}; forest is of length {}!",
                self.green.len()
            )
        }
        match self
            .origin
            .inner
            .insert(with_id.clone(), CstPath::here_at(index))
        {
            None => (),
            Some(existing) => panic!(
                "IdSupply invariant invalidated: {with_id:?} has existing mapping to {existing:?}!"
            ),
        }
        self
    }
}
