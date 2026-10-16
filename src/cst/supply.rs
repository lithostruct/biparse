use std::{fmt::Debug, hash::Hash, marker::PhantomData};

/// A protocol for supplying unique `Id`'s to a parser combinator.
/// 
/// # Implementor Invariants
/// - Production of `Id` should be infallible barring exceptional circumstances (eg. out-of-memory).
/// - All `Id` values produced *must* be distinct.
/// - Clones of a [`IdSupply`] implementor must produce the same values as the original supply.
/// 
/// Failure to adhere to the above may cause the `biparse` parser ecosystem to panic.
pub trait IdSupply: Clone + Default {
    /// The type of the Id this object produces.
    type Id: Debug + Clone + Eq + Hash;

    /// Produce the next [`Self::Id`] value.
    /// This value must be distinct from all prior values produced via this function.
    fn next_id(&mut self) -> Self::Id;
}

/// A [`IdSupply`] implementor that wraps a newtype around a monotonically-increasing [`usize`].
///
/// This supply assumes that `T::from(value : usize)` is injective, which is the case for transparent newtypes.
/// Violating this invariant may lead to undefined behaviour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewtypeSupply<T>(usize, PhantomData<T>);
impl<T: From<usize>> Default for NewtypeSupply<T> {
    fn default() -> Self {
        NewtypeSupply(0, PhantomData)
    }
}
impl<T: From<usize>> Iterator for NewtypeSupply<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        let output = self.0;
        self.0 += 1;
        Some(T::from(output))
    }
}
impl<T: From<usize> + Debug + Clone + Eq + Hash> IdSupply for NewtypeSupply<T> {
    type Id = T;
    fn next_id(&mut self) -> Self::Id {
        let output = self.0;
        self.0 += 1;
        T::from(output)
    }
}

impl IdSupply for usize {
    type Id = usize;
    fn next_id(&mut self) -> Self::Id {
        let output = *self;
        *self += 1;
        output
    }
}
