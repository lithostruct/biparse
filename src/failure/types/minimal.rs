use thiserror::Error;

use crate::{
    failure::{FailureKind, ParseFailure, UnboundedRecursion},
    input::Input,
};

/// Minimal failure type. Other than [`UnboundedRecursion`] information, all failure information is discarded.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[error("{}", if .0.is_empty() { String::from("parse failure") } else {
    String::from("unbounded recursion")
})]
pub struct Minimal(Vec<(usize, UnboundedRecursion)>);
impl ParseFailure for Minimal {
    fn failure_at<I: Input + ?Sized>(_: &I, _: I::Cursor, _: FailureKind<I>) -> Self {
        Minimal(Vec::new())
    }
    fn custom_failure_at<I: Input + ?Sized, E: std::error::Error>(
        _: &I,
        _: I::Cursor,
        _: E,
    ) -> Self {
        Minimal(Vec::new())
    }
    fn composite<I: Input + ?Sized>(
        _: &I,
        _: I::Cursor,
        branches: impl IntoIterator<Item = Self>,
    ) -> Self {
        let mut recursive_info: Vec<(usize, UnboundedRecursion)> = Vec::new();
        for branch in branches {
            if branch.0.is_empty() {
                return Minimal(Vec::new());
            } else {
                recursive_info.extend(branch.0);
            }
        }
        Minimal(recursive_info)
    }
    fn supplement(self, _: Self) -> Self {
        self
    }

    fn unbounded_recursion<I: Input + ?Sized>(
        input: &I,
        cursor: I::Cursor,
        info: UnboundedRecursion,
    ) -> Self {
        Minimal(vec![(input.byte_offset(cursor), info)])
    }
    fn is_unbounded_recursion_at<I: Input + ?Sized>(
        &self,
        input: &I,
        cursor: I::Cursor,
        info: UnboundedRecursion,
    ) -> bool {
        let offset = input.byte_offset(cursor);
        self.0.contains(&(offset, info))
    }
}
