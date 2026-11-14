use std::{borrow::Cow, hash::Hash, sync::Arc};

use thiserror::Error;

use crate::{
    failure::{Descriptor, FailureKind, ParseFailure, UnboundedRecursion},
    input::{Input, LineColumn},
    parse::ParseHint,
};

/// Enum used by the [`Rich`] failure type, representing the failure kinds of parsing.
///
/// Variants and fields closely resemble that of [`FailureKind`].
#[derive(Debug, Clone, Error)]
pub enum RichKind {
    /// Expected a certain pattern, failed to match.
    /// This may correspond to a cursor location in which the input diverges.
    #[error("{}{}",
        {
            let mut tree = hint.clone().canonize().to_expectation();
            if tree.ends_with('\n') {
                tree.pop();
            }
            tree
        },
        if diverges_at.len() == 1 {
            format!("\nfound: {} at {}", diverges_at[0].1, diverges_at[0].0)
        } else { String::new() }
    )]
    ExpectedFound {
        hint: ParseHint,
        diverges_at: Vec<(LineColumn, Descriptor)>,
    },
    /// Unbounded recursion. Carries payload information relating to composites.
    #[error("unbounded recursion{}", if info.len() == 1 {
        info[1].1.name.as_deref().map_or_else(String::new, |inner| format!(": {inner}"))
    } else { String::from("") })]
    UnboundedRecursion {
        info: Vec<(LineColumn, UnboundedRecursion)>,
    },
    /// Failure caused by the [`Fail`](crate::combinator::Fail) primitive combinator.
    #[error("primitive failure{}", message.as_ref().map_or_else(String::new, |inner| format!(": {inner}")))]
    FailPrimitive { message: Option<Cow<'static, str>> },

    /// Undefined parser. This is always fatal.
    #[error("undefined parser{}", name.as_ref().map_or_else(String::new, |inner| format!(": {inner}")))]
    UndefinedParser { name: Option<Cow<'static, str>> },

    /// Dropped parser. This is always fatal.
    #[error("dropped parser{}", name.as_ref().map_or_else(String::new, |inner| format!(": {inner}")))]
    DroppedParser { name: Option<Cow<'static, str>> },

    /// Non-growing repetition in [`Many`](crate::combinator::Many). This is always fatal.
    #[error("primitive failure{}", {
        let mut tree = inner_hint.to_expectation();
        if tree.ends_with('\n') {
            tree.pop();
        }
        format!(": {}", tree)
    })]
    NonGrowingRepetition { inner_hint: ParseHint },

    /// Composite failure, which cannot be further reduced.
    ///
    /// This variant is meaningful where the enclosing [`Rich`] value has a non-empty `supplemental` field.
    #[error("composite failure")]
    CompositeFailure,

    /// Custom failure from an [`std::error::Error`] implementor, as its textual representation.
    /// 
    /// Note that for [`PartialEq`], [`Eq`] and [`Hash`](std::hash::Hash), the payload is compared or hashed by *pointer* equality.
    /// This means that custom failures constructed at different sites will not be considered equal,
    /// even if they contain identical payloads.
    /// However, values from [`Clone`] always will compare as equal.
    #[error(transparent)]
    Custom(Arc<dyn std::error::Error>),
}
impl PartialEq for RichKind {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Custom(l0), Self::Custom(r0)) => {
                // Non-default implementation: dyn objects compared by pointer identity.
                // This satisfies symmetry, transitivity, consistency and reflexivity.
                //
                // Thin pointer comparison is assumed sufficient under `Arc`,
                // barring unsafe shenanigans.
                std::ptr::addr_eq(Arc::as_ptr(l0), Arc::as_ptr(r0))
            },

            // Default implementation...

            (Self::ExpectedFound { hint: l_hint, diverges_at: l_diverges_at }, Self::ExpectedFound { hint: r_hint, diverges_at: r_diverges_at }) => l_hint == r_hint && l_diverges_at == r_diverges_at,
            (Self::UnboundedRecursion { info: l_info }, Self::UnboundedRecursion { info: r_info }) => l_info == r_info,
            (Self::FailPrimitive { message: l_message }, Self::FailPrimitive { message: r_message }) => l_message == r_message,
            (Self::UndefinedParser { name: l_name }, Self::UndefinedParser { name: r_name }) => l_name == r_name,
            (Self::DroppedParser { name: l_name }, Self::DroppedParser { name: r_name }) => l_name == r_name,
            (Self::NonGrowingRepetition { inner_hint: l_inner_hint }, Self::NonGrowingRepetition { inner_hint: r_inner_hint }) => l_inner_hint == r_inner_hint,
            
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}
impl Eq for RichKind {}
impl Hash for RichKind {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Default implementation.
        core::mem::discriminant(self).hash(state);

        match self {
            // Non-default implementation: dyn objects hashed by pointer identity.
            // This is consistent with `PartialEq` and `Eq`.
            RichKind::Custom(error) => {
                Arc::as_ptr(error).hash(state);
            },

            // Default implementation...
            RichKind::ExpectedFound { hint, diverges_at } => {
                hint.hash(state);
                diverges_at.hash(state);
            },
            RichKind::UnboundedRecursion { info } => {
                info.hash(state);
            },
            RichKind::FailPrimitive { message } => {
                message.hash(state);
            },
            RichKind::UndefinedParser { name }
            | RichKind::DroppedParser { name } => {
                name.hash(state);
            },
            RichKind::NonGrowingRepetition { inner_hint } => {
                inner_hint.hash(state);
            },
            RichKind::CompositeFailure => (),
        }
    }
}

