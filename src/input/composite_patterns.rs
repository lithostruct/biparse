use derive_more::{AsMut, AsRef, Deref, DerefMut, From};
use rowan::SyntaxKind;

use crate::{
    input::{Input, Pattern, PatternMatch, PatternMatchResult},
    parse::ParseHint,
};

/// Token matching for any `Input` type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, AsRef, AsMut)]
pub struct Matches<K, P>(
    K,
    #[as_ref]
    #[as_mut]
    pub P,
);

impl<'src, K, F, I: Input + ?Sized> Pattern<'src, I> for Matches<K, F>
where
    I: 'src,
    K: Clone + Into<SyntaxKind>,
    F: Fn(&I::Token<'src>) -> bool,
{
    type Payload
        = I::Token<'src>
    where
        I: 'src;

    fn matches_at(
        &self,
        input: &'src I,
        cursor: <I as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, I> {
        let Some(item) = input.peek(cursor) else {
            return Err(Some(cursor));
        };
        if (self.1)(&item.token) {
            Ok(item.into_match_info(self.0.clone().into()))
        } else {
            Err(Some(cursor))
        }
    }

    fn describe(&self) -> String {
        "predicate-satisfying token".into()
    }
    fn hint(&self) -> ParseHint {
        ParseHint::Opaque
    }
}

/// Sequential matching of all inner patterns.
///
/// Supported collections include:
/// - [`Vec<T>`]
/// - [`[T; N]`](std::array)
///
/// # Notes on [`Pattern`] Atomicity
///
/// The [`Pattern`] implementation on [`AllOf`] is non-atomic; it defers to the inner patterns for matching,
/// and chains the output of the previous element to the next.  
/// This may have unintended interactions with [`SkippingStr`](crate::input::SkippingStr).
/// Refer to [`SkippingStr`](crate::input::SkippingStr) "Notes on Trivia Matching and Pattern Atomicity" for more information.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, From, Deref, DerefMut, AsRef, AsMut,
)]
pub struct AllOf<P>(#[from] pub P);

impl<'src, P, I: Input + ?Sized> Pattern<'src, I> for AllOf<Vec<P>>
where
    I: 'src,
    P: Pattern<'src, I>,
{
    type Payload
        = Vec<P::Payload>
    where
        I: 'src;

    fn matches_at(
        &self,
        input: &'src I,
        cursor: <I as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, I> {
        let mut accumulative = PatternMatch {
            payload: Vec::new(),
            cst_spans: Vec::new(),
            next_cursor: cursor,
        };
        for child in self.0.iter() {
            match child.matches_at(input, accumulative.next_cursor) {
                Ok(mut matched) => {
                    accumulative.payload.push(matched.payload);
                    accumulative.cst_spans.append(&mut matched.cst_spans);
                    accumulative.next_cursor = matched.next_cursor;
                }
                Err(maybe_cursor) => {
                    return Err(maybe_cursor);
                }
            }
        }
        Ok(accumulative)
    }

    fn describe(&self) -> String {
        format!(
            "sequential matching of [{}]",
            self.0
                .iter()
                .map(|child| child.describe())
                .collect::<Vec<String>>()
                .join(", ")
        )
    }
    fn hint(&self) -> ParseHint {
        ParseHint::Sequence(self.0.iter().map(|child| child.hint()).collect())
    }
}
impl<'src, P, I: Input + ?Sized, const N: usize> Pattern<'src, I> for AllOf<[P; N]>
where
    I: 'src,
    P: Pattern<'src, I>,
{
    type Payload
        = [P::Payload; N]
    where
        I: 'src;

    fn matches_at(
        &self,
        input: &'src I,
        cursor: <I as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, I> {
        let mut accumulative = PatternMatch {
            payload: Vec::new(),
            cst_spans: Vec::new(),
            next_cursor: cursor,
        };
        for child in self.0.iter() {
            match child.matches_at(input, accumulative.next_cursor) {
                Ok(mut matched) => {
                    accumulative.payload.push(matched.payload);
                    accumulative.cst_spans.append(&mut matched.cst_spans);
                    accumulative.next_cursor = matched.next_cursor;
                }
                Err(maybe_cursor) => {
                    return Err(maybe_cursor);
                }
            }
        }
        Ok(PatternMatch {
            payload: TryInto::<[P::Payload; N]>::try_into(accumulative.payload)
                .unwrap_or_else(|_| unreachable!()),
            cst_spans: accumulative.cst_spans,
            next_cursor: accumulative.next_cursor,
        })
    }

    fn describe(&self) -> String {
        format!(
            "all of [{}]",
            self.0
                .iter()
                .map(|child| child.describe())
                .collect::<Vec<String>>()
                .join(", ")
        )
    }
    fn hint(&self) -> ParseHint {
        ParseHint::Sequence(self.0.iter().map(|child| child.hint()).collect())
    }
}

/// Short-circuiting matching of the first succeeding inner patterns.
///
/// Supported collections include:
/// - [`Vec<T>`]
/// - [`[T; N]`](std::array)
///
/// # Notes on [`Pattern`] Atomicity
///
/// The [`Pattern`] implementation on [`AllOf`] is non-atomic; it defers to the inner patterns for matching,
/// and chains the output of the previous element to the next.  
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, From, Deref, DerefMut, AsRef, AsMut,
)]
pub struct AnyOf<P>(#[from] pub P);

impl<'src, P, I: Input + ?Sized> Pattern<'src, I> for AnyOf<Vec<P>>
where
    I: 'src,
    P: Pattern<'src, I>,
{
    type Payload
        = P::Payload
    where
        I: 'src;

    fn matches_at(
        &self,
        input: &'src I,
        cursor: <I as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, I> {
        for child in self.0.iter() {
            match child.matches_at(input, cursor) {
                Ok(matched) => return Ok(matched),
                Err(_) => {
                    continue;
                }
            }
        }
        // All alternatives failed.
        Err(None)
    }

    fn describe(&self) -> String {
        format!(
            "any of [{}]",
            self.0
                .iter()
                .map(|child| child.describe())
                .collect::<Vec<String>>()
                .join(", ")
        )
    }
    fn hint(&self) -> ParseHint {
        ParseHint::Choice(self.0.iter().map(|child| child.hint()).collect())
    }
}
impl<'src, P, I: Input + ?Sized, const N: usize> Pattern<'src, I> for AnyOf<[P; N]>
where
    I: 'src,
    P: Pattern<'src, I>,
{
    type Payload
        = P::Payload
    where
        I: 'src;

    fn matches_at(
        &self,
        input: &'src I,
        cursor: <I as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, I> {
        for child in self.0.iter() {
            match child.matches_at(input, cursor) {
                Ok(matched) => return Ok(matched),
                Err(_) => {
                    continue;
                }
            }
        }
        // All alternatives failed.
        Err(None)
    }

    fn describe(&self) -> String {
        format!(
            "any of [{}]",
            self.0
                .iter()
                .map(|child| child.describe())
                .collect::<Vec<String>>()
                .join(", ")
        )
    }
    fn hint(&self) -> ParseHint {
        ParseHint::Choice(self.0.iter().map(|child| child.hint()).collect())
    }
}
