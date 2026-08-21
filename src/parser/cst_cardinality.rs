use std::{
    collections::VecDeque, convert::Infallible, fmt::Debug, hash::Hash, marker::PhantomData,
};

use crate::{
    green::green_interner::{GreenElement, GreenElementRef, borrow_element},
    parser::common::Association,
};
use derive_more::{From, Into};
use rowan::NodeOrToken;

mod sealed {
    pub trait SealedMerge {}
    pub trait SealedCstOutput {}
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Zero<T>(PhantomData<T>);
impl<T> Zero<T> {
    pub fn new() -> Self {
        Zero(PhantomData)
    }
}
impl<T> IntoIterator for Zero<T> {
    type IntoIter = std::iter::Empty<T>;
    type Item = T;
    fn into_iter(self) -> Self::IntoIter {
        std::iter::empty()
    }
}
impl<T> sealed::SealedMerge for Zero<T> {}
impl<T> sealed::SealedCstOutput for Zero<T> {}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, From)]
pub struct One<T>(pub T);
impl<T> IntoIterator for One<T> {
    type IntoIter = std::iter::Once<T>;
    type Item = T;
    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self.0)
    }
}
impl<T> sealed::SealedMerge for One<T> {}
impl<T> sealed::SealedCstOutput for One<T> {}

#[derive(Debug, Clone, PartialEq, Eq, Hash, From, Into)]
pub struct Many<T: Clone>(pub(crate) im::Vector<T>);
impl<T: Clone> IntoIterator for Many<T> {
    type IntoIter = im::vector::ConsumingIter<T>;
    type Item = T;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
impl<T: Clone> sealed::SealedMerge for Many<T> {}
impl<T: Clone> sealed::SealedCstOutput for Many<T> {}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TreePath(VecDeque<usize>);
impl TreePath {
    pub fn here() -> Self {
        TreePath(VecDeque::new())
    }
    pub fn into_forest(self, index: usize) -> ForestPath {
        ForestPath(index, self.0)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ForestPath(usize, VecDeque<usize>);
impl ForestPath {
    pub fn with_parent(self) -> TreePath {
        let mut path = self.1;
        path.push_front(self.0);
        TreePath(path)
    }
    pub fn prepended_by(mut self, width: usize) -> Self {
        self.0 += width;
        self
    }
    pub fn prepended_mut(&mut self, width: usize) -> &mut Self {
        self.0 += width;
        self
    }
}

/// This trait is sealed.
pub trait CstOutput: sealed::SealedCstOutput {
    type PathType: std::fmt::Debug + Clone;

    fn find(&self, path: &Self::PathType) -> GreenElementRef<'_>;
    fn into_many(self) -> Many<GreenElement>;
}
impl CstOutput for Zero<GreenElement> {
    type PathType = Infallible;

    fn find(&self, _: &Self::PathType) -> GreenElementRef<'_> {
        unreachable!()
    }
    fn into_many(self) -> Many<GreenElement> {
        Many(im::Vector::new())
    }
}
impl CstOutput for One<GreenElement> {
    type PathType = TreePath;

    fn find(&self, path: &Self::PathType) -> GreenElementRef<'_> {
        let mut item = borrow_element(&self.0);
        for (depth, index) in path.0.iter().enumerate() {
            match item {
                NodeOrToken::Node(node) => {
                    let child = node.children().nth(*index).unwrap_or_else(|| {
                        panic!("Bad path {path:?}: expecting child {index} at depth {depth}!")
                    });
                    item = child;
                }
                NodeOrToken::Token(_) => {
                    unreachable!("Bad path {path:?}: expecting child {index} at depth {depth}!")
                }
            }
        }
        item
    }
    fn into_many(self) -> Many<GreenElement> {
        Many(im::Vector::unit(self.0))
    }
}
impl CstOutput for Many<GreenElement> {
    type PathType = ForestPath;

    fn find(&self, path: &Self::PathType) -> GreenElementRef<'_> {
        let mut item = borrow_element(&self.0[path.0]);
        for (depth, index) in path.1.iter().enumerate() {
            match item {
                NodeOrToken::Node(node) => {
                    let child = node.children().nth(*index).unwrap_or_else(|| {
                        panic!("Bad path {path:?}: expecting child {index} at depth {depth}!")
                    });
                    item = child;
                }
                NodeOrToken::Token(_) => {
                    unreachable!("Bad path {path:?}: expecting child {index} at depth {depth}!")
                }
            }
        }
        item
    }
    fn into_many(self) -> Many<GreenElement> {
        self
    }
}

/// This trait is sealed.
pub trait Merge<Other: CstOutput>: CstOutput + sealed::SealedMerge + Sized {
    type Merged: CstOutput;
    // Associative merge of left and right CSTs and origin tables.
    fn merge<Id: Debug + Clone + Eq + Hash>(
        left: Self,
        left_origin: Association<Id, Self>,
        right: Other,
        right_origin: Association<Id, Other>,
    ) -> (Self::Merged, Association<Id, Self::Merged>);
}

