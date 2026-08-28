use std::{fmt::Debug, hash::Hash};

use rowan::{NodeOrToken, SyntaxKind, SyntaxNode};

use crate::{
    cst::{Cst, CstAssociation, CstPath},
    green::{GreenElement, GreenInterner},
};

/// The output of CST construction, representing a green CST forest.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CstOutput<Id>
where
    Id: Eq + Hash,
{
    /// The forest of CST nodes.
    pub green: Cst<GreenElement>,
    /// The association table between `Id` and paths into `self.cst` to get those nodes.
    /// Named `origin` due to origin-tracking.
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

    /// Associatively merges with `self` as the left-hand-side, and a given right-hand-side.
    ///
    /// # Panics
    ///
    /// Panics if the [`IdSupply`](crate::cst::supply::IdSupply) protocol is violated,
    /// such that there exists overlapping entries in both origin tables.
    pub fn merge_with(self, rhs: Self) -> Self {
        let merged_origin = self.origin.merge_with(rhs.origin, self.green.len());
        let merged_forest = self.green + rhs.green;
        CstOutput {
            green: merged_forest,
            origin: merged_origin,
        }
    }
    /// Creates a new parent taking the forest of `self` as children.
    pub fn attach_parent(self, kind: SyntaxKind, interner: &mut GreenInterner) -> Self {
        let new_origin = self.origin.with_new_parent(0);
        let new_node = interner.node(kind, self.green);
        CstOutput {
            green: Cst::unit(new_node.into()),
            origin: new_origin,
        }
    }

    /// Associates a root node at the given index of the forest with the corresponding Id.
    ///
    /// # Panics
    ///
    /// Panics if `index >= self.green.len()` or if `with_id` already exists in the origin table.
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

    /// Canonizes `self` as a syntax tree, returning the materialized red node and origin table as an [`im::HashMap`].
    ///
    /// Returns `None` if `self.cst.len != 1`, or the current root is a `GreenToken`.
    pub fn canonize<L: rowan::Language>(
        &self,
    ) -> Option<(SyntaxNode<L>, im::HashMap<Id, CstPath>)> {
        // Preconditions.
        if self.green.len() != 1 || matches!(self.green[0], NodeOrToken::Token(_)) {
            return None;
        }

        let root = self.green[0]
            .as_node()
            .cloned()
            .expect("Preconditions met!");
        let red_tree = SyntaxNode::<L>::new_root(root);
        Some((red_tree, self.origin.inner().clone()))
    }
    /// Canonizes `self` as a syntax tree using the given kind as the root node,
    /// returning the materialized red node and origin table as an [`im::HashMap`].
    pub fn canonize_with<L: rowan::Language>(
        self,
        root_kind: SyntaxKind,
    ) -> (SyntaxNode<L>, im::HashMap<Id, CstPath>) {
        // It doesn't matter that the GreenInterner here is bogus.
        // We don't need to deduplicate the root node.
        let rooted = self.attach_parent(root_kind, &mut GreenInterner::new());
        rooted.canonize().expect("Just appended root node!")
    }
}
