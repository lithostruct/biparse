use std::marker::PhantomData;

use derive_more::{From, Into};
use rowan::{NodeOrToken, GreenNode, GreenToken};

type GreenItem = NodeOrToken<GreenNode, GreenToken>;

pub trait Merge<Other> {
    type Merged;
    fn merge(self, other: Other) -> Self::Merged;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct None<T>(PhantomData<T>);
impl<T> Into<Vec<T>> for None<T> {
    fn into(self) -> Vec<T> {
        Vec::new()
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, From)]
pub struct One<T>(pub T);
impl<T> Into<Vec<T>> for One<T> {
    fn into(self) -> Vec<T> {
        vec![self.0]
    }
}

impl<T> Merge<None<T>> for None<T> {
    type Merged = None<T>;
    fn merge(self, _: None<T>) -> Self::Merged {
        self
    }
}
impl<T> Merge<One<T>> for None<T> {
    type Merged = One<T>;
    fn merge(self, other: One<T>) -> Self::Merged {
        other
    }
}
impl<T> Merge<None<T>> for One<T> {
    type Merged = One<T>;
    fn merge(self, _: None<T>) -> Self::Merged {
        self
    }
}

impl<T> Merge<None<T>> for Vec<T> {
    type Merged = Vec<T>;
    fn merge(self, _: None<T>) -> Self::Merged {
        self
    }
}
impl<T> Merge<Vec<T>> for None<T> {
    type Merged = Vec<T>;
    fn merge(self, other: Vec<T>) -> Self::Merged {
        other
    }
}
impl<T> Merge<One<T>> for Vec<T> {
    type Merged = Vec<T>;
    fn merge(mut self, other: One<T>) -> Self::Merged {
        self.push(other.0);
        self
    }
}
impl<T> Merge<Vec<T>> for One<T> {
    type Merged = Vec<T>;
    fn merge(self, mut other: Vec<T>) -> Self::Merged {
        other.insert(0, self.0);
        other
    }
}