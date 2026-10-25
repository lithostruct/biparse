use std::{
    collections::{VecDeque, vec_deque},
    iter::*,
};

use rowan::{GreenNode, Language, api::SyntaxNode};
use thiserror::Error;

use crate::{cst::Cst, green::GreenElement};

/// The path through a forest of CST nodes.
///
/// Not recommended for direct use. Use [`AssociatedGreenCst`](crate::cst::AssociatedGreenCst)
/// or [`AssociatedRedCst`](crate::cst::AssociatedRedCst) where possible instead.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CstPath(usize, VecDeque<usize>);
impl CstPath {
    /// Constructs a path pointing to the `index`-th root position.
    pub fn here_at(index: usize) -> Self {
        CstPath(index, VecDeque::new())
    }
    /// Extends the path with a new parent at the specified root index, taking `self` as a child.
    pub fn with_new_parent(mut self, at: usize) -> Self {
        self.1.push_front(self.0);
        self.0 = at;
        self
    }
    /// Extends the path with a new parent at the specified root index, taking `self` as a child.
    pub fn with_new_parent_mut(&mut self, at: usize) {
        self.1.push_front(self.0);
        self.0 = at;
    }
    /// Shifts the path of `self` to the right by an offset of `width`.
    pub fn shift_left_by(mut self, width: usize) -> Self {
        self.0 += width;
        self
    }
    /// Shifts the path of `self` to the right by an offset of `width`.
    pub fn shift_left_mut(&mut self, width: usize) {
        self.0 += width;
    }

    /// Borrows the components of `CstPath`.
    pub const fn as_parts(&self) -> (usize, &VecDeque<usize>) {
        (self.0, &self.1)
    }
    /// Deconstructs and returns the components of `CstPath`.
    pub fn into_parts(self) -> (usize, VecDeque<usize>) {
        (self.0, self.1)
    }
    /// Constructs and returns a `CstPath` from its components.
    ///
    /// This method bypasses the construction invariants that [`AssociatedGreenCst`](crate::cst::AssociatedGreenCst)
    /// enforces and relies upon.
    /// Direct use is not recommended.
    pub fn from_parts(root_index: usize, subpath: VecDeque<usize>) -> Self {
        CstPath(root_index, subpath)
    }

    /// Creates a non-consuming iterator for `self`.
    ///
    /// You may use the consuming iterator produced by [`IntoIterator::into_iter`] instead if desired.
    pub fn iter(&self) -> impl Iterator<Item = usize> {
        let deref: fn(&usize) -> usize = |u: &usize| *u;
        std::iter::once(self.0).chain(self.1.iter().map(deref))
    }
}
impl IntoIterator for CstPath {
    type Item = usize;
    type IntoIter = Chain<Once<usize>, vec_deque::IntoIter<usize>>;

    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self.0).chain(self.1)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PathfindError {
    #[error("expected node, got token")]
    TokenNotNode { path: CstPath, depth: usize },
    #[error("index of {indexed:} at {depth:} but got forest of width {got_width:}")]
    OutOfBounds {
        path: CstPath,
        depth: usize,
        indexed: usize,
        got_width: usize,
    },
}

/// Implementation that allows for the index-based `CstPath` to identify a specific node within this forest.
pub trait Pathfind {
    /// The output type associated with the search operation.
    type Output;
    /// Search within `self` with the given path, returning a successful match or a pathfinding error.
    fn search(&self, path: &CstPath) -> Result<Self::Output, PathfindError>;
}

impl Pathfind for Cst<GreenElement> {
    type Output = GreenNode;

    fn search(&self, path: &CstPath) -> Result<Self::Output, PathfindError> {
        let (root_index, subpath) = path.as_parts();

        // First search is done separate because `Vector<...>` and `GreenNodeData` have distinct indexing mechanisms.
        let mut node: &rowan::GreenNodeData = self
            .get(root_index)
            .ok_or_else(|| PathfindError::OutOfBounds {
                path: path.clone(),
                depth: 0,
                indexed: root_index,
                got_width: self.len(),
            })?
            .as_node()
            .ok_or_else(|| PathfindError::TokenNotNode {
                path: path.clone(),
                depth: 0,
            })?;

        for (depth, &index) in subpath.iter().enumerate() {
            let child = node
                .children()
                .nth(index)
                .ok_or_else(|| PathfindError::OutOfBounds {
                    path: path.clone(),
                    depth: depth + 1,
                    indexed: index,
                    got_width: node.children().len(),
                })?;
            node = child
                .into_node()
                .ok_or_else(|| PathfindError::TokenNotNode {
                    path: path.clone(),
                    depth: depth + 1,
                })?;
        }
        Ok(node.to_owned())
    }
}

impl<L: Language> Pathfind for SyntaxNode<L> {
    type Output = SyntaxNode<L>;

    fn search(&self, path: &CstPath) -> Result<Self::Output, PathfindError> {
        let (root_index, subpath) = path.as_parts();
        // check that path does not describe forest (an actualized red tree is always single).
        if root_index != 0 {
            return Err(PathfindError::OutOfBounds {
                path: path.clone(),
                depth: 0,
                indexed: root_index,
                got_width: 1,
            });
        }
        // This clone is cheap. All `SyntaxNode`s are refcounted.
        let mut node = self.clone();

        for (depth, &index) in subpath.iter().enumerate() {
            let child = node.children_with_tokens().nth(index).ok_or_else(|| {
                PathfindError::OutOfBounds {
                    path: path.clone(),
                    depth: depth + 1,
                    indexed: index,
                    got_width: node.children_with_tokens().count(),
                }
            })?;
            node = child
                .into_node()
                .ok_or_else(|| PathfindError::TokenNotNode {
                    path: path.clone(),
                    depth: depth + 1,
                })?;
        }
        Ok(node)
    }
}
