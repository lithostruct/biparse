use derive_more::{AsRef, Deref};

/// A string structure that caches line start information of its source.
/// Line starts are defined as the character following a newline sequence of
/// either LF `\n` or CRLF `\r\n` only.
///
/// This performs the non-essential optimization for calculating line starts,
/// thus taking O(N) time on construction and O(lg N) on subsequent lookup.
///
/// # Note on `Input` Implementation
///
/// Originally, this was intended to be exposed as a public `Input` implementor,
/// but the way that the `Input<'src>` and `Cursor` traits were interacting was *really* not working well,
/// particularly with how `Pattern` methods can be used over any `'src` where `I: Input<'src>` is implemented,
/// and that the most natural way to implement for `CachedStr` is to delegate to the inner `str` representation.
/// This was running into limitations surrounding shadowed trait implementations.
/// See also Rust tracking issue #152409: https://github.com/rust-lang/rust/issues/152409
///
/// It was more trouble than it was worth, frankly.
/// [`LexerStream`](crate::input::LexerStream) still uses it internally though so I'll be keeping this type around.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deref, AsRef)]
pub(super) struct CachedStr<'src> {
    #[deref]
    #[as_ref]
    source: &'src str,
    linestarts: Vec<usize>,
}
impl<'src> CachedStr<'src> {
    /// Constructs a string with cached line start positions.
    pub fn new(source: &'src str) -> Self {
        let mut linestarts = vec![0];
        let mut current_offset = 0;

        for line in source.lines() {
            // text.lines() strips both \n and \r\n, so we must manually
            // calculate the full skipped stride including the newline characters.
            current_offset += line.len();

            // Peek into the remaining string to see if it used CRLF or LF
            let remaining = &source[current_offset..];
            if remaining.starts_with("\r\n") {
                current_offset += 2;
            } else if remaining.starts_with('\n') {
                current_offset += 1;
            } else {
                // end-of-input. Do not push a linestart, break.
                break;
            }

            // push the offset as the linestart offset of the next line.
            linestarts.push(current_offset);
        }

        CachedStr { source, linestarts }
    }
}
impl<'src> From<&'src str> for CachedStr<'src> {
    fn from(value: &'src str) -> Self {
        CachedStr::new(value)
    }
}
impl<'src> From<CachedStr<'src>> for &'src str {
    fn from(value: CachedStr<'src>) -> Self {
        value.source
    }
}