impl Merge<Zero<GreenElement>> for Zero<GreenElement> {
    type Merged = Zero<GreenElement>;
    fn merge<Id: Debug + Clone + Eq + Hash>(
        _: Self,
        _: Association<Id, Self>,
        _: Zero<GreenElement>,
        _: Association<Id, Zero<GreenElement>>,
    ) -> (Self::Merged, Association<Id, Self::Merged>) {
        (Zero::new(), Association::new())
    }
}
impl Merge<One<GreenElement>> for Zero<GreenElement> {
    type Merged = One<GreenElement>;
    fn merge<Id: Debug + Clone + Eq + Hash>(
        _: Self,
        _: Association<Id, Self>,
        right: One<GreenElement>,
        right_origin: Association<Id, One<GreenElement>>,
    ) -> (Self::Merged, Association<Id, Self::Merged>) {
        (right, right_origin)
    }
}
impl Merge<Zero<GreenElement>> for One<GreenElement> {
    type Merged = One<GreenElement>;
    fn merge<Id: Debug + Clone + Eq + Hash>(
        left: Self,
        left_origin: Association<Id, Self>,
        _: Zero<GreenElement>,
        _: Association<Id, Zero<GreenElement>>,
    ) -> (Self::Merged, Association<Id, Self::Merged>) {
        (left, left_origin)
    }
}

impl Merge<One<GreenElement>> for One<GreenElement> {
    type Merged = Many<GreenElement>;
    fn merge<Id: Debug + Clone + Eq + Hash>(
        left: Self,
        left_origin: Association<Id, Self>,
        right: One<GreenElement>,
        right_origin: Association<Id, One<GreenElement>>,
    ) -> (Self::Merged, Association<Id, Self::Merged>) {
        let left_child = IntoIterator::into_iter(left_origin.inner)
            .map(|(k, v)| (k, v.into_forest(0)))
            .collect::<im::HashMap<Id, ForestPath>>();
        let right_child = IntoIterator::into_iter(right_origin.inner)
            .map(|(k, v)| (k, v.into_forest(1)))
            .collect::<im::HashMap<Id, ForestPath>>();
        let resultant_origin = left_child.union_with_key(right_child, |key, _, _| {
            panic!("Id duplicate '{key:?}' found in both children; Supply invariant broken!")
        });

        let merged = Many(im::vector![left.0, right.0]);
        (
            merged,
            Association {
                inner: resultant_origin,
            },
        )
    }
}
impl Merge<Zero<GreenElement>> for Many<GreenElement> {
    type Merged = Many<GreenElement>;
    fn merge<Id: Debug + Clone + Eq + Hash>(
        left: Self,
        left_origin: Association<Id, Self>,
        _: Zero<GreenElement>,
        _: Association<Id, Zero<GreenElement>>,
    ) -> (Self::Merged, Association<Id, Self::Merged>) {
        (left, left_origin)
    }
}
impl Merge<Many<GreenElement>> for Zero<GreenElement> {
    type Merged = Many<GreenElement>;
    fn merge<Id: Debug + Clone + Eq + Hash>(
        _: Self,
        _: Association<Id, Self>,
        right: Many<GreenElement>,
        right_origin: Association<Id, Many<GreenElement>>,
    ) -> (Self::Merged, Association<Id, Self::Merged>) {
        (right, right_origin)
    }
}
impl Merge<One<GreenElement>> for Many<GreenElement> {
    type Merged = Many<GreenElement>;
    fn merge<Id: Debug + Clone + Eq + Hash>(
        mut left: Self,
        left_origin: Association<Id, Self>,
        right: One<GreenElement>,
        right_origin: Association<Id, One<GreenElement>>,
    ) -> (Self::Merged, Association<Id, Self::Merged>) {
        let right_child = IntoIterator::into_iter(right_origin.inner)
            .map(|(k, v)| (k, v.into_forest(left.0.len())))
            .collect::<im::HashMap<Id, ForestPath>>();
        let resultant_origin = left_origin.inner.union_with_key(right_child, |key, _, _| {
            panic!("Id duplicate '{key:?}' found in both children; Supply invariant broken!")
        });

        left.0.push_back(right.0);
        (
            left,
            Association {
                inner: resultant_origin,
            },
        )
    }
}
impl Merge<Many<GreenElement>> for One<GreenElement> {
    type Merged = Many<GreenElement>;
    fn merge<Id: Debug + Clone + Eq + Hash>(
        left: Self,
        left_origin: Association<Id, Self>,
        mut right: Many<GreenElement>,
        mut right_origin: Association<Id, Many<GreenElement>>,
    ) -> (Self::Merged, Association<Id, Self::Merged>) {
        right_origin
            .inner
            .iter_mut()
            .map(|(_, v)| {
                v.prepended_mut(1);
            })
            .for_each(drop);
        let left_child = IntoIterator::into_iter(left_origin.inner)
            .map(|(k, v)| (k, v.into_forest(0)))
            .collect::<im::HashMap<Id, ForestPath>>();
        let resultant_origin = right_origin.inner.union_with_key(left_child, |key, _, _| {
            panic!("Id duplicate '{key:?}' found in both children; Supply invariant broken!")
        });

        right.0.push_front(left.0);
        (
            right,
            Association {
                inner: resultant_origin,
            },
        )
    }
}
impl Merge<Many<GreenElement>> for Many<GreenElement> {
    type Merged = Many<GreenElement>;
    fn merge<Id: Debug + Clone + Eq + Hash>(
        mut left: Self,
        left_origin: Association<Id, Self>,
        right: Many<GreenElement>,
        mut right_origin: Association<Id, Many<GreenElement>>,
    ) -> (Self::Merged, Association<Id, Self::Merged>) {
        right_origin
            .inner
            .iter_mut()
            .map(|(_, v)| {
                v.prepended_mut(left.0.len());
            })
            .for_each(drop);
        let resultant_origin = left_origin
            .inner
            .union_with_key(right_origin.inner, |key, _, _| {
                panic!("Id duplicate '{key:?}' found in both children; Supply invariant broken!")
            });

        left.0.append(right.0);
        (
            left,
            Association {
                inner: resultant_origin,
            },
        )
    }
}
