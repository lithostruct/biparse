use thiserror::Error;

use crate::{
    failure::{FailureKind, ParseFailure, UnboundedRecursion},
    input::{Input, LineColumn},
};

/// Simplistic failure kind discarding all payload information of [`FailureKind`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
pub enum SimpleKind {
    /// Expected a certain pattern, failed to match.
    /// This may correspond to a cursor location in which the input diverges.
    #[error("expected-found mismatch")]
    ExpectedFound,
    /// Failure caused by the [`Fail`](crate::combinator::Fail) primitive combinator.
    #[error("primitive failure")]
    FailPrimitive,

    /// Undefined parser. This is always fatal.
    #[error("undefined parser")]
    UndefinedParser,
    /// Dropped parser. This is always fatal.
    #[error("dropped parser")]
    DroppedParser,
    /// Non-growing repetition in [`Many`](crate::combinator::Many). This is always fatal.
    #[error("non growing repetition")]
    NonGrowingRepetition,

    /// Custom failure.
    #[error("{0}")]
    Custom(String),
    /// Unknown failure.
    ///
    /// This typically results from compositing of distinct failure kinds.
    #[error("unknown failure")]
    Unknown,

    /// Unbounded recursion.
    /// Note that provenance information is not retained,
    /// and is tracked independently in [`Simple`]
    #[error("unbounded recursion")]
    UnboundedRecursion,
}
impl<I: Input + ?Sized> From<FailureKind<I>> for SimpleKind {
    fn from(value: FailureKind<I>) -> Self {
        match value {
            FailureKind::ExpectedFound { .. } => SimpleKind::ExpectedFound,
            FailureKind::FailPrimitive { .. } => SimpleKind::FailPrimitive,
            FailureKind::UndefinedParser { .. } => SimpleKind::UndefinedParser,
            FailureKind::DroppedParser { .. } => SimpleKind::DroppedParser,
            FailureKind::NonGrowingRepetition { .. } => SimpleKind::NonGrowingRepetition,
        }
    }
}
impl From<UnboundedRecursion> for SimpleKind {
    fn from(_value: UnboundedRecursion) -> Self {
        SimpleKind::UnboundedRecursion
    }
}

/// Failure type retaining positional and simplistic kind identification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[error("{line_col} {kind}")]
pub struct Simple {
    pub kind: SimpleKind,
    pub line_col: LineColumn,
    recursive_info: Vec<(LineColumn, UnboundedRecursion)>,
}
impl ParseFailure for Simple {
    fn failure_at<I: Input + ?Sized>(input: &I, cursor: I::Cursor, kind: FailureKind<I>) -> Self {
        Simple {
            kind: kind.into(),
            line_col: input.to_line_col(cursor),
            recursive_info: Vec::new(),
        }
    }
    fn custom_failure_at<I: Input + ?Sized, E: std::error::Error>(
        input: &I,
        cursor: I::Cursor,
        error: E,
    ) -> Self {
        Simple {
            kind: SimpleKind::Custom(error.to_string()),
            line_col: input.to_line_col(cursor),
            recursive_info: Vec::new(),
        }
    }
    fn composite<I: Input + ?Sized>(
        input: &I,
        cursor: I::Cursor,
        branches: impl IntoIterator<Item = Self>,
    ) -> Self {
        let line_col = input.to_line_col(cursor);
        let branches = branches.into_iter().collect::<Vec<_>>();

        let kind = if !branches.is_empty()
            && branches
                .windows(2)
                .all(|slice| slice[0].kind == slice[1].kind)
        {
            branches[0].kind.clone()
        } else {
            SimpleKind::Unknown
        };

        // We can short-circuit if the kind is not unbounded recursion.
        // This eliminates composites that don't solely consist of unbounded recursion failures.
        if kind != SimpleKind::UnboundedRecursion {
            return Simple {
                kind,
                line_col,
                recursive_info: Vec::new(),
            };
        }

        // Composites consisting solely of unbounded recursion failures.
        let mut recursive_info: Vec<(LineColumn, UnboundedRecursion)> = Vec::new();
        for branch in branches {
            recursive_info.extend(branch.recursive_info);
        }
        Simple {
            kind,
            line_col,
            recursive_info,
        }
    }
    fn supplement(self, _: Self) -> Self {
        self
    }

    fn unbounded_recursion<I: Input + ?Sized>(
        input: &I,
        cursor: I::Cursor,
        info: UnboundedRecursion,
    ) -> Self {
        let line_col = input.to_line_col(cursor);
        Simple {
            kind: SimpleKind::UnboundedRecursion,
            recursive_info: vec![(line_col, info)],
            line_col,
        }
    }
    fn is_unbounded_recursion_at<I: Input + ?Sized>(
        &self,
        input: &I,
        cursor: I::Cursor,
        info: UnboundedRecursion,
    ) -> bool {
        let line_col = input.to_line_col(cursor);
        self.recursive_info.contains(&(line_col, info))
    }
}
