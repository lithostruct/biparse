use derive_more::{AsMut, AsRef, Deref, DerefMut, From};

use crate::{
    input::{Input, Pattern, PatternMatch, PatternMatchResult},
    parse::ParseHint,
};

/// Sequential matching of all inner patterns.
///
/// Supported collections include:
/// - [`Vec<T>`]
/// - [`[T; N]`](std::array)
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, From, Deref, DerefMut, AsRef, AsMut,
)]
pub struct AllOf<P>(#[from] pub P);

impl<P, I: Input + ?Sized> Pattern<I> for AllOf<Vec<P>>
where
    P: Pattern<I>,
{
    type Payload<'src>
        = Vec<P::Payload<'src>>
    where
        I: 'src;

    fn matches_at<'src>(
        &self,
        input: &'src I,
        cursor: <I as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, I> {
        let mut success = PatternMatch::new(Vec::new(), Vec::new(), cursor);
        for child in self.0.iter() {
            match child.matches_at(input, success.next_cursor) {
                Ok(mut matched) => {
                    success.payload.push(matched.payload);
                    success.cst_spans.append(&mut matched.cst_spans);
                    success.next_cursor = matched.next_cursor;
                }
                Err(maybe_cursor) => {
                    return Err(maybe_cursor);
                }
            }
        }
        Ok(success)
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
impl<P, I: Input + ?Sized, const N: usize> Pattern<I> for AllOf<[P; N]>
where
    P: Pattern<I>,
{
    type Payload<'src>
        = [P::Payload<'src>; N]
    where
        I: 'src;

    fn matches_at<'src>(
        &self,
        input: &'src I,
        cursor: <I as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, I> {
        let mut success = PatternMatch::new(Vec::new(), Vec::new(), cursor);
        for child in self.0.iter() {
            match child.matches_at(input, success.next_cursor) {
                Ok(mut matched) => {
                    success.payload.push(matched.payload);
                    success.cst_spans.append(&mut matched.cst_spans);
                    success.next_cursor = matched.next_cursor;
                }
                Err(maybe_cursor) => {
                    return Err(maybe_cursor);
                }
            }
        }
        Ok(PatternMatch::new(
            success
                .payload
                .try_into()
                .unwrap_or_else(|_| unreachable!()),
            success.cst_spans,
            success.next_cursor,
        ))
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
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, From, Deref, DerefMut, AsRef, AsMut,
)]
pub struct AnyOf<P>(#[from] pub P);

impl<P, I: Input + ?Sized> Pattern<I> for AnyOf<Vec<P>>
where
    P: Pattern<I>,
{
    type Payload<'src>
        = P::Payload<'src>
    where
        I: 'src;

    fn matches_at<'src>(
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
impl<P, I: Input + ?Sized, const N: usize> Pattern<I> for AnyOf<[P; N]>
where
    P: Pattern<I>,
{
    type Payload<'src>
        = P::Payload<'src>
    where
        I: 'src;

    fn matches_at<'src>(
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