/// Failure type that preserves most information supplied at the site of failure.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[error("{position} {}{}",
    {
        let prefix_length = position.to_string().len();
        let prefix = " ".repeat(prefix_length - 1) + "| ";
        indent::indent_with(prefix, kind.to_string())
    },
    if !self.supplemental.is_empty() {
        format!("\nsupplemental diagnostics:{}",
            self.supplemental.iter()
                .map(|inner| {
                    let mut inner_str = inner.to_string();
                    inner_str.insert(0, '\n');
                    indent::indent_all_by(4, inner_str)
                })
                .collect::<String>()
        )
    } else { String::new() }
)]
pub struct Rich {
    pub position: LineColumn,
    pub kind: RichKind,
    pub supplemental: Vec<Self>,
}
impl ParseFailure for Rich {
    fn failure_at<I: Input + ?Sized>(input: &I, cursor: I::Cursor, kind: FailureKind<I>) -> Self {
        let kind = match kind {
            FailureKind::ExpectedFound {
                hint,
                difference_at,
            } => RichKind::ExpectedFound {
                hint,
                diverges_at: difference_at.map_or(Vec::new(), |diff_cursor| {
                    let diff_position = input.to_line_col(diff_cursor);
                    let descriptor = input
                        .peek(diff_cursor)
                        .map_or(Descriptor::EndOfInput, |item| input.describe(&item.token));
                    vec![(diff_position, descriptor)]
                }),
            },
            FailureKind::FailPrimitive { message } => RichKind::FailPrimitive { message },
            FailureKind::UndefinedParser { name } => RichKind::UndefinedParser { name },
            FailureKind::DroppedParser { name } => RichKind::DroppedParser { name },
            FailureKind::NonGrowingRepetition { inner_hint } => {
                RichKind::NonGrowingRepetition { inner_hint }
            }
        };
        Self {
            kind,
            position: input.to_line_col(cursor),
            supplemental: Vec::new(),
        }
    }

    fn custom_failure_at<I: Input + ?Sized, E: std::error::Error + 'static>(
        input: &I,
        cursor: I::Cursor,
        payload: E,
    ) -> Self {
        Self {
            position: input.to_line_col(cursor),
            kind: RichKind::Custom(Arc::new(payload)),
            supplemental: Vec::new(),
        }
    }

    fn composite<I: Input + ?Sized>(
        input: &I,
        cursor: I::Cursor,
        branches: impl IntoIterator<Item = Self>,
    ) -> Self {
        let position = input.to_line_col(cursor);
        let supplemental: Vec<Self> = branches.into_iter().collect();

        // Unbounded recursion failure check.
        let try_fold = supplemental.iter().try_fold(Vec::new(), |mut acc, inner| {
            // Invariant: unbounded recursion composites are labelled as unbounded recursion.
            // This does not inspect deeply-nested structures and relies on
            match &inner.kind {
                RichKind::UnboundedRecursion { info } => {
                    acc.extend_from_slice(info);
                    Ok(acc)
                }
                _ => Err(()),
            }
        });
        if let Ok(info) = try_fold {
            return Rich {
                position,
                kind: RichKind::UnboundedRecursion { info },
                supplemental,
            };
        }

        // Attempt unification of the common expected-found speculative error.
        let all_expected_found = supplemental.clone().into_iter().try_fold(
            (Vec::new(), Vec::new(), Vec::new()),
            |(mut hints, mut divergence, mut supplemental), mut inner| match inner.kind {
                RichKind::ExpectedFound {
                    hint,
                    mut diverges_at,
                } if inner.position == position => {
                    hints.push(hint);
                    divergence.append(&mut diverges_at);
                    supplemental.append(&mut inner.supplemental);
                    Ok((hints, divergence, supplemental))
                }
                _ => Err(()),
            },
        );
        if let Ok((hints, diverges_at, supplemental)) = all_expected_found {
            return Self {
                position,
                kind: RichKind::ExpectedFound {
                    hint: ParseHint::choice(hints),
                    diverges_at,
                },
                supplemental,
            };
        }

        // Composite failure flat-mapping.
        let supplemental = supplemental
            .into_iter()
            .flat_map(|elem| {
                if elem.kind == RichKind::CompositeFailure {
                    // Position information of inner composite lost.
                    // I consider this acceptable.
                    elem.supplemental
                } else {
                    vec![elem]
                }
            })
            .collect();

        Self {
            kind: RichKind::CompositeFailure,
            position,
            supplemental,
        }
    }
    fn supplement(mut self, additional: Self) -> Self {
        if self != additional {
            self.supplemental.push(additional);
        } // Redundant information identical to principal failure is discarded.
        self
    }

    fn unbounded_recursion<I: Input + ?Sized>(
        input: &I,
        cursor: I::Cursor,
        info: UnboundedRecursion,
    ) -> Self {
        let line_col = input.to_line_col(cursor);
        Rich {
            position: line_col,
            kind: RichKind::UnboundedRecursion {
                info: vec![(line_col, info)],
            },
            supplemental: Vec::new(),
        }
    }
    fn is_unbounded_recursion_at<I: Input + ?Sized>(
        &self,
        input: &I,
        cursor: I::Cursor,
        info: UnboundedRecursion,
    ) -> bool {
        if let RichKind::UnboundedRecursion { info: existing } = &self.kind {
            existing.contains(&(input.to_line_col(cursor), info))
        } else {
            false
        }
    }
}
