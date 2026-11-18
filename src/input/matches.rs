use derive_more::{AsMut, AsRef};
use rowan::SyntaxKind;

use crate::{
    input::{Input, Pattern, PatternMatchResult},
    parse::ParseHint,
};

/// Token matching for any `Input` type.
///
/// While it is preferred to use `Input`-specific `Pattern` implementors,
/// this provides a way to work with any `Input` implementor.
/// This may be helpful for downstream implementations.
///
/// `K` should be `Into<rowan::SyntaxKind>`, and
/// `P` should be a closure or function implementing `Fn(&I::Token<'src>) -> bool`
/// for some [`Input`] implementor `I` over some lifetime `'src`.
///
/// # Note on Type Inference
///
/// Due to type inference limitations, an annotation on the closure argument is typically required,
/// especially if the argument is unused.
///
/// A common usage pattern where this limitation arises is in the predicate function that always returns true or false.
///
/// ```
/// use biparse::{input::Matches, prelude::*};
///
/// // Matches any character in a `str` input.
/// let parser : Box<dyn Parser<'_, str, char, environment::Rich>> =
///     just(Matches(SyntaxKind(1), |_ : &char| true)).boxed();
///                                   /* ^ type annotation required here! */
/// ```
///
/// Use [`Matches::always`] where applicable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, AsRef, AsMut)]
pub struct Matches<K, P>(
    /// The syntax kind corresponding to a successful match. Fixed.
    pub K,
    /// A token predicate. Meaningful for `P : Fn(&I::Token<'src>) -> bool` for some `I : Input` only.
    #[as_ref]
    #[as_mut]
    pub P,
);
impl<'src, T> Matches<SyntaxKind, fn(&T) -> bool> {
    /// Constructs a predicate that unconditionally accepts any token in some input `I`.
    /// For unconditional rejection, see [`Matches::never`].
    ///
    /// # Note on Type Inference
    ///
    /// The type of `I` *must* be supplied at the callsite.
    /// Type inference fails otherwise when used directly in `just` combinators.
    ///
    /// ```
    /// # use biparse::input::Matches;
    /// use rowan::SyntaxKind;
    /// // Unconditionally accepts any `char` token from a string input.
    /// let _ = Matches::always::<str>(SyntaxKind(0));
    /// ```
    pub fn always<I>(kind: impl Into<SyntaxKind>) -> Self
    where
        I: Input<Token<'src> = T> + ?Sized + 'src,
    {
        Matches(kind.into(), |_| true)
    }
    /// Constructs a predicate that unconditionally *rejects* any token in some input `I`.
    /// For unconditional acceptance, see [`Matches::always`].
    ///
    /// # Note on Type Inference
    ///
    /// The type of `I` *must* be supplied at the callsite.
    /// Type inference fails otherwise when used directly in `just` combinators.
    ///
    /// ```
    /// # use biparse::input::Matches;
    /// use rowan::SyntaxKind;
    /// // Unconditionally accepts any `char` token from a string input.
    /// let _ = Matches::always::<str>(SyntaxKind(0));
    /// ```
    pub fn never<I>(kind: impl Into<SyntaxKind>) -> Self
    where
        I: Input<Token<'src> = T> + ?Sized + 'src,
    {
        Matches(kind.into(), |_| false)
    }
}

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

    fn hint(&self) -> ParseHint {
        ParseHint::Opaque
    }
}